// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        if arg == "--gestures" || arg == "gestures" {
            #[cfg(target_os = "linux")]
            {
                let _ = env_logger::Builder::from_env(
                    env_logger::Env::default().default_filter_or("info"),
                )
                .try_init();
                let code = magicpad_companion_lib::gesture_daemon::run();
                std::process::exit(code);
            }
            #[cfg(not(target_os = "linux"))]
            {
                eprintln!("Gesture daemon is only available on Linux.");
                std::process::exit(1);
            }
        }
        if arg == "--help" || arg == "-h" {
            eprintln!(
                "MagicPad Companion\n  (no args)     Launch GUI\n  --gestures    Run multi-finger gesture daemon (Linux)"
            );
            std::process::exit(0);
        }
    }
    magicpad_companion_lib::run();
}
