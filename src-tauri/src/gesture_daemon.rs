//! Linux multi-finger gesture daemon.
//!
//! Listens to `libinput debug-events` (from package `libinput-tools`) and maps
//! swipe / pinch gestures to compositor actions via `wtype` (Wayland) or
//! `xdotool` (X11). Tuned defaults match EndeavourOS Budgie + labwc keybinds:
//!   Super+Page_Up/Down → workspace left/right
//!   Super+d            → show desktop
//!   Super+a            → Raven / notification panel
//!   Super+Tab          → app switcher
//!
//! Run:  magicpad-companion --gestures

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::models::{
    GestureAction, GestureBinding, GestureMap, GestureTrigger,
};

static RUNNING: AtomicBool = AtomicBool::new(true);

pub fn run() -> i32 {
    ctrlc_install();
    let config_dir = config_dir();
    log::info!("MagicPad gesture daemon starting (config={})", config_dir.display());

    if !which("libinput") {
        log::error!(
            "libinput CLI not found. Install:  sudo pacman -S libinput-tools"
        );
        return 1;
    }
    if !which("wtype") && !which("xdotool") {
        log::error!(
            "No key injector found. Install:  sudo pacman -S wtype"
        );
        return 1;
    }
    log_device_access();

    let mut last_load = Instant::now() - Duration::from_secs(60);
    let mut map = load_map(&config_dir);
    log::info!("backend={} bindings={}", map.backend, map.bindings.len());

    while RUNNING.load(Ordering::SeqCst) {
        if last_load.elapsed() > Duration::from_secs(2) {
            map = load_map(&config_dir);
            last_load = Instant::now();
        }

        match run_libinput_session(&map) {
            Ok(()) => {
                if !RUNNING.load(Ordering::SeqCst) {
                    break;
                }
                log::warn!("libinput exited; restarting in 1s…");
                std::thread::sleep(Duration::from_secs(1));
            }
            Err(e) => {
                log::error!("{e}");
                std::thread::sleep(Duration::from_secs(3));
            }
        }
    }
    log::info!("gesture daemon stopped");
    0
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("magicpad-companion")
}

fn load_map(config_dir: &Path) -> GestureMap {
    let path = config_dir.join("gestures.json");
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(mut map) = serde_json::from_str::<GestureMap>(&text) {
            if map.backend.is_empty() {
                map.backend = "libinput-daemon".into();
            }
            return map;
        }
    }
    let mut map = GestureMap::default();
    map.backend = "libinput-daemon".into();
    map
}

