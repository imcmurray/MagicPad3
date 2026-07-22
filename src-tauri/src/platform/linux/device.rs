//! Discover Apple Magic Trackpads via sysfs HID / input / power_supply.
//!
//! A single USB Magic Trackpad exposes multiple HID interfaces (multitouch,
//! mouse boot protocol, etc.). We collapse those into one logical device per
//! serial number / USB parent.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::device_group::collapse_physical_devices;
use crate::error::AppResult;
use crate::models::{
    is_known_trackpad_pid, model_name_for_pid, BatteryInfo, ConnectionType, DeviceInfo, APPLE_VID,
};

const SYS_HID: &str = "/sys/bus/hid/devices";
const SYS_INPUT: &str = "/sys/class/input";
const SYS_POWER: &str = "/sys/class/power_supply";

#[derive(Default)]
struct HidCandidate {
    path: PathBuf,
    fname: String,
    vid: u16,
    pid: u16,
    uniq: Option<String>,
    phys: Option<String>,
    hid_name: Option<String>,
    /// Prefer interface 0 / lower HID instance index as the primary face.
    rank: u32,
    usb_parent: Option<String>,
}

pub fn enumerate_devices() -> AppResult<Vec<DeviceInfo>> {
    // group_key -> best candidate
    let mut groups: HashMap<String, HidCandidate> = HashMap::new();

    if let Ok(entries) = fs::read_dir(SYS_HID) {
        for ent in entries.flatten() {
            if let Some(c) = parse_hid_candidate(&ent.path()) {
                let key = group_key(&c);
                match groups.get(&key) {
                    Some(existing) if existing.rank <= c.rank => {}
                    _ => {
                        groups.insert(key, c);
                    }
                }
            }
        }
    }

    let mut devices: Vec<DeviceInfo> = groups
        .into_values()
        .map(|c| candidate_to_device(c))
        .collect();

    // Bluetooth / odd stacks: input nodes not already covered by HID groups.
    if let Ok(entries) = fs::read_dir(SYS_INPUT) {
        for ent in entries.flatten() {
            let class_path = ent.path();
            // Prefer .../name directly under the input node
            let name = fs::read_to_string(class_path.join("name"))
                .or_else(|_| fs::read_to_string(class_path.join("device/name")))
                .unwrap_or_default();
            let name_l = name.trim().to_ascii_lowercase();
            if !(name_l.contains("magic trackpad")
                || name_l.contains("apple inc. magic trackpad"))
            {
                continue;
            }

            // Resolve to real device path for dedupe
            let resolved = fs::canonicalize(&class_path).unwrap_or_else(|_| class_path.clone());
            let resolved_s = resolved.display().to_string();

            // Skip if this input hangs under a USB/HID path we already reported
            if devices.iter().any(|d| {
                d.path
                    .as_ref()
                    .map(|p| resolved_s.contains(p.trim_end_matches('/')) || p.contains(&resolved_s))
                    .unwrap_or(false)
                    || device_shares_usb_parent(d, &resolved_s)
            }) {
                continue;
            }

            // Also skip if serial in power_supply / uniq already represented
            let uniq = read_uevent_field(&class_path.join("device"), "HID_UNIQ")
                .or_else(|| read_uevent_field(&resolved, "HID_UNIQ"));
            if let Some(ref u) = uniq {
                if devices.iter().any(|d| d.id.contains(u) || d.notes.iter().any(|n| n.contains(u))) {
                    continue;
                }
            }

            let connection = if name_l.contains("bluetooth") || resolved_s.contains("bluetooth") {
                ConnectionType::Bluetooth
            } else {
                ConnectionType::Unknown
            };

            let id = if let Some(u) = uniq.clone() {
                format!("input:{u}")
            } else {
                format!("input:{}", ent.file_name().to_string_lossy())
            };

            devices.push(DeviceInfo {
                id,
                name: name.trim().to_string(),
                model: "Magic Trackpad".into(),
                vid: Some(APPLE_VID),
                pid: None,
                connection,
                connected: true,
                battery: None,
                driver_bound: Some(true),
                path: Some(class_path.display().to_string()),
                notes: uniq
                    .map(|u| vec![format!("Serial {u}")])
                    .unwrap_or_default(),
            });
        }
    }

    for d in &mut devices {
        if d.battery.is_none() {
            d.battery = battery_for_device(d);
        }
    }

    // Final collapse in case input + HID paths both contributed.
    let mut devices = collapse_physical_devices(devices);

    // Stable order: USB-C first, then by id
    devices.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(devices)
}

pub fn read_battery(device_id: &str) -> AppResult<Option<BatteryInfo>> {
    let mut dummy = DeviceInfo {
        id: device_id.to_string(),
        name: String::new(),
        model: String::new(),
        vid: None,
        pid: None,
        connection: ConnectionType::Unknown,
        connected: true,
        battery: None,
        driver_bound: None,
        path: None,
        notes: vec![],
    };
    // Extract serial from id if present: "usb:J84…" or "0003:05AC:0324…"
    if let Some(serial) = extract_serial_hint(device_id) {
        dummy.notes.push(format!("Serial {serial}"));
    }
    Ok(battery_for_device(&dummy))
}

