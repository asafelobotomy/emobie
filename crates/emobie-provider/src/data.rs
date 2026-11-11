//! The shared emoji data: `/usr/share/emobie/emoji.json`, generated from emojibase by
//! `scripts/build-emoji-json.ts` (the app imports the same file).

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The format `scripts/build-emoji-json.ts` writes (`EMOJI_DATA_FORMAT`).
pub const FORMAT: u32 = 1;
pub const DEFAULT_PATH: &str = "/usr/share/emobie/emoji.json";
/// Development override: an absolute path to another `emoji.json`.
pub const PATH_ENV: &str = "EMOBIE_EMOJI_JSON";
/// The real file is about 430 KiB.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct File {
    format: u32,
    emojis: Vec<RawEmoji>,
}

#[derive(Debug, Deserialize)]
struct RawEmoji {
    emoji: String,
    hexcode: String,
    name: String,
    order: u32,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    shortcodes: Vec<String>,
    #[serde(default)]
    emoticons: Vec<String>,
    #[serde(default)]
    skins: Vec<RawSkin>,
}

#[derive(Debug, Deserialize)]
struct RawSkin {
    emoji: String,
    tone: Tone,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Tone {
    One(u8),
    /// Multi-person emoji: one tone per person; the first decides, as in the app.
    Many(Vec<u8>),
}

impl Tone {
    fn first(&self) -> u8 {
        match self {
            Tone::One(t) => *t,
            Tone::Many(ts) => ts.first().copied().unwrap_or(0),
        }
    }
}

/// One emoji with its search fields lowercased once.
#[derive(Debug)]
pub struct Entry {
    pub emoji: String,
    pub hexcode: String,
    pub name: String,
    pub order: u32,
    pub name_lc: String,
    pub keywords_lc: Vec<String>,
    pub shortcodes_lc: Vec<String>,
    pub emoticons_lc: Vec<String>,
    /// `(tone 1-5, emoji)`.
    pub skins: Vec<(u8, String)>,
}

impl Entry {
    /// This emoji in skin tone `tone` (0 = default), falling back to the default.
    pub fn with_tone(&self, tone: u8) -> &str {
        if tone == 0 {
            return &self.emoji;
        }
        self.skins
            .iter()
            .find(|(t, _)| *t == tone)
            .map(|(_, e)| e.as_str())
            .unwrap_or(&self.emoji)
    }
}

#[derive(Debug)]
pub struct Catalog {
    pub entries: Vec<Entry>,
    /// Every emoji and skin variant → its entry.
    by_char: HashMap<String, usize>,
    by_hexcode: HashMap<String, usize>,
}

impl Catalog {
    pub fn parse(text: &str) -> Result<Self, String> {
        let file: File = serde_json::from_str(text).map_err(|e| format!("bad emoji data: {e}"))?;
        if file.format != FORMAT {
            return Err(format!(
                "emoji data format {} (this emobie-provider reads {FORMAT})",
                file.format
            ));
        }
        let lc = |v: Vec<String>| v.into_iter().map(|s| s.to_lowercase()).collect::<Vec<_>>();
        let mut entries: Vec<Entry> = file
            .emojis
            .into_iter()
            .map(|e| Entry {
                name_lc: e.name.to_lowercase(),
                keywords_lc: lc(e.keywords),
                shortcodes_lc: lc(e.shortcodes),
                emoticons_lc: lc(e.emoticons),
                skins: e
                    .skins
                    .into_iter()
                    .map(|s| (s.tone.first(), s.emoji))
                    .collect(),
                emoji: e.emoji,
                hexcode: e.hexcode,
                name: e.name,
                order: e.order,
            })
            .collect();
        entries.sort_by_key(|e| e.order);
        let mut by_char = HashMap::new();
        let mut by_hexcode = HashMap::new();
        for (i, e) in entries.iter().enumerate() {
            by_char.entry(e.emoji.clone()).or_insert(i);
            for (_, s) in &e.skins {
                by_char.entry(s.clone()).or_insert(i);
            }
            by_hexcode.entry(e.hexcode.clone()).or_insert(i);
        }
        if entries.is_empty() {
            return Err("emoji data has no emoji".into());
        }
        Ok(Self {
            entries,
            by_char,
            by_hexcode,
        })
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if meta.len() > MAX_BYTES {
            return Err(format!("{}: larger than {MAX_BYTES} bytes", path.display()));
        }
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&text)
    }

