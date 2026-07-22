//! Collapse multi-interface HID/USB nodes into one logical Magic Trackpad.
//!
//! A single physical trackpad appears as many Windows SetupAPI / Linux HID
//! nodes (mouse boot protocol, multitouch, vendor, USB MI_*, composite parent).
//!
//! Windows puts the USB serial only on the composite parent
//! (`USB\VID_…&PID_…\SERIAL`); child `MI_` / HID nodes share VID/PID but not
//! the serial string — so we always group by VID+PID+bus first, then only
//! split when multiple distinct serials are present (two trackpads).

use std::collections::HashMap;

use crate::models::{model_name_for_pid, ConnectionType, DeviceInfo};

/// Merge interface-level nodes into physical devices.
pub fn collapse_physical_devices(devices: Vec<DeviceInfo>) -> Vec<DeviceInfo> {
    if devices.len() <= 1 {
        return devices;
    }

    // Primary bucket: same Apple trackpad model on same bus type.
    let mut primary: HashMap<String, Vec<DeviceInfo>> = HashMap::new();
    for d in devices {
        let key = bus_group_key(&d);
        primary.entry(key).or_default().push(d);
    }

    let mut out: Vec<DeviceInfo> = Vec::new();
    for (bus_key, members) in primary {
        out.extend(split_or_merge_by_serial(bus_key, members));
    }

    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// VID + PID + usb/bt — NOT serial (serial is only on one Windows node).
fn bus_group_key(d: &DeviceInfo) -> String {
    let vid = d.vid.unwrap_or(0);
    let pid = d.pid.unwrap_or(0);
    let conn = connection_bucket(d.connection);

    // Prefer parsing VID/PID from the instance path when present.
    let path = d.path.as_deref().unwrap_or(d.id.as_str());
    let upper = path.to_ascii_uppercase();
    if let Some(base) = windows_vid_pid_base(&upper) {
        return format!("{base}:{conn}");
    }

    format!("{vid:04X}:{pid:04X}:{conn}")
}

fn split_or_merge_by_serial(bus_key: String, members: Vec<DeviceInfo>) -> Vec<DeviceInfo> {
    // Collect distinct hardware serials from any member.
    let mut by_serial: HashMap<String, Vec<DeviceInfo>> = HashMap::new();
    let mut no_serial: Vec<DeviceInfo> = Vec::new();

    for d in members {
        if let Some(s) = device_serial(&d) {
            by_serial.entry(s).or_default().push(d);
        } else {
            no_serial.push(d);
        }
    }

    match by_serial.len() {
        0 => {
            // No serial anywhere — one logical device for this bus/model.
            vec![merge_group(bus_key, no_serial)]
        }
        1 => {
            // Normal case: composite has serial, children do not → one device.
            let (serial, mut group) = by_serial.into_iter().next().unwrap();
            group.extend(no_serial);
            vec![merge_group(format!("{bus_key}:{serial}"), group)]
        }
        _ => {
            // Two+ trackpads same model: keep serial groups separate.
            // Orphans without serial join the largest serial group (best-effort).
            let mut groups: Vec<(String, Vec<DeviceInfo>)> = by_serial.into_iter().collect();
            groups.sort_by_key(|(_, g)| std::cmp::Reverse(g.len()));
            if !no_serial.is_empty() {
                groups[0].1.extend(no_serial);
            }
            groups
                .into_iter()
                .map(|(serial, group)| merge_group(format!("{bus_key}:{serial}"), group))
                .collect()
        }
    }
}

fn device_serial(d: &DeviceInfo) -> Option<String> {
    let path = d.path.as_deref().unwrap_or(d.id.as_str());
    extract_serial(&path.to_ascii_uppercase())
        .or_else(|| extract_serial_from_notes(d))
        .or_else(|| {
            d.id.strip_prefix("apple-trackpad:")
                .and_then(|rest| {
                    // apple-trackpad:05AC:0324:SERIAL or apple-trackpad:SERIAL
                    let last = rest.rsplit(':').next().unwrap_or(rest);
                    if last.len() >= 8
                        && last.chars().any(|c| c.is_ascii_alphabetic())
                        && !last.contains('/')
                    {
                        Some(last.to_ascii_uppercase())
                    } else {
                        None
                    }
                })
        })
}

fn connection_bucket(c: ConnectionType) -> &'static str {
    match c {
        ConnectionType::Bluetooth => "bt",
        ConnectionType::UsbC | ConnectionType::Usb => "usb",
        ConnectionType::Unknown => "unk",
    }
}

/// `USB\VID_05AC&PID_0324\J84HRT00JQC0000509` → serial
fn extract_serial(upper_path: &str) -> Option<String> {
    for part in upper_path.split(|c| c == '\\' || c == '/') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        if p.starts_with("VID_")
            || p.starts_with("PID_")
            || p.starts_with("MI_")
            || p.starts_with("COL")
            || p == "HID"
            || p == "USB"
            || p == "USBPRINT"
            || p == "BTHENUM"
            || p == "SWD"
            || p.starts_with("BTH")
        {
            continue;
        }
        if p.contains('&') {
            continue;
        }
        if p.len() >= 8
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            && p.chars().any(|c| c.is_ascii_alphabetic())
        {
            return Some(p.to_string());
        }
    }
    None
}