fn run_libinput_session(map: &GestureMap) -> Result<(), String> {
    let mut child = Command::new("libinput")
        .args(["debug-events"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            format!(
                "failed to start `libinput debug-events`: {e}. \
                 Install libinput-tools and ensure access to /dev/input \
                 (sudo usermod -aG input $USER && re-login)."
            )
        })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "no stdout from libinput".to_string())?;
    let reader = BufReader::new(stdout);

    let mut swipe: Option<SwipeState> = None;
    let mut pinch: Option<PinchState> = None;
    let mut hold: Option<HoldState> = None;
    let mut last_fire = Instant::now() - Duration::from_secs(1);

    for line in reader.lines() {
        if !RUNNING.load(Ordering::SeqCst) {
            break;
        }
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Help diagnose devices that emit gestures under unexpected names
        if line.to_ascii_uppercase().contains("GESTURE_") {
            log::info!("libinput: {line}");
        }

        if let Some(ev) = parse_event(line) {
            match ev {
                Parsed::SwipeBegin { fingers } => {
                    hold = None; // swipe cancels pending hold/tap
                    swipe = Some(SwipeState {
                        fingers,
                        dx: 0.0,
                        dy: 0.0,
                    });
                }
                Parsed::SwipeUpdate { fingers, dx, dy } => {
                    if let Some(s) = swipe.as_mut() {
                        if s.fingers == fingers {
                            s.dx += dx;
                            s.dy += dy;
                        }
                    } else {
                        swipe = Some(SwipeState {
                            fingers,
                            dx,
                            dy,
                        });
                    }
                }
                Parsed::SwipeEnd { fingers } => {
                    if let Some(s) = swipe.take() {
                        let f = if fingers > 0 { fingers } else { s.fingers };
                        maybe_fire_swipe(map, f, s.dx, s.dy, &mut last_fire);
                    }
                }
                Parsed::PinchBegin { fingers } => {
                    hold = None;
                    pinch = Some(PinchState {
                        fingers,
                        scale: 1.0,
                    });
                }
                Parsed::PinchUpdate { scale, .. } => {
                    if let Some(p) = pinch.as_mut() {
                        p.scale = scale;
                    }
                }
                Parsed::PinchEnd => {
                    if let Some(p) = pinch.take() {
                        maybe_fire_pinch(map, p.scale, &mut last_fire);
                    }
                }
                Parsed::HoldBegin { fingers } => {
                    hold = Some(HoldState {
                        fingers,
                        started: Instant::now(),
                    });
                }
                Parsed::HoldEnd { fingers } => {
                    if let Some(h) = hold.take() {
                        let f = if fingers > 0 { fingers } else { h.fingers };
                        // Short hold ≈ multi-finger tap (libinput has no GESTURE_TAP)
                        maybe_fire_tap(map, f, h.started.elapsed(), &mut last_fire);
                    }
                }
            }
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[derive(Debug)]
struct SwipeState {
    fingers: u8,
    dx: f64,
    dy: f64,
}

#[derive(Debug)]
struct PinchState {
    #[allow(dead_code)]
    fingers: u8,
    scale: f64,
}

#[derive(Debug)]
struct HoldState {
    fingers: u8,
    started: Instant,
}

#[derive(Debug)]
enum Parsed {
    SwipeBegin { fingers: u8 },
    SwipeUpdate { fingers: u8, dx: f64, dy: f64 },
    SwipeEnd { fingers: u8 },
    PinchBegin { fingers: u8 },
    PinchUpdate { #[allow(dead_code)] fingers: u8, scale: f64 },
    PinchEnd,
    HoldBegin { fingers: u8 },
    HoldEnd { fingers: u8 },
}

/// Parse a single libinput debug-events line.
fn parse_event(line: &str) -> Option<Parsed> {
    // Examples:
    //  event3  GESTURE_SWIPE_BEGIN  +1.234s  3
    //  event3  GESTURE_SWIPE_UPDATE +1.235s  3  0.50/ -0.10 ( 0.50/ -0.10 unaccelerated)
    //  event3  GESTURE_SWIPE_END    +1.400s  3
    //  event3  GESTURE_PINCH_BEGIN  +2.0s  2
    //  event3  GESTURE_PINCH_UPDATE +2.1s  2 1.05 (0.50/0.00 unaccelerated)
    //  event3  GESTURE_HOLD_BEGIN   +3.0s  3
    //  event3  GESTURE_HOLD_END     +3.1s  3
    let upper = line.to_ascii_uppercase();
    if !upper.contains("GESTURE_") {
        return None;
    }

    if upper.contains("GESTURE_SWIPE_BEGIN") {
        return Some(Parsed::SwipeBegin {
            fingers: extract_fingers(line).unwrap_or(3),
        });
    }
    if upper.contains("GESTURE_SWIPE_END") {
        return Some(Parsed::SwipeEnd {
            fingers: extract_fingers(line).unwrap_or(0),
        });
    }
    if upper.contains("GESTURE_SWIPE_UPDATE") {
        let fingers = extract_fingers(line).unwrap_or(3);
        let (dx, dy) = extract_dx_dy(line).unwrap_or((0.0, 0.0));
        return Some(Parsed::SwipeUpdate { fingers, dx, dy });
    }
    if upper.contains("GESTURE_PINCH_BEGIN") {
        return Some(Parsed::PinchBegin {
            fingers: extract_fingers(line).unwrap_or(2),
        });
    }
    if upper.contains("GESTURE_PINCH_END") {
        return Some(Parsed::PinchEnd);
    }
    if upper.contains("GESTURE_PINCH_UPDATE") {
        let fingers = extract_fingers(line).unwrap_or(2);
        let scale = extract_scale(line).unwrap_or(1.0);
        return Some(Parsed::PinchUpdate { fingers, scale });
    }
    if upper.contains("GESTURE_HOLD_BEGIN") {
        return Some(Parsed::HoldBegin {
            fingers: extract_fingers(line).unwrap_or(3),
        });
    }
    if upper.contains("GESTURE_HOLD_END") {
        return Some(Parsed::HoldEnd {
            fingers: extract_fingers(line).unwrap_or(0),
        });
    }
    None
}

fn extract_fingers(line: &str) -> Option<u8> {
    // After the timestamp field, first integer 2–5 is finger count
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, p) in parts.iter().enumerate() {
        if p.contains("GESTURE_") {
            // look ahead for a bare digit
            for q in parts.iter().skip(i + 1).take(4) {
                if let Ok(n) = q.parse::<u8>() {
                    if (2..=5).contains(&n) {
                        return Some(n);
                    }
                }
            }
        }
    }
    None
}

fn extract_dx_dy(line: &str) -> Option<(f64, f64)> {
    // Formats seen from libinput debug-events:
    //   "12.5/-0.3"
    //   "12.5/ -0.3"
    //   "12.5/" "-0.3"  (split across tokens)
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, token) in parts.iter().enumerate() {
        let cleaned = token.trim_matches(|c: char| c == '(' || c == ')');
        if let Some((a, b)) = cleaned.split_once('/') {
            let a = a.trim();
            let b = b.trim();
            if let Ok(dx) = a.parse::<f64>() {
                if let Ok(dy) = b.parse::<f64>() {
                    return Some((dx, dy));
                }
                // "12.5/" with dy in next token
                if b.is_empty() {
                    if let Some(next) = parts.get(i + 1) {
                        let nb = next
                            .trim_matches(|c: char| c == '(' || c == ')')
                            .trim();
                        if let Ok(dy) = nb.parse::<f64>() {
                            return Some((dx, dy));
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_scale(line: &str) -> Option<f64> {
    // pinch update: "... 2 1.05 (..."
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, p) in parts.iter().enumerate() {
        if p.contains("GESTURE_PINCH_UPDATE") {
            for q in parts.iter().skip(i + 1).take(5) {
                if let Ok(n) = q.parse::<f64>() {
                    if n > 0.1 && n < 10.0 && n != 2.0 && n != 3.0 && n != 4.0 {
                        return Some(n);
                    }
                    // finger count might be 2; scale often follows
                }
            }
            // second float after fingers
            let mut seen_finger = false;
            for q in parts.iter().skip(i + 1) {
                if let Ok(n) = q.parse::<f64>() {
                    if !seen_finger && (2.0..=5.0).contains(&n) && n.fract() == 0.0 {
                        seen_finger = true;
                        continue;
                    }
                    if seen_finger {
                        return Some(n);
                    }
                }
            }
        }
    }
    None
}

fn maybe_fire_swipe(
    map: &GestureMap,
    fingers: u8,
    dx: f64,
    dy: f64,
    last_fire: &mut Instant,
) {
    // libinput units are often small; keep this modest so short swipes still fire
    const THRESH: f64 = 3.0;
    if dx.abs() < THRESH && dy.abs() < THRESH {
        log::debug!("swipe ignored (below threshold) fingers={fingers} dx={dx:.2} dy={dy:.2}");
        return;
    }
    if last_fire.elapsed() < Duration::from_millis(350) {
        return;
    }

    let trigger = if dx.abs() > dy.abs() {
        if dx > 0.0 {
            match fingers {
                3 => GestureTrigger::ThreeFingerSwipeRight,
                4 => GestureTrigger::FourFingerSwipeRight,
                _ => return,
            }
        } else {
            match fingers {
                3 => GestureTrigger::ThreeFingerSwipeLeft,
                4 => GestureTrigger::FourFingerSwipeLeft,
                _ => return,
            }
        }
    } else if dy > 0.0 {
        match fingers {
            3 => GestureTrigger::ThreeFingerSwipeDown,
            4 => GestureTrigger::FourFingerSwipeDown,
            _ => return,
        }
    } else {
        match fingers {
            3 => GestureTrigger::ThreeFingerSwipeUp,
            4 => GestureTrigger::FourFingerSwipeUp,
            _ => return,
        }
    };

    if fire_trigger(map, trigger) {
        *last_fire = Instant::now();
        log::info!(
            "gesture {:?} fingers={} dx={:.1} dy={:.1}",
            trigger,
            fingers,
            dx,
            dy
        );
    }
}

fn maybe_fire_tap(
    map: &GestureMap,
    fingers: u8,
    held_for: Duration,
    last_fire: &mut Instant,
) {
    // Multi-finger "tap" ≈ short hold without a swipe/pinch following
    if held_for > Duration::from_millis(450) {
        return;
    }
    if last_fire.elapsed() < Duration::from_millis(400) {
        return;
    }
    let trigger = match fingers {
        3 => GestureTrigger::ThreeFingerTap,
        4 => GestureTrigger::FourFingerTap,
        _ => return,
    };
    if fire_trigger(map, trigger) {
        *last_fire = Instant::now();
        log::info!("gesture {:?} fingers={} held={:?}", trigger, fingers, held_for);
    }
}

fn maybe_fire_pinch(map: &GestureMap, scale: f64, last_fire: &mut Instant) {
    // Slightly longer debounce so continuous pinch doesn't spam zoom steps
    if last_fire.elapsed() < Duration::from_millis(280) {
        return;
    }
    // scale is absolute from gesture start (1.0 = no change)
    let trigger = if scale < 0.94 {
        GestureTrigger::PinchIn // fingers together → zoom out (map view away)
    } else if scale > 1.06 {
        GestureTrigger::PinchOut // fingers apart → zoom in
    } else {
        return;
    };
    if fire_trigger(map, trigger) {
        *last_fire = Instant::now();
        log::info!("gesture {:?} scale={:.2}", trigger, scale);
    }
}

fn fire_trigger(map: &GestureMap, trigger: GestureTrigger) -> bool {
    let Some(binding) = map.bindings.iter().find(|b| b.trigger == trigger) else {
        return false;
    };
    if binding.action == GestureAction::None {
        return false;
    }
    execute_action(binding);
    true
}

fn execute_action(binding: &GestureBinding) {
    if binding.action == GestureAction::Custom {
        if let Some(cmd) = binding.custom.as_deref() {
            let _ = Command::new("sh").args(["-c", cmd]).spawn();
        }
        return;
    }

    // Prefer explicit custom override even for named actions
    if let Some(cmd) = binding.custom.as_deref() {
        if !cmd.is_empty() {
            let _ = Command::new("sh").args(["-c", cmd]).spawn();
            return;
        }
    }

    // Prefer spawning apps (screenshot tool) before key injection
    if let Some(cmd) = action_to_command(binding.action) {
        match Command::new("sh").args(["-c", cmd]).status() {
            Ok(s) if s.success() => return,
            Ok(_) | Err(_) => {
                // fall through to key chord fallback when available
            }
        }
    }

    if let Some(keys) = action_to_keys(binding.action) {
        inject_keys(&keys);
    }
}

/// Wayland/X11 key chords matching Budgie+labwc defaults.
fn action_to_keys(action: GestureAction) -> Option<KeyChord> {
    use GestureAction::*;
    match action {
        PrevDesktop => Some(KeyChord::super_key("Prior")), // Super+Page_Up
        NextDesktop => Some(KeyChord::super_key("Next")),  // Super+Page_Down
        DesktopShow => Some(KeyChord::super_key("d")),
        NotificationCenter => Some(KeyChord::super_key("a")), // Budgie Raven
        SwitchApp => Some(KeyChord::super_key("Tab")),
        MissionControl => Some(KeyChord::super_key("Tab")), // best available on labwc
        AppExpose => Some(KeyChord {
            mods: vec!["alt"],
            key: "Tab".into(),
        }),
        BrowserBack => Some(KeyChord {
            mods: vec!["alt"],
            key: "Left".into(),
        }),
        BrowserForward => Some(KeyChord {
            mods: vec!["alt"],
            key: "Right".into(),
        }),
        VolumeUp => Some(KeyChord {
            mods: vec![],
            key: "XF86AudioRaiseVolume".into(),
        }),
        VolumeDown => Some(KeyChord {
            mods: vec![],
            key: "XF86AudioLowerVolume".into(),
        }),
        MediaPlayPause => Some(KeyChord {
            mods: vec![],
            key: "XF86AudioPlay".into(),
        }),
        // Browsers, Electron apps, many viewers: Ctrl+= / Ctrl+-
        ZoomIn => Some(KeyChord {
            mods: vec!["ctrl"],
            key: "equal".into(),
        }),
        ZoomOut => Some(KeyChord {
            mods: vec!["ctrl"],
            key: "minus".into(),
        }),
        // Prefer launching the app via action_to_command; Print as last resort
        Screenshot => Some(KeyChord {
            mods: vec![],
            key: "Print".into(),
        }),
        GestureAction::None | Custom => Option::None,
    }
}

fn action_to_command(action: GestureAction) -> Option<&'static str> {
    match action {
        GestureAction::VolumeUp => {
            Some("pactl set-sink-volume @DEFAULT_SINK@ +5% 2>/dev/null || true")
        }
        GestureAction::VolumeDown => {
            Some("pactl set-sink-volume @DEFAULT_SINK@ -5% 2>/dev/null || true")
        }
        GestureAction::Screenshot => Some(
            // Budgie Screenshot interactive UI (EndeavourOS Budgie default).
            // Exit non-zero if none found so we can fall back to Print key.
            "if command -v org.buddiesofbudgie.BudgieScreenshot >/dev/null; then \
               org.buddiesofbudgie.BudgieScreenshot -i; \
             elif command -v dbus-send >/dev/null; then \
               dbus-send --type=method_call \
                 --dest=org.buddiesofbudgie.BudgieScreenshotControl \
                 /org/buddiesofbudgie/ScreenshotControl \
                 org.buddiesofbudgie.BudgieScreenshotControl.StartMainWindow; \
             elif command -v gnome-screenshot >/dev/null; then \
               gnome-screenshot -i; \
             elif command -v flameshot >/dev/null; then \
               flameshot gui; \
             else exit 1; fi",
        ),
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct KeyChord {
    mods: Vec<&'static str>,
    key: String,
}

impl KeyChord {
    fn super_key(key: &str) -> Self {
        Self {
            mods: vec!["logo"],
            key: key.into(),
        }
    }
}

fn inject_keys(chord: &KeyChord) {
    if which("wtype") {
        let mut cmd = Command::new("wtype");
        for m in &chord.mods {
            cmd.args(["-M", m]);
        }
        cmd.args(["-k", &chord.key]);
        for m in chord.mods.iter().rev() {
            cmd.args(["-m", m]);
        }
        match cmd.status() {
            Ok(s) if s.success() => return,
            Ok(s) => log::warn!("wtype exited {:?}", s.code()),
            Err(e) => log::warn!("wtype failed: {e}"),
        }
    }

    if which("xdotool") {
        let mut args = vec!["key".to_string()];
        let mut combo = String::new();
        for m in &chord.mods {
            let xm = match *m {
                "logo" | "super" => "super",
                "alt" => "alt",
                "ctrl" | "control" => "ctrl",
                "shift" => "shift",
                other => other,
            };
            if !combo.is_empty() {
                combo.push('+');
            }
            combo.push_str(xm);
        }
        if !combo.is_empty() {
            combo.push('+');
        }
        combo.push_str(&chord.key);
        args.push(combo);
        let _ = Command::new("xdotool").args(&args).status();
    }
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn ctrlc_install() {
    let _ = ctrlc::set_handler(|| {
        RUNNING.store(false, Ordering::SeqCst);
    });
}

// ── Service management (called from GUI backend) ───────────────────────────

pub use crate::models::GestureDaemonStatus;

pub fn daemon_status() -> GestureDaemonStatus {
    let libinput_ok = which("libinput");
    let wtype_ok = which("wtype") || which("xdotool");
    let input_group = user_in_input_group() || user_in_input_group_passwd();
    let can_read = can_read_trackpad_events();
    let unit_installed = unit_path().map(|p| p.exists()).unwrap_or(false);
    let running = unit_is_active();

    let mut parts = Vec::new();
    if !libinput_ok {
        parts.push("install libinput-tools");
    }
    if !wtype_ok {
        parts.push("install wtype");
    }
    if !input_group {
        parts.push("sudo usermod -aG input $USER  (then re-login, or re-run install --gestures)");
    } else if !can_read {
        parts.push(
            "session cannot open /dev/input yet — restart magicpad-gestures (uses sg input) or re-login",
        );
    }
    if libinput_ok && wtype_ok && input_group && can_read && !running {
        parts.push("click Save gestures to start the daemon");
    }

    let message = if running && can_read {
        "Gesture daemon is running".into()
    } else if running && !can_read {
        "Daemon process is up but cannot read the trackpad (input permissions). Restart the service after joining the input group.".into()
    } else if parts.is_empty() {
        "Ready — save gestures to start the daemon".into()
    } else {
        format!("Setup needed: {}", parts.join("; "))
    };

    GestureDaemonStatus {
        available: cfg!(target_os = "linux"),
        running,
        libinput_ok,
        wtype_ok,
        input_group,
        unit_installed,
        message,
    }
}

pub fn ensure_daemon_running() -> Result<String, String> {
    #[cfg(not(target_os = "linux"))]
    {
        return Err("Gesture daemon is Linux-only".into());
    }
    #[cfg(target_os = "linux")]
    {
        if !which("libinput") {
            return Err(
                "libinput CLI missing. Run: sudo pacman -S libinput-tools".into(),
            );
        }
        if !which("wtype") && !which("xdotool") {
            return Err("wtype missing. Run: sudo pacman -S wtype".into());
        }
        if !user_in_input_group() && !user_in_input_group_passwd() {
            return Err(
                "Your user is not in the 'input' group (needed to read trackpad events). \
                 Run:  sudo usermod -aG input $USER   then: \
                 systemctl --user restart magicpad-gestures.service \
                 (or log out and back in)."
                    .into(),
            );
        }

        install_user_unit()?;
        // reload + enable --now
        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();
        let status = Command::new("systemctl")
            .args(["--user", "enable", "--now", "magicpad-gestures.service"])
            .output()
            .map_err(|e| format!("systemctl failed: {e}"))?;
        if !status.status.success() {
            return Err(format!(
                "failed to start magicpad-gestures.service: {}",
                String::from_utf8_lossy(&status.stderr)
            ));
        }
        // Also write XDG autostart as backup
        let _ = install_autostart();
        Ok("Gesture daemon enabled and started (systemd --user)".into())
    }
}

fn install_user_unit() -> Result<(), String> {
    let unit_dir = dirs::config_dir()
        .ok_or("no config dir")?
        .join("systemd/user");
    std::fs::create_dir_all(&unit_dir).map_err(|e| e.to_string())?;
    let unit = unit_dir.join("magicpad-gestures.service");
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .display()
        .to_string();
    let uid = users_uid();
    let wayland = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into());
    let xdg = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{uid}"));
    // `sg input` applies the input group even when the user systemd session was
    // started before `usermod -aG input` (no full re-login required).
    let exec = if which("sg") {
        format!("/usr/bin/sg input -c '{exe} --gestures'")
    } else {
        format!("{exe} --gestures")
    };
    let body = format!(
        r#"[Unit]
Description=MagicPad Companion multi-finger gesture daemon
Documentation=https://github.com/imcmurray/MagicPad3
PartOf=graphical-session.target
After=graphical-session.target

[Service]
Type=simple
ExecStart={exec}
Restart=on-failure
RestartSec=2
Environment=RUST_LOG=info
Environment=WAYLAND_DISPLAY={wayland}
Environment=XDG_RUNTIME_DIR={xdg}

[Install]
WantedBy=graphical-session.target
WantedBy=default.target
"#
    );
    std::fs::write(&unit, body).map_err(|e| e.to_string())?;
    Ok(())
}

/// Quick self-check used at daemon start.
fn log_device_access() {
    if can_read_trackpad_events() {
        log::info!("trackpad event devices are readable (input group / sg ok)");
    } else {
        log::error!(
            "cannot open /dev/input event devices — gestures will not work. \
             Fix: sudo usermod -aG input $USER && systemctl --user restart magicpad-gestures.service"
        );
    }
}

fn install_autostart() -> Result<(), String> {
    let dir = dirs::config_dir()
        .ok_or("no config dir")?
        .join("autostart");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .display()
        .to_string();
    let body = format!(
        r#"[Desktop Entry]
Type=Application
Name=MagicPad Gestures
Comment=Multi-finger trackpad gestures for MagicPad Companion
Exec={exe} --gestures
X-GNOME-Autostart-enabled=true
Hidden=false
NoDisplay=true
"#
    );
    std::fs::write(dir.join("magicpad-gestures.desktop"), body).map_err(|e| e.to_string())?;
    Ok(())
}

fn unit_path() -> Option<PathBuf> {
    Some(
        dirs::config_dir()?
            .join("systemd/user/magicpad-gestures.service"),
    )
}

fn unit_is_active() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", "magicpad-gestures.service"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        || {
            // fallback: any --gestures process
            Command::new("pgrep")
                .args(["-f", "magicpad-companion --gestures"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
}

fn user_in_input_group() -> bool {
    // Current process credentials (session groups)
    Command::new("id")
        .arg("-nG")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.split_whitespace().any(|g| g == "input"))
        .unwrap_or(false)
}

/// True if /etc/group lists the user in `input` (even before re-login).
fn user_in_input_group_passwd() -> bool {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_default();
    if user.is_empty() {
        return false;
    }
    std::fs::read_to_string("/etc/group")
        .ok()
        .map(|g| {
            g.lines().any(|line| {
                let mut parts = line.split(':');
                let name = parts.next().unwrap_or("");
                let members = parts.nth(2).unwrap_or(""); // skip passwd, gid
                name == "input" && members.split(',').any(|m| m == user)
            })
        })
        .unwrap_or(false)
}

fn can_read_trackpad_events() -> bool {
    // Direct probe with current credentials
    if let Ok(rd) = std::fs::read_dir("/dev/input") {
        for ent in rd.flatten() {
            let name = ent.file_name();
            let n = name.to_string_lossy();
            if !n.starts_with("event") {
                continue;
            }
            if std::fs::File::open(ent.path()).is_ok() {
                return true;
            }
        }
    }
    // Session may lack input group while /etc/group has it — probe via sg
    if which("sg") {
        let status = Command::new("sg")
            .args(["input", "-c", "test -r /dev/input/event0 -o -r /dev/input/event3 -o -r /dev/input/event8"])
            .status();
        if let Ok(s) = status {
            if s.success() {
                return true;
            }
        }
        // Broader: any event* readable under sg
        let status = Command::new("sg")
            .args([
                "input",
                "-c",
                "sh -c 'for f in /dev/input/event*; do test -r \"$f\" && exit 0; done; exit 1'",
            ])
            .status();
        if let Ok(s) = status {
            return s.success();
        }
    }
    false
}

fn users_uid() -> u32 {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_swipe_update() {
        let line = " event3   GESTURE_SWIPE_UPDATE  +1.23s\t3\t 12.5/ -0.3 ( 12.5/ -0.3 unaccelerated)";
        match parse_event(line) {
            Some(Parsed::SwipeUpdate { fingers, dx, dy }) => {
                assert_eq!(fingers, 3);
                assert!(dx > 10.0, "dx={dx}");
                assert!(dy < 0.0, "dy={dy}");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parse_swipe_begin() {
        let line = " event3  GESTURE_SWIPE_BEGIN +0.1s  3";
        match parse_event(line) {
            Some(Parsed::SwipeBegin { fingers }) => assert_eq!(fingers, 3),
            other => panic!("{other:?}"),
        }
    }
}
