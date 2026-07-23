//! Gesture map persistence + libinput daemon apply + input-remapper export.

use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::gesture_daemon;
use crate::models::{GestureAction, GestureMap, GestureTrigger};

pub fn load(config_dir: &Path) -> AppResult<GestureMap> {
    let path = config_dir.join("gestures.json");
    if !path.exists() {
        let mut map = GestureMap::default();
        map.backend = detect_backend();
        return Ok(map);
    }
    let mut map: GestureMap = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    map.backend = detect_backend();
    // Upgrade older configs that left pinch unbound
    for b in &mut map.bindings {
        match b.trigger {
            GestureTrigger::PinchIn if b.action == GestureAction::None => {
                b.action = GestureAction::ZoomOut;
            }
            GestureTrigger::PinchOut if b.action == GestureAction::None => {
                b.action = GestureAction::ZoomIn;
            }
            _ => {}
        }
    }
    Ok(map)
}

pub fn save(config_dir: &Path, gestures: &GestureMap) -> AppResult<()> {
    std::fs::create_dir_all(config_dir)?;
    let mut g = gestures.clone();
    g.backend = detect_backend();
    std::fs::write(
        config_dir.join("gestures.json"),
        serde_json::to_string_pretty(&g)?,
    )?;
    Ok(())
}

/// Persist map, export remapper profile, and start the user gesture daemon.
pub fn apply(config_dir: &Path, gestures: &GestureMap) -> AppResult<String> {
    save(config_dir, gestures)?;
    let _ = export_input_remapper(gestures);
    match gesture_daemon::ensure_daemon_running() {
        Ok(msg) => Ok(msg),
        Err(e) => {
            // Still saved — report setup needs
            Err(AppError::msg(format!(
                "Gestures saved, but daemon not started: {e}"
            )))
        }
    }
}

fn detect_backend() -> String {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if desktop.contains("budgie") || which("labwc") {
        "libinput-daemon (Budgie/labwc)".into()
    } else if which("input-remapper-control") || which("input-remapper-gtk") {
        "libinput-daemon + input-remapper".into()
    } else if desktop.contains("gnome") {
        "libinput-daemon (GNOME keys)".into()
    } else {
        "libinput-daemon".into()
    }
}

fn which(bin: &str) -> bool {
    std::process::Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Export a simplified input-remapper-compatible JSON preset under
/// `~/.config/input-remapper-2/presets/Magic Trackpad/MagicPad.json` when possible.
pub fn export_input_remapper(gestures: &GestureMap) -> AppResult<()> {
    let Some(cfg) = dirs::config_dir() else {
        return Ok(());
    };
    let dir = cfg
        .join("input-remapper-2")
        .join("presets")
        .join("Magic Trackpad");
    // Always also write under our config for packaging / manual install
    let local = cfg
        .join("magicpad-companion")
        .join("input-remapper")
        .join("MagicPad.json");

    let preset = build_remapper_preset(gestures);
    let body = serde_json::to_string_pretty(&preset)?;

    if let Some(parent) = local.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&local, &body)?;

    // Best-effort install into input-remapper tree
    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = std::fs::write(dir.join("MagicPad.json"), &body);
    }

    Ok(())
}

fn build_remapper_preset(gestures: &GestureMap) -> serde_json::Value {
    // input-remapper preset schema is complex; we emit a documented stub that
    // users (or a later version) can refine. Includes mapping table for clarity.
    let mappings: Vec<serde_json::Value> = gestures
        .bindings
        .iter()
        .filter(|b| b.action != GestureAction::None)
        .map(|b| {
            serde_json::json!({
                "trigger": trigger_name(b.trigger),
                "action": action_name(b.action),
                "custom": b.custom,
            })
        })
        .collect();

    serde_json::json!({
        "version": "0.1",
        "generator": "MagicPad Companion",
        "device": "Magic Trackpad",
        "note": "Import or merge into input-remapper. Advanced 3/4-finger maps may need compositor support.",
        "mappings": mappings,
    })
}

fn trigger_name(t: GestureTrigger) -> &'static str {
    use GestureTrigger::*;
    match t {
        ThreeFingerSwipeLeft => "3finger_swipe_left",
        ThreeFingerSwipeRight => "3finger_swipe_right",
        ThreeFingerSwipeUp => "3finger_swipe_up",
        ThreeFingerSwipeDown => "3finger_swipe_down",
        ThreeFingerTap => "3finger_tap",
        FourFingerSwipeLeft => "4finger_swipe_left",
        FourFingerSwipeRight => "4finger_swipe_right",
        FourFingerSwipeUp => "4finger_swipe_up",
        FourFingerSwipeDown => "4finger_swipe_down",
        FourFingerTap => "4finger_tap",
        PinchIn => "pinch_in",
        PinchOut => "pinch_out",
    }
}

fn action_name(a: GestureAction) -> &'static str {
    use GestureAction::*;
    match a {
        None => "none",
        MissionControl => "overview",
        AppExpose => "app_expose",
        DesktopShow => "show_desktop",
        NextDesktop => "workspace_next",
        PrevDesktop => "workspace_prev",
        BrowserBack => "browser_back",
        BrowserForward => "browser_forward",
        SwitchApp => "switch_app",
        NotificationCenter => "notifications",
        VolumeUp => "volume_up",
        VolumeDown => "volume_down",
        MediaPlayPause => "play_pause",
        ZoomIn => "zoom_in",
        ZoomOut => "zoom_out",
        Custom => "custom",
    }
}