fn extract_serial_from_notes(d: &DeviceInfo) -> Option<String> {
    for n in &d.notes {
        if let Some(s) = n.strip_prefix("Serial ") {
            let s = s.trim();
            if !s.is_empty() {
                return Some(s.to_ascii_uppercase());
            }
        }
    }
    None
}

/// Normalize `HID\VID_05AC&PID_0324&MI_01&COL01\…` → `VID_05AC&PID_0324`
fn windows_vid_pid_base(upper: &str) -> Option<String> {
    let vid_at = upper.find("VID_")?;
    let slice = &upper[vid_at..];
    let bytes = slice.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    if !slice.starts_with("VID_") {
        return None;
    }
    out.push_str("VID_");
    i += 4;
    let mut hex = 0;
    while i < bytes.len() && hex < 4 && (bytes[i] as char).is_ascii_hexdigit() {
        out.push((bytes[i] as char).to_ascii_uppercase());
        i += 1;
        hex += 1;
    }
    if hex != 4 {
        return None;
    }
    if !slice.get(i..).is_some_and(|s| s.starts_with("&PID_")) {
        return None;
    }
    out.push_str("&PID_");
    i += 5;
    hex = 0;
    while i < bytes.len() && hex < 4 && (bytes[i] as char).is_ascii_hexdigit() {
        out.push((bytes[i] as char).to_ascii_uppercase());
        i += 1;
        hex += 1;
    }
    if hex != 4 {
        return None;
    }
    Some(out)
}

fn merge_group(key: String, mut members: Vec<DeviceInfo>) -> DeviceInfo {
    debug_assert!(!members.is_empty());
    members.sort_by_key(|d| std::cmp::Reverse(interface_score(d)));
    let best = members[0].clone();
    let interface_count = members.len();

    let mut driver_bound = None;
    for m in &members {
        if m.driver_bound == Some(true) {
            driver_bound = Some(true);
            break;
        }
        if m.driver_bound == Some(false) {
            driver_bound = Some(false);
        }
    }

    let battery = members.iter().find_map(|m| m.battery.clone());

    let connection = members
        .iter()
        .map(|m| m.connection)
        .find(|c| !matches!(c, ConnectionType::Unknown))
        .unwrap_or(best.connection);

    let vid = best.vid.or_else(|| members.iter().find_map(|m| m.vid));
    let pid = best.pid.or_else(|| members.iter().find_map(|m| m.pid));

    let model = pid
        .map(model_name_for_pid)
        .unwrap_or_else(|| best.model.clone());

    let name = members
        .iter()
        .map(|m| m.name.as_str())
        .find(|n| {
            let l = n.to_ascii_lowercase();
            l.contains("precision touchpad")
                || l.contains("magic trackpad")
                || (l.contains("apple") && l.contains("trackpad"))
        })
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            if model.to_ascii_lowercase().contains("magic trackpad") {
                format!("Apple Inc. {model}")
            } else {
                best.name.clone()
            }
        });

    let serial = members.iter().find_map(device_serial);

    let path = members
        .iter()
        .find(|m| {
            let p = m.path.as_deref().unwrap_or("");
            let u = p.to_ascii_uppercase();
            (u.contains("USB\\VID_") || u.contains(r"USB\VID_")) && !u.contains("&MI_")
        })
        .or_else(|| {
            members.iter().find(|m| {
                let p = m.path.as_deref().unwrap_or("");
                !p.to_ascii_uppercase().contains("&COL")
            })
        })
        .and_then(|m| m.path.clone())
        .or(best.path.clone());

    let mut notes = Vec::new();
    if let Some(ref s) = serial {
        notes.push(format!("Serial {s}"));
    }
    if interface_count > 1 {
        notes.push(format!(
            "{interface_count} USB/HID interfaces collapsed into one device"
        ));
    }

    let id = if let Some(ref s) = serial {
        format!(
            "apple-trackpad:{:04X}:{:04X}:{s}",
            vid.unwrap_or(0),
            pid.unwrap_or(0)
        )
    } else {
        format!("apple-trackpad:{key}")
    };

    DeviceInfo {
        id,
        name,
        model,
        vid,
        pid,
        connection,
        connected: members.iter().any(|m| m.connected),
        battery,
        driver_bound,
        path,
        notes,
    }
}