    /// `$EMOBIE_EMOJI_JSON` when it is an absolute path, else the installed file.
    pub fn default_path() -> PathBuf {
        std::env::var_os(PATH_ENV)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| PathBuf::from(DEFAULT_PATH))
    }

    /// The entry for an emoji or one of its skin variants.
    pub fn by_char(&self, emoji: &str) -> Option<usize> {
        self.by_char.get(emoji).copied()
    }

    pub fn by_hexcode(&self, hexcode: &str) -> Option<usize> {
        self.by_hexcode.get(hexcode).copied()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const FIXTURE: &str = r#"{
      "format": 1, "source": "fixture", "groups": [{"id":0,"key":"smileys-emotion","name":"smileys & emotion"}],
      "emojis": [
        {"emoji":"😀","hexcode":"1F600","name":"grinning face","group":0,"order":1,"keywords":["happy","smile"],"shortcodes":["grinning"],"emoticons":[":D"]},
        {"emoji":"😄","hexcode":"1F604","name":"grinning face with smiling eyes","group":0,"order":3,"keywords":["smile","laugh"],"shortcodes":["smile"],"emoticons":[]},
        {"emoji":"🙂","hexcode":"1F642","name":"slightly smiling face","group":0,"order":10,"keywords":["face","smile"],"shortcodes":["slight_smile"],"emoticons":[":)"]},
        {"emoji":"👋","hexcode":"1F44B","name":"waving hand","group":0,"order":20,"keywords":["wave","hello"],"shortcodes":["wave"],"emoticons":[],
         "skins":[{"emoji":"👋🏻","tone":1},{"emoji":"👋🏽","tone":3}]},
        {"emoji":"🧑‍🤝‍🧑","hexcode":"1F9D1-200D-1F91D-200D-1F9D1","name":"people holding hands","group":0,"order":30,"keywords":["hand"],"shortcodes":[],"emoticons":[],
         "skins":[{"emoji":"🧑🏻‍🤝‍🧑🏼","tone":[1,2]}]},
        {"emoji":"🐱","hexcode":"1F431","name":"cat face","group":0,"order":40,"keywords":["cat","pet"],"shortcodes":["cat"],"emoticons":[]}
      ]
    }"#;

    pub(crate) fn fixture() -> Catalog {
        Catalog::parse(FIXTURE).unwrap()
    }

    #[test]
    fn parses_and_indexes() {
        let c = fixture();
        assert_eq!(c.entries.len(), 6);
        assert_eq!(c.by_char("👋🏽"), c.by_char("👋"));
        assert_eq!(c.by_hexcode("1F431"), Some(5));
        assert_eq!(c.entries[3].with_tone(3), "👋🏽");
        assert_eq!(c.entries[3].with_tone(5), "👋");
        assert_eq!(c.entries[4].with_tone(1), "🧑🏻‍🤝‍🧑🏼");
        assert_eq!(c.by_char("x"), None);
    }

    #[test]
    fn rejects_other_formats_and_garbage() {
        assert!(Catalog::parse(r#"{"format":2,"emojis":[]}"#)
            .unwrap_err()
            .contains("format 2"));
        assert!(Catalog::parse(r#"{"format":1,"emojis":[]}"#).is_err());
        assert!(Catalog::parse("not json").is_err());
    }

    #[test]
    fn the_generated_file_loads() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/data/emoji.json");
        let c = Catalog::load(&path).unwrap();
        assert!(c.entries.len() > 1000);
        let thumbs = c.by_hexcode("1F44D").unwrap();
        assert_eq!(c.entries[thumbs].emoji, "👍\u{fe0f}");
        assert_eq!(c.entries[thumbs].skins.len(), 5);
    }
}