fn battery_for_device(d: &DeviceInfo) -> Option<BatteryInfo> {
    let serial_hint = d
        .notes
        .iter()
        .find_map(|n| n.strip_prefix("Serial ").map(|s| s.to_string()))
        .or_else(|| extract_serial_hint(&d.id));

    let mut best: Option<BatteryInfo> = None;

    let Ok(entries) = fs::read_dir(SYS_POWER) else {
        return None;
    };

    for ent in entries.flatten() {
        let path = ent.path();
        let type_ = fs::read_to_string(path.join("type")).unwrap_or_default();
        if !type_.trim().eq_ignore_ascii_case("Battery") {
            continue;
        }

        let fname = ent.file_name().to_string_lossy().to_string();
        let model = fs::read_to_string(path.join("model_name")).unwrap_or_default();
        let model_l = model.to_ascii_lowercase();

        let looks_trackpad = model_l.contains("trackpad")
            || model_l.contains("magic")
            || fname.to_ascii_lowercase().contains("trackpad");

        if !looks_trackpad {
            continue;
        }

        // Match serial when we have one (hid-SERIAL-battery-N)
        if let Some(ref serial) = serial_hint {
            if !fname.contains(serial) {
                continue;
            }
        }

        let percent = fs::read_to_string(path.join("capacity"))
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok());
        let status = fs::read_to_string(path.join("status")).unwrap_or_default();
        let charging = match status.trim().to_ascii_lowercase().as_str() {
            "charging" => Some(true),
            "full" => Some(false),
            "discharging" | "not charging" => Some(false),
            _ => None,
        };

        let candidate = BatteryInfo {
            percent,
            charging,
            source: path.display().to_string(),
        };

        best = Some(pick_better_battery(best, candidate));
    }

    // Fallback: any Apple trackpad battery if nothing serial-matched
    if best.is_none() && serial_hint.is_some() {
        // already filtered strictly; try without serial
        if let Ok(entries) = fs::read_dir(SYS_POWER) {
            for ent in entries.flatten() {
                let path = ent.path();
                let model = fs::read_to_string(path.join("model_name")).unwrap_or_default();
                if !model.to_ascii_lowercase().contains("trackpad") {
                    continue;
                }
                let percent = fs::read_to_string(path.join("capacity"))
                    .ok()
                    .and_then(|s| s.trim().parse::<u8>().ok());
                let status = fs::read_to_string(path.join("status")).unwrap_or_default();
                let charging = match status.trim().to_ascii_lowercase().as_str() {
                    "charging" => Some(true),
                    "full" | "discharging" | "not charging" => Some(false),
                    _ => None,
                };
                let candidate = BatteryInfo {
                    percent,
                    charging,
                    source: path.display().to_string(),
                };
                best = Some(pick_better_battery(best, candidate));
            }
        }
    }

    best
}

/// Prefer charging + higher capacity (MT3 often exposes two HID batteries).
fn pick_better_battery(a: Option<BatteryInfo>, b: BatteryInfo) -> BatteryInfo {
    let Some(a) = a else {
        return b;
    };
    let score = |x: &BatteryInfo| -> i32 {
        let mut s = x.percent.unwrap_or(0) as i32;
        if x.charging == Some(true) {
            s += 200;
        }
        s
    };
    if score(&b) >= score(&a) {
        b
    } else {
        a
    }
}

fn parse_hid_candidate(path: &Path) -> Option<HidCandidate> {
    // HID sysfs names look like: 0003:05AC:0324.0005
    let fname = path.file_name()?.to_string_lossy().to_string();
    let parts: Vec<&str> = fname.split([':', '.']).collect();
    if parts.len() < 3 {
        return None;
    }
    let vid = u16::from_str_radix(parts[1], 16).ok()?;
    let pid = u16::from_str_radix(parts[2], 16).ok()?;
    if vid != APPLE_VID {
        return None;
    }

    let uevent = fs::read_to_string(path.join("uevent")).unwrap_or_default();
    let uniq = field_from_uevent(&uevent, "HID_UNIQ");
    let phys = field_from_uevent(&uevent, "HID_PHYS");
    let hid_name = field_from_uevent(&uevent, "HID_NAME");

    let is_trackpad = is_known_trackpad_pid(pid)
        || hid_name
            .as_ref()
            .map(|n| n.to_ascii_lowercase().contains("trackpad"))
            .unwrap_or(false)
        || hid_has_trackpad_name(path)
        || uevent.to_ascii_lowercase().contains("trackpad");

    if !is_trackpad {
        return None;
    }

    // Instance index after the last '.' (0004, 0005, …) — lower is usually primary
    let instance = parts
        .last()
        .and_then(|s| u32::from_str_radix(s, 16).ok())
        .unwrap_or(999);
    // Prefer HID_PHYS ending in /input0
    let phys_rank = phys
        .as_ref()
        .and_then(|p| p.rsplit('/').next())
        .and_then(|s| s.strip_prefix("input"))
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(50);
    let rank = phys_rank.saturating_mul(1000) + instance;

    let usb_parent = find_usb_device_parent(path);

    Some(HidCandidate {
        path: path.to_path_buf(),
        fname,
        vid,
        pid,
        uniq,
        phys,
        hid_name,
        rank,
        usb_parent,
    })
}

