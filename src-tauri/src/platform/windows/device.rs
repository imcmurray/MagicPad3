//! Device enumeration for Apple Magic Trackpads on Windows.
//!
//! Uses SetupAPI when the `windows` crate APIs are available. Falls back to
//! scanning common device instance path patterns via PowerShell-free registry
//! keys when possible.

use crate::error::AppResult;
use crate::models::{
    is_known_trackpad_pid, model_name_for_pid, BatteryInfo, ConnectionType, DeviceInfo, APPLE_VID,
};

/// Enumerate connected Magic Trackpad devices.
pub fn enumerate_devices() -> AppResult<Vec<DeviceInfo>> {
    let mut devices = Vec::new();

    // Prefer SetupAPI HID / USB enumeration.
    if let Ok(list) = setupapi_enumerate() {
        devices.extend(list);
    }

    // Supplement with Bluetooth paired Apple trackpads if not already found.
    if let Ok(bt) = bluetooth_enumerate() {
        for d in bt {
            if !devices.iter().any(|x| x.id == d.id) {
                devices.push(d);
            }
        }
    }

    // Ensure battery fields are filled when possible.
    for d in &mut devices {
        if d.battery.is_none() {
            d.battery = read_battery(&d.id).ok().flatten();
        }
    }

    Ok(devices)
}

pub fn read_battery(device_id: &str) -> AppResult<Option<BatteryInfo>> {
    // Precision driver / HID may expose battery via device properties.
    // Placeholder path: look for companion control panel state or WMI later.
    let _ = device_id;
    Ok(None)
}

#[cfg(windows)]
fn setupapi_enumerate() -> AppResult<Vec<DeviceInfo>> {
    use windows::core::GUID;
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
        SetupDiGetDeviceInstanceIdW, DIGCF_PRESENT, DIGCF_ALLCLASSES, SPDRP_DEVICEDESC,
        SPDRP_HARDWAREID, SP_DEVINFO_DATA,
    };
    use windows::Win32::Foundation::INVALID_HANDLE_VALUE;

    let mut out = Vec::new();

    unsafe {
        let handle = SetupDiGetClassDevsW(
            None,
            windows::core::PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_ALLCLASSES,
        )
        .map_err(|e| crate::error::AppError::msg(format!("SetupDiGetClassDevsW: {e}")))?;

        if handle == INVALID_HANDLE_VALUE {
            return Ok(out);
        }

        let mut index = 0u32;
        loop {
            let mut data = SP_DEVINFO_DATA {
                cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
                ClassGuid: GUID::zeroed(),
                DevInst: 0,
                Reserved: 0,
            };

            if SetupDiEnumDeviceInfo(handle, index, &mut data).is_err() {
                break;
            }
            index += 1;

            // Instance ID
            let mut id_buf = [0u16; 512];
            let mut req = 0u32;
            if SetupDiGetDeviceInstanceIdW(handle, &data, Some(&mut id_buf), &mut req).is_err() {
                continue;
            }
            let instance = wchar_to_string(&id_buf);
            let upper = instance.to_ascii_uppercase();
            if !upper.contains("VID_05AC") && !upper.contains("05AC") {
                // Also accept friendly names for Bluetooth PTP devices
                let desc = registry_property_string(handle, &data, SPDRP_DEVICEDESC)
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if !(desc.contains("magic trackpad")
                    || desc.contains("apple") && desc.contains("trackpad")
                    || desc.contains("precision touchpad") && desc.contains("apple"))
                {
                    continue;
                }
            }

            let hwids =
                registry_property_multi(handle, &data, SPDRP_HARDWAREID).unwrap_or_default();
            let (vid, pid) = parse_vid_pid_from_strings(&hwids).or_else(|| {
                parse_vid_pid_from_instance(&instance)
            });

            if let Some(p) = pid {
                if !is_known_trackpad_pid(p) && !upper.contains("TRACKPAD") {
                    // Skip non-trackpad Apple HIDs unless name matches
                    let desc = registry_property_string(handle, &data, SPDRP_DEVICEDESC)
                        .unwrap_or_default()
                        .to_ascii_lowercase();
                    if !desc.contains("trackpad") && !desc.contains("touchpad") {
                        continue;
                    }
                }
            }

            let name = registry_property_string(handle, &data, SPDRP_DEVICEDESC)
                .unwrap_or_else(|| "Apple Magic Trackpad".into());
            let model = pid
                .map(model_name_for_pid)
                .unwrap_or_else(|| name.clone());
            let connection = if upper.contains("BTH") || upper.contains("BLUETOOTH") {
                ConnectionType::Bluetooth
            } else if pid == Some(0x0324) || pid == Some(0x0325) {
                ConnectionType::UsbC
            } else if upper.contains("USB") {
                ConnectionType::Usb
            } else {
                ConnectionType::Unknown
            };

            let driver_bound = upper.contains("AMTPTP")
                || name.to_ascii_lowercase().contains("precision touchpad");

            out.push(DeviceInfo {
                id: instance.clone(),
                name,
                model,
                vid: vid.or(Some(APPLE_VID)),
                pid,
                connection,
                connected: true,
                battery: None,
                driver_bound: Some(driver_bound),
                path: Some(instance),
                notes: vec![],
            });
        }

        let _ = SetupDiDestroyDeviceInfoList(handle);
    }

    // Dedupe by id
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out.dedup_by(|a, b| a.id == b.id);
    Ok(out)
}

