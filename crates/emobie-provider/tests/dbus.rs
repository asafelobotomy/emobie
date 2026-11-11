//! `org.uq.Launcher1.Provider` on a private session bus, as uQ's launcher bridge calls it:
//! Search rows and ranking, input validation on Activate, copy through a stub `wl-copy`, `type`
//! through a fake helper (and its fallback to copy), and bus activation of the real binary with
//! its idle exit. Skipped when `dbus-daemon` is not installed.

use emobie_provider::activate::{Clipboard, Paster};
use emobie_provider::data::Catalog;
use emobie_provider::prefs::PrefsSource;
use emobie_provider::{Provider, BUS_NAME, PATH};
use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;

type Row = (String, String, String, String, String, f64);

#[zbus::proxy(
    interface = "org.uq.Launcher1.Provider",
    default_service = "org.emobie.Provider",
    default_path = "/org/uq/Launcher1"
)]
trait Launcher {
    fn search(&self, query: &str) -> zbus::Result<Vec<Row>>;
    fn activate(&self, id: &str, action: &str) -> zbus::Result<()>;
}

/// A `dbus-daemon` of our own, stopped on drop.
struct PrivateBus {
    child: Child,
    address: String,
}

impl PrivateBus {
    fn start(dir: &Path, env: &[(&str, &Path)]) -> Option<Self> {
        let services = dir.join("services");
        std::fs::create_dir_all(&services).unwrap();
        let config = dir.join("bus.conf");
        std::fs::write(
            &config,
            format!(
                r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:path={}</listen>
  <servicedir>{}</servicedir>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow receive_sender="*"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#,
                dir.join("bus").display(),
                services.display()
            ),
        )
        .unwrap();
        let mut cmd = Command::new("dbus-daemon");
        cmd.arg(format!("--config-file={}", config.display()))
            .args(["--nofork", "--nopidfile", "--print-address=1"])
            .stdout(Stdio::piped());
        for (k, v) in env {
            cmd.env(k, v);
        }
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
            Err(e) => panic!("dbus-daemon: {e}"),
        };
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        assert!(line.starts_with("unix:"), "dbus-daemon printed {line:?}");
        Some(Self {
            child,
            address: line.trim().to_string(),
        })
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "emobie-provider-dbus-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn emoji_json() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/data/emoji.json")
}