/// Higher score = better representative for the physical device.
fn interface_score(d: &DeviceInfo) -> i32 {
    let mut s = 0i32;
    let name = d.name.to_ascii_lowercase();
    let path = d
        .path
        .as_deref()
        .unwrap_or(d.id.as_str())
        .to_ascii_uppercase();

    if name.contains("precision touchpad") {
        s += 200;
    }
    if name.contains("magic trackpad") {
        s += 150;
    }
    if name.contains("touchpad") || name.contains("trackpad") {
        s += 80;
    }
    if name.contains("mouse") {
        s += 20;
    }
    if name.contains("vendor-defined") {
        s -= 10;
    }
    if d.driver_bound == Some(true) {
        s += 100;
    }
    if d.battery.is_some() {
        s += 40;
    }
    // Prefer composite parent path (serial) slightly under Precision name
    if path.contains("USB\\VID_") && !path.contains("&MI_") {
        s += 50;
    }
    if path.contains("&COL") {
        s -= 30;
    }
    if path.contains("&MI_") {
        s -= 15;
    }
    if path.starts_with("HID\\") {
        s -= 5;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(id: &str, name: &str, path: &str) -> DeviceInfo {
        DeviceInfo {
            id: id.into(),
            name: name.into(),
            model: "Magic Trackpad 3 (USB-C)".into(),
            vid: Some(0x05AC),
            pid: Some(0x0324),
            connection: ConnectionType::UsbC,
            connected: true,
            battery: None,
            driver_bound: Some(false),
            path: Some(path.into()),
            notes: vec![],
        }
    }

    #[test]
    fn collapses_windows_multi_interface() {
        let devices = vec![
            dev(
                "1",
                "HID-compliant mouse",
                r"HID\VID_05AC&PID_0324&MI_01&COL01\9&305DF14F&0&0000",
            ),
            dev(
                "2",
                "HID-compliant touch pad",
                r"HID\VID_05AC&PID_0324&MI_01&COL02\9&305DF14F&0&0001",
            ),
            dev(
                "3",
                "USB Composite Device",
                r"USB\VID_05AC&PID_0324\J84HRT00JQC0000509",
            ),
            dev(
                "4",
                "USB Input Device",
                r"USB\VID_05AC&PID_0324&MI_00\8&107BF680&0&0000",
            ),
            dev(
                "5",
                "HID-compliant vendor-defined device",
                r"HID\VID_05AC&PID_0324&MI_02\9&C86B38D&0&0000",
            ),
        ];
        let out = collapse_physical_devices(devices);
        assert_eq!(out.len(), 1, "expected one physical device, got {out:?}");
        assert!(
            out[0]
                .notes
                .iter()
                .any(|n| n.contains("Serial J84HRT00JQC0000509")),
            "notes: {:?}",
            out[0].notes
        );
        assert!(
            out[0].notes.iter().any(|n| n.contains("5 USB/HID")),
            "notes: {:?}",
            out[0].notes
        );
    }

    #[test]
    fn collapses_composite_serial_with_precision_mi_nodes() {
        // Reproduces the Windows v0.2.0 dual-card bug: composite parent has
        // serial; Precision user-mode stack is under MI_ without serial.
        let devices = vec![
            dev(
                "composite",
                "USB Composite Device",
                r"USB\VID_05AC&PID_0324\J84HRT00JQC0000509",
            ),
            {
                let mut d = dev(
                    "ptp",
                    "Apple USB Precision Touchpad Device (User-mode)",
                    r"USB\VID_05AC&PID_0324&MI_01\8&107BF680&0&0001",
                );
                d.driver_bound = Some(true);
                d
            },
            dev(
                "hid-mouse",
                "HID-compliant mouse",
                r"HID\VID_05AC&PID_0324&MI_01&COL01\9&305DF14F&0&0000",
            ),
            dev(
                "hid-tp",
                "HID-compliant touch pad",
                r"HID\VID_05AC&PID_0324&MI_01&COL02\9&305DF14F&0&0001",
            ),
            dev(
                "mi0",
                "USB Input Device",
                r"USB\VID_05AC&PID_0324&MI_00\8&107BF680&0&0000",
            ),
            dev(
                "mi2",
                "USB Input Device",
                r"USB\VID_05AC&PID_0324&MI_02\8&107BF680&0&0002",
            ),
            dev(
                "vendor",
                "HID-compliant vendor-defined device",
                r"HID\VID_05AC&PID_0324&MI_02\9&C86B38D&0&0000",
            ),
            dev(
                "vendor2",
                "HID-compliant vendor-defined device",
                r"HID\VID_05AC&PID_0324&MI_01&COL03\9&305DF14F&0&0002",
            ),
        ];
        let out = collapse_physical_devices(devices);
        assert_eq!(out.len(), 1, "got {} devices: {:?}", out.len(), out);
        assert_eq!(out[0].driver_bound, Some(true));
        assert!(
            out[0]
                .name
                .to_ascii_lowercase()
                .contains("precision touchpad"),
            "name={}",
            out[0].name
        );
        assert!(out[0]
            .notes
            .iter()
            .any(|n| n.contains("Serial J84HRT00JQC0000509")));
        assert!(out[0].notes.iter().any(|n| n.contains("8 USB/HID")));
    }

    #[test]
    fn two_serials_stay_separate() {
        let devices = vec![
            dev(
                "a",
                "USB Composite Device",
                r"USB\VID_05AC&PID_0324\SERIALAAAA001",
            ),
            dev(
                "b",
                "USB Composite Device",
                r"USB\VID_05AC&PID_0324\SERIALBBBB002",
            ),
        ];
        let out = collapse_physical_devices(devices);
        assert_eq!(out.len(), 2);
    }
}
