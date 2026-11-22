// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // A crash leaves a record that uQ lists on Settings › Problems.
    emobie_lib::uq_diag::install_panic_hook("emobie");
    // `emobie status | config | macro …` answers on the command line and exits; anything else
    // starts the app.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = emobie_lib::cli::run(&args) {
        std::process::exit(code);
    }
    emobie_lib::run()
}
