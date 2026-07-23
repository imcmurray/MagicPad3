//! Install udev rules, optional systemd unit, and packaging helpers.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::AppResult;
use crate::models::{DriverActionResult, DriverState, DriverStatus};

const UDEV_RULE_NAME: &str = "99-magic-trackpad.rules";
const UDEV_DEST: &str = "/etc/udev/rules.d/99-magic-trackpad.rules";

pub fn status() -> AppResult<DriverStatus> {
    let udev_installed = Path::new(UDEV_DEST).exists();
    let remapper = which("input-remapper-control") || which("input-remapper-gtk");
    let kernel_mod = module_loaded("hid_magicmouse") || module_loaded("hid_multitouch");

    let mut detail = Vec::new();
    detail.push(format!(
        "udev rule: {}",
        if udev_installed {
            "installed"
        } else {
            "not installed"
        }
    ));
    detail.push(format!(
        "input-remapper: {}",
        if remapper { "present" } else { "not found" }
    ));
    detail.push(format!(
        "hid modules: {}",
        if kernel_mod {
            "magicmouse/multitouch loaded"
        } else {
            "check lsmod"
        }
    ));

    let state = if udev_installed {
        DriverState::Installed
    } else {
        DriverState::NotInstalled
    };

    Ok(DriverStatus {
        state,
        name: "MagicPad Linux helpers (udev + remapper)".into(),
        version: Some("0.3.1".into()),
        detail: detail.join(" · "),
        can_install: true,
        can_uninstall: udev_installed,
        recommended_source: Some("packaging/linux".into()),
        conflicts: vec![],
    })
}

pub fn install() -> AppResult<DriverActionResult> {
    let mut log_lines = Vec::new();
    let rule_src = locate_udev_rule();

    let Some(src) = rule_src else {
        log_lines.push("Could not find 99-magic-trackpad.rules in package resources.".into());
        return Ok(DriverActionResult {
            success: false,
            message: "udev rule source missing from bundle.".into(),
            log_lines,
            needs_reboot: false,
        });
    };

    log_lines.push(format!("Source: {}", src.display()));

    // Prefer pkexec for graphical elevation on EndeavourOS / Arch
    let install_ok = if which("pkexec") {
        log_lines.push("Installing via pkexec…".into());
        match Command::new("pkexec")
            .args([
                "bash",
                "-c",
                &format!(
                    "install -m 644 '{}' '{}' && udevadm control --reload-rules && udevadm trigger",
                    src.display(),
                    UDEV_DEST
                ),
            ])
            .status()
        {
            Ok(s) => s.success(),
            Err(e) => {
                log_lines.push(format!("pkexec failed: {e}"));
                false
            }
        }
    } else if nix_is_root() {
        log_lines.push("Installing as root…".into());
        if let Err(e) = std::fs::copy(&src, UDEV_DEST) {
            log_lines.push(format!("copy failed: {e}"));
            false
        } else {
            let _ = Command::new("udevadm")
                .args(["control", "--reload-rules"])
                .status();
            let _ = Command::new("udevadm").args(["trigger"]).status();
            true
        }
    } else {
        // Write user-local copy + instructions
        if let Some(home) = dirs::home_dir() {
            let dest = home
                .join(".config/magicpad-companion")
                .join(UDEV_RULE_NAME);
            if let Some(p) = dest.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::copy(&src, &dest)?;
            log_lines.push(format!(
                "Copied rule to {} (needs root to install system-wide).",
                dest.display()
            ));
            log_lines.push(format!(
                "Run: sudo install -m 644 {} {} && sudo udevadm control --reload-rules && sudo udevadm trigger",
                dest.display(),
                UDEV_DEST
            ));
        }
        return Ok(DriverActionResult {
            success: false,
            message: "Root privileges required for udev install. Rule staged for manual sudo."
                .into(),
            log_lines,
            needs_reboot: false,
        });
    };

    if install_ok {
        stage_extra_helpers(&mut log_lines);
        Ok(DriverActionResult {
            success: true,
            message: "Linux helpers installed. Replug the trackpad to apply udev rules.".into(),
            log_lines,
            needs_reboot: false,
        })
    } else {
        Ok(DriverActionResult {
            success: false,
            message: "Helper install did not complete successfully.".into(),
            log_lines,
            needs_reboot: false,
        })
    }
}

pub fn uninstall() -> AppResult<DriverActionResult> {
    let mut log_lines = Vec::new();
    if !Path::new(UDEV_DEST).exists() {
        return Ok(DriverActionResult {
            success: true,
            message: "No system udev rule installed.".into(),
            log_lines,
            needs_reboot: false,
        });
    }

    let ok = if which("pkexec") {
        Command::new("pkexec")
            .args([
                "bash",
                "-c",
                &format!(
                    "rm -f '{}' && udevadm control --reload-rules && udevadm trigger",
                    UDEV_DEST
                ),
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else if nix_is_root() {
        std::fs::remove_file(UDEV_DEST).is_ok()
    } else {
        log_lines.push(format!("sudo rm {UDEV_DEST}"));
        return Ok(DriverActionResult {
            success: false,
            message: "Root required to remove system udev rule.".into(),
            log_lines,
            needs_reboot: false,
        });
    };

    Ok(DriverActionResult {
        success: ok,
        message: if ok {
            "udev rule removed.".into()
        } else {
            "Failed to remove udev rule.".into()
        },
        log_lines,
        needs_reboot: false,
    })
}

fn stage_extra_helpers(log: &mut Vec<String>) {
    if let Some(cfg) = dirs::config_dir() {
        let unit_src = locate_resource("magicpad-companion.service");
        if let Some(src) = unit_src {
            let dest = cfg
                .join("systemd")
                .join("user")
                .join("magicpad-companion.service");
            if let Some(p) = dest.parent() {
                let _ = std::fs::create_dir_all(p);
            }
            if std::fs::copy(&src, &dest).is_ok() {
                log.push(format!("Staged user unit: {}", dest.display()));
                log.push("Enable with: systemctl --user enable --now magicpad-companion.service".into());
            }
        }
    }
}

fn locate_udev_rule() -> Option<PathBuf> {
    locate_resource(UDEV_RULE_NAME)
}

fn locate_resource(name: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("packaging/linux").join(name));
            candidates.push(dir.join("../packaging/linux").join(name));
            candidates.push(dir.join(name));
        }
    }
    candidates.push(PathBuf::from("packaging/linux").join(name));
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../packaging/linux")
            .join(name),
    );
    candidates.into_iter().find(|p| p.is_file())
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn module_loaded(name: &str) -> bool {
    std::fs::read_to_string("/proc/modules")
        .map(|s| s.lines().any(|l| l.starts_with(name)))
        .unwrap_or(false)
}

fn nix_is_root() -> bool {
    #[cfg(target_os = "linux")]
    {
        nix::unistd::Uid::effective().is_root()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}