fn group_key(c: &HidCandidate) -> String {
    if let Some(ref u) = c.uniq {
        if !u.is_empty() {
            return format!("uniq:{u}");
        }
    }
    if let Some(ref usb) = c.usb_parent {
        return format!("usb:{usb}");
    }
    // Fall back to vid:pid only (one device assumed if serial missing)
    format!("vidpid:{:04X}:{:04X}", c.vid, c.pid)
}

fn candidate_to_device(c: HidCandidate) -> DeviceInfo {
    let model = model_name_for_pid(c.pid);
    let connection = detect_connection(&c.path, c.pid);
    let friendly = c
        .hid_name
        .clone()
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| model.clone());

    let mut notes = Vec::new();
    if connection == ConnectionType::UsbC {
        notes.push("USB-C Magic Trackpad (A3120 lineage)".into());
    }
    if let Some(ref u) = c.uniq {
        notes.push(format!("Serial {u}"));
    }
    if let Some(ref p) = c.phys {
        notes.push(format!("HID {p}"));
    }

    // Stable id: prefer serial so refresh doesn't thrash
    let id = if let Some(ref u) = c.uniq {
        format!("apple-trackpad:{u}")
    } else if let Some(ref usb) = c.usb_parent {
        format!("apple-trackpad:{usb}")
    } else {
        c.fname.clone()
    };

    DeviceInfo {
        id,
        name: friendly,
        model,
        vid: Some(c.vid),
        pid: Some(c.pid),
        connection,
        connected: true,
        battery: None,
        driver_bound: Some(true),
        path: Some(c.path.display().to_string()),
        notes,
    }
}

fn device_shares_usb_parent(d: &DeviceInfo, resolved_input: &str) -> bool {
    // If device path is under same usb bus device directory
    if let Some(ref p) = d.path {
        // e.g. both under .../usb1/1-4/
        if let Some(parent) = find_usb_device_parent(Path::new(p)) {
            if resolved_input.contains(&parent) {
                return true;
            }
        }
    }
    false
}

fn find_usb_device_parent(path: &Path) -> Option<String> {
    // Walk up until a directory that looks like a USB device (e.g. 1-4) containing idVendor
    let mut cur = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    for _ in 0..16 {
        if cur.join("idVendor").is_file() && cur.join("idProduct").is_file() {
            return Some(cur.display().to_string());
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

fn hid_has_trackpad_name(path: &Path) -> bool {
    let input_root = path.join("input");
    if let Ok(entries) = fs::read_dir(input_root) {
        for ent in entries.flatten() {
            if let Ok(n) = fs::read_to_string(ent.path().join("name")) {
                if n.to_ascii_lowercase().contains("trackpad") {
                    return true;
                }
            }
        }
    }
    false
}

fn detect_connection(path: &Path, pid: u16) -> ConnectionType {
    let mut cur: PathBuf = path.to_path_buf();
    for _ in 0..12 {
        let s = cur.display().to_string().to_ascii_lowercase();
        if s.contains("bluetooth") || s.contains("/bluetooth/") {
            return ConnectionType::Bluetooth;
        }
        if s.contains("/usb") {
            return if pid == 0x0324 || pid == 0x0325 {
                ConnectionType::UsbC
            } else {
                ConnectionType::Usb
            };
        }
        if !cur.pop() {
            break;
        }
    }
    if pid == 0x0324 || pid == 0x0325 {
        ConnectionType::UsbC
    } else {
        ConnectionType::Unknown
    }
}

fn field_from_uevent(uevent: &str, key: &str) -> Option<String> {
    for line in uevent.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.strip_prefix('=').unwrap_or(rest);
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

fn read_uevent_field(dir: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(dir.join("uevent")).ok()?;
    field_from_uevent(&text, key)
}

fn extract_serial_hint(device_id: &str) -> Option<String> {
    // apple-trackpad:SERIAL or input:SERIAL or hid-SERIAL-battery
    if let Some(s) = device_id.strip_prefix("apple-trackpad:") {
        return Some(s.to_string());
    }
    if let Some(s) = device_id.strip_prefix("input:") {
        if !s.starts_with("event") && !s.starts_with("input") {
            return Some(s.to_string());
        }
    }
    None
}
