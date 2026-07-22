//! Precision Touchpad driver install / status for Windows.
//!
//! Preferred upstream: vitoplantamura/MagicTrackpad2ForWindows
//! (Microsoft-signed, USB-C + Bluetooth + battery).
//! Fallback lineage: imbushuo/mac-precision-touchpad.

use std::process::Command;

use crate::error::{AppError, AppResult};
use crate::models::{DriverActionResult, DriverState, DriverStatus};

pub const RECOMMENDED_DRIVER_URL: &str =
    "https://github.com/vitoplantamura/MagicTrackpad2ForWindows/releases";
pub const RECOMMENDED_DRIVER_NAME: &str = "MagicTrackpad2ForWindows (vitoplantamura)";
const IMBUSHUO_URL: &str = "https://github.com/imbushuo/mac-precision-touchpad/releases";

pub fn status() -> AppResult<DriverStatus> {
    let mut conflicts = Vec::new();
    let mut state = DriverState::NotInstalled;
    let mut version = None;
    let mut detail = String::from(
        "Looking for Apple USB Precision Touchpad / AmtPtp driver packages.",
    );

    // Detect installed packages via pnputil
    if let Ok(output) = Command::new("pnputil").args(["/enum-drivers"]).output() {
        let text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        if text.contains("amtptp")
            || text.contains("magic trackpad")
            || text.contains("apple usb precision")
            || text.contains("mac-precision")
        {
            state = DriverState::Installed;
            detail = "Precision Touchpad driver package appears installed.".into();
            // Best-effort version scrape is left for later releases.
            version = Some("detected".into());
        }
    }

    // Conflict heuristics: common commercial stacks
    if let Ok(output) = Command::new("pnputil").args(["/enum-drivers"]).output() {
        let text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        if text.contains("magic utilities") || text.contains("magicutilities") {
            conflicts.push("Magic Utilities".into());
        }
        if text.contains("trackpad++") || text.contains("trackpadplusplus") {
            conflicts.push("Trackpad++".into());
        }
    }

    if !conflicts.is_empty() {
        state = DriverState::Conflict;
        detail = format!(
            "Conflicting software detected: {}. Uninstall before installing the Precision driver.",
            conflicts.join(", ")
        );
    }

    Ok(DriverStatus {
        state,
        name: RECOMMENDED_DRIVER_NAME.into(),
        version,
        detail,
        can_install: true,
        can_uninstall: matches!(
            state,
            DriverState::Installed | DriverState::Outdated | DriverState::Conflict
        ),
        recommended_source: Some(RECOMMENDED_DRIVER_URL.into()),
        conflicts,
    })
}

pub fn install() -> AppResult<DriverActionResult> {
    let mut log_lines = Vec::new();
    log_lines.push("Windows Precision driver install requested.".into());
    log_lines.push(format!("Recommended source: {RECOMMENDED_DRIVER_URL}"));
    log_lines.push(format!("Fallback source: {IMBUSHUO_URL}"));

    // Locate bundled or previously downloaded INF tree.
    let candidates = driver_search_paths();
    let mut inf_path: Option<std::path::PathBuf> = None;
    for dir in &candidates {
        if let Ok(found) = find_inf(dir) {
            inf_path = Some(found);
            break;
        }
    }

    let Some(inf) = inf_path else {
        log_lines.push(
            "No local INF found. Download the AMD64/ARM64 package from Releases, extract it, then place under %LOCALAPPDATA%\\MagicPadCompanion\\drivers\\".into(),
        );
        log_lines.push(
            "Then re-run Install, or right-click the INF → Install (as documented by upstream).".into(),
        );
        return Ok(DriverActionResult {
            success: false,
            message: "Driver package not found locally. Open the download page, then retry.".into(),
            log_lines,
            needs_reboot: false,
        });
    };

    log_lines.push(format!("Using INF: {}", inf.display()));

    // Elevate + install via pnputil. May fail without admin — report clearly.
    let output = Command::new("pnputil")
        .args([
            "/add-driver",
            &inf.to_string_lossy(),
            "/install",
        ])
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if !stdout.is_empty() {
                log_lines.push(stdout.trim().to_string());
            }
            if !stderr.is_empty() {
                log_lines.push(stderr.trim().to_string());
            }
            let success = o.status.success();
            Ok(DriverActionResult {
                success,
                message: if success {
                    "Driver package submitted to Windows. Replug the trackpad if it does not appear."
                        .into()
                } else {
                    "pnputil failed. Run MagicPad Companion as Administrator and retry, or install the INF manually.".into()
                },
                log_lines,
                needs_reboot: false,
            })
        }
        Err(e) => Err(AppError::msg(format!("failed to run pnputil: {e}"))),
    }
}

pub fn uninstall() -> AppResult<DriverActionResult> {
    let mut log_lines = Vec::new();
    log_lines.push(
        "Automatic uninstall lists published driver packages; preferred tool is DriverStore Explorer or Device Manager.".into(),
    );

    // Best-effort: user must confirm which oemXX.inf — we guide rather than force-delete.
    let output = Command::new("pnputil").args(["/enum-drivers"]).output();
    if let Ok(o) = output {
        let text = String::from_utf8_lossy(&o.stdout);
        for line in text.lines() {
            let l = line.to_ascii_lowercase();
            if l.contains("amtptp")
                || l.contains("magic")
                || l.contains("apple") && l.contains("touchpad")
            {
                log_lines.push(line.to_string());
            }
        }
    }

    Ok(DriverActionResult {
        success: true,
        message: "Review matching packages above. Use Device Manager → uninstall 'Apple USB Precision Touchpad' or DriverStore Explorer, then reboot.".into(),
        log_lines,
        needs_reboot: true,
    })
}

fn driver_search_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Some(local) = dirs::data_local_dir() {
        paths.push(local.join("MagicPadCompanion").join("drivers"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("drivers"));
            paths.push(dir.join("packaging").join("windows").join("drivers"));
        }
    }
    // Dev-tree relative
    paths.push(std::path::PathBuf::from("packaging/windows/drivers"));
    paths
}

fn find_inf(root: &std::path::Path) -> Result<std::path::PathBuf, ()> {
    if !root.exists() {
        return Err(());
    }
    // Prefer architecture-specific folders
    let arch = std::env::consts::ARCH;
    let arch_dir = match arch {
        "x86_64" => "AMD64",
        "aarch64" => "ARM64",
        _ => "",
    };
    let mut stack = vec![root.to_path_buf()];
    if !arch_dir.is_empty() {
        let preferred = root.join(arch_dir);
        if preferred.is_dir() {
            stack.insert(0, preferred);
        }
    }
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for ent in rd.flatten() {
                let p = ent.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("inf"))
                    .unwrap_or(false)
                {
                    return Ok(p);
                }
            }
        }
    }
    Err(())
}