/// A `wl-copy` that appends its arguments, one per line, to `log`.
fn stub_wl_copy(dir: &Path, log: &Path) -> PathBuf {
    let path = dir.join("wl-copy");
    let script = format!(
        "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\"; done >> '{}'\n",
        log.display()
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[derive(Default)]
struct FakeHelper {
    running: bool,
    pastes: AtomicUsize,
}

impl Paster for FakeHelper {
    fn available(&self) -> bool {
        self.running
    }
    fn paste(&self) -> Result<(), String> {
        self.pastes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

async fn connect(address: &str) -> zbus::Connection {
    zbus::connection::Builder::address(address)
        .build()
        .await
        .unwrap()
}

/// The D-Bus error name, which zbus puts first in the message.
fn error_name(e: zbus::Error) -> String {
    e.to_string()
        .split(':')
        .next()
        .unwrap_or_default()
        .to_string()
}

async fn serve(address: &str, dir: &Path, log: &Path, helper: Arc<FakeHelper>) -> zbus::Connection {
    std::fs::write(
        dir.join("prefs.json"),
        r#"{"preferences":{"favorites":["1F642"],"recents":["👋🏽"],"skinTone":0}}"#,
    )
    .unwrap();
    let provider = Provider {
        catalog: Arc::new(Catalog::load(&emoji_json()).unwrap()),
        prefs: Arc::new(PrefsSource::new(vec![dir.join("prefs.json")])),
        clipboard: Clipboard {
            program: stub_wl_copy(dir, log),
            timeout: Duration::from_secs(5),
        },
        paster: helper,
        paste_delay: Duration::from_millis(10),
        touched: Arc::new(Notify::new()),
    };
    zbus::connection::Builder::address(address)
        .name(BUS_NAME)
        .serve_at(PATH, provider)
        .build()
        .await
        .unwrap()
}

async fn wait_for(what: &str, mut f: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !f() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_and_activate_on_a_private_bus() {
    let dir = tempdir("iface");
    let Some(bus) = PrivateBus::start(&dir, &[]) else {
        eprintln!("dbus-daemon is not installed; skipping");
        return;
    };
    let log = dir.join("copied");
    let helper = Arc::new(FakeHelper {
        running: true,
        ..Default::default()
    });
    let _service = serve(&bus.address, &dir, &log, helper.clone()).await;
    let conn = connect(&bus.address).await;
    let launcher = LauncherProxy::new(&conn).await.unwrap();

    // Favourites and recents first; the emoji is the title and the id, the name the subtitle.
    let rows = launcher.search("smil").await.unwrap();
    assert!(!rows.is_empty() && rows.len() <= 20);
    assert_eq!(rows[0].0, "🙂");
    assert_eq!(
        (rows[0].1.as_str(), rows[0].2.as_str()),
        ("🙂", "slightly smiling face")
    );
    assert_eq!(rows[0].4, r#"["copy","type"]"#);
    assert!(rows.windows(2).all(|w| w[0].5 >= w[1].5));
    let wave = launcher.search("wave").await.unwrap();
    assert_eq!(wave[0].0, "👋🏽", "the recent variant, as copied");
    assert!(launcher.search("x\u{1}y").await.unwrap().is_empty());
    assert_eq!(launcher.search("face").await.unwrap().len(), 20);

    // Activate takes only catalog emoji and known actions.
    for (id, action) in [
        ("not-an-emoji", ""),
        ("$(id)", "copy"),
        ("🙂", "rm"),
        ("🙂", "COPY"),
        ("", ""),
    ] {
        let e = launcher.activate(id, action).await.unwrap_err();
        assert_eq!(
            error_name(e),
            "org.freedesktop.DBus.Error.InvalidArgs",
            "{id:?} {action:?}"
        );
    }
    let long = "🙂".repeat(40);
    assert!(launcher.activate(&long, "").await.is_err());
    assert!(!log.exists(), "nothing was copied for a rejected call");

    launcher.activate("🙂", "").await.unwrap();
    launcher.activate("👋🏽", "copy").await.unwrap();
    assert_eq!(std::fs::read_to_string(&log).unwrap(), "--\n🙂\n--\n👋🏽\n");
    assert_eq!(helper.pastes.load(Ordering::SeqCst), 0);

    // `type`: copy, then the helper pastes.
    launcher.activate("🐱", "type").await.unwrap();
    wait_for("the paste", || helper.pastes.load(Ordering::SeqCst) == 1).await;
    assert!(std::fs::read_to_string(&log).unwrap().ends_with("--\n🐱\n"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn type_without_the_helper_is_a_copy() {
    let dir = tempdir("nohelper");
    let Some(bus) = PrivateBus::start(&dir, &[]) else {
        eprintln!("dbus-daemon is not installed; skipping");
        return;
    };
    let log = dir.join("copied");
    let helper = Arc::new(FakeHelper::default());
    let _service = serve(&bus.address, &dir, &log, helper.clone()).await;
    let launcher = LauncherProxy::new(&connect(&bus.address).await)
        .await
        .unwrap();
    launcher.activate("🐱", "type").await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(std::fs::read_to_string(&log).unwrap(), "--\n🐱\n");
    assert_eq!(helper.pastes.load(Ordering::SeqCst), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bus_activation_and_idle_exit() {
    let dir = tempdir("activation");
    let data_home = dir.join("data");
    let emoji = emoji_json().canonicalize().unwrap();
    // The packaged service file, with the test binary in place of /usr/bin/emobie-provider.
    let packaged = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../uq/dbus/org.emobie.Provider.service"),
    )
    .unwrap();
    assert!(packaged.contains("Exec=/usr/bin/emobie-provider\n"));
    let service = packaged.replace(
        "/usr/bin/emobie-provider",
        env!("CARGO_BIN_EXE_emobie-provider"),
    );
    // Written before the bus starts, so it is in the first scan of the service directory.
    std::fs::create_dir_all(dir.join("services")).unwrap();
    std::fs::write(dir.join("services/org.emobie.Provider.service"), service).unwrap();
    let Some(bus) = PrivateBus::start(
        &dir,
        &[
            ("EMOBIE_EMOJI_JSON", emoji.as_path()),
            ("EMOBIE_PROVIDER_IDLE_SECS", Path::new("2")),
            ("XDG_DATA_HOME", data_home.as_path()),
        ],
    ) else {
        eprintln!("dbus-daemon is not installed; skipping");
        return;
    };
    std::fs::create_dir_all(data_home.join("emobie")).unwrap();
    std::fs::write(
        data_home.join("emobie/preferences.json"),
        r#"{"favorites":["1F431"]}"#,
    )
    .unwrap();

    let conn = connect(&bus.address).await;
    let dbus = zbus::fdo::DBusProxy::new(&conn).await.unwrap();
    let activatable = dbus.list_activatable_names().await.unwrap();
    assert!(activatable.iter().any(|n| n.as_str() == BUS_NAME));

    // As uQ does: start it by name, then query only the running name.
    dbus.start_service_by_name(BUS_NAME.try_into().unwrap(), 0)
        .await
        .unwrap();
    assert!(dbus
        .name_has_owner(BUS_NAME.try_into().unwrap())
        .await
        .unwrap());
    let launcher = LauncherProxy::new(&conn).await.unwrap();
    let rows = launcher.search("cat").await.unwrap();
    assert_eq!(rows[0].0, "🐱");
    assert_eq!(rows[0].2, "cat face");

    // Two idle seconds later it is gone.
    let deadline = Instant::now() + Duration::from_secs(15);
    while dbus
        .name_has_owner(BUS_NAME.try_into().unwrap())
        .await
        .unwrap()
    {
        assert!(
            Instant::now() < deadline,
            "emobie-provider did not exit when idle"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    drop(bus);
    let _ = std::fs::remove_dir_all(&dir);
}