#[cfg(windows)]
fn bluetooth_enumerate() -> AppResult<Vec<DeviceInfo>> {
    // Additional BT pass is folded into SetupAPI for now.
    Ok(vec![])
}

#[cfg(windows)]
unsafe fn registry_property_string(
    handle: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    data: &windows::Win32::Devices::DeviceAndDriverInstallation::SP_DEVINFO_DATA,
    prop: windows::Win32::Devices::DeviceAndDriverInstallation::SETUP_DI_REGISTRY_PROPERTY,
) -> Option<String> {
    use windows::Win32::Devices::DeviceAndDriverInstallation::SetupDiGetDeviceRegistryPropertyW;
    let mut buf = [0u8; 1024];
    let mut req = 0u32;
    let mut reg_type = 0u32;
    if SetupDiGetDeviceRegistryPropertyW(
        handle,
        data,
        prop,
        Some(&mut reg_type),
        Some(&mut buf),
        &mut req,
    )
    .is_err()
    {
        return None;
    }
    let u16s: Vec<u16> = buf
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|c| *c != 0)
        .collect();
    Some(String::from_utf16_lossy(&u16s))
}

#[cfg(windows)]
unsafe fn registry_property_multi(
    handle: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    data: &windows::Win32::Devices::DeviceAndDriverInstallation::SP_DEVINFO_DATA,
    prop: windows::Win32::Devices::DeviceAndDriverInstallation::SETUP_DI_REGISTRY_PROPERTY,
) -> Option<Vec<String>> {
    use windows::Win32::Devices::DeviceAndDriverInstallation::SetupDiGetDeviceRegistryPropertyW;
    let mut buf = [0u8; 4096];
    let mut req = 0u32;
    let mut reg_type = 0u32;
    if SetupDiGetDeviceRegistryPropertyW(
        handle,
        data,
        prop,
        Some(&mut reg_type),
        Some(&mut buf),
        &mut req,
    )
    .is_err()
    {
        return None;
    }
    let u16s: Vec<u16> = buf
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let mut out = Vec::new();
    let mut cur = Vec::new();
    for w in u16s {
        if w == 0 {
            if cur.is_empty() {
                break;
            }
            out.push(String::from_utf16_lossy(&cur));
            cur.clear();
        } else {
            cur.push(w);
        }
    }
    Some(out)
}

#[cfg(windows)]
fn wchar_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

fn parse_vid_pid_from_instance(instance: &str) -> (Option<u16>, Option<u16>) {
    parse_vid_pid_from_strings(&[instance.to_string()])
}

fn parse_vid_pid_from_strings(parts: &[String]) -> (Option<u16>, Option<u16>) {
    let mut vid = None;
    let mut pid = None;
    for s in parts {
        let u = s.to_ascii_uppercase();
        if let Some(v) = extract_hex_after(&u, "VID_") {
            vid = Some(v);
        }
        if let Some(p) = extract_hex_after(&u, "PID_") {
            pid = Some(p);
        }
    }
    (vid, pid)
}

fn extract_hex_after(s: &str, key: &str) -> Option<u16> {
    let idx = s.find(key)?;
    let rest = &s[idx + key.len()..];
    let hex: String = rest
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .take(4)
        .collect();
    u16::from_str_radix(&hex, 16).ok()
}

// Non-Windows compile stubs (so the module type-checks in docs; real builds use cfg).
#[cfg(not(windows))]
fn setupapi_enumerate() -> AppResult<Vec<DeviceInfo>> {
    Ok(vec![])
}

#[cfg(not(windows))]
fn bluetooth_enumerate() -> AppResult<Vec<DeviceInfo>> {
    Ok(vec![])
}
