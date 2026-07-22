import { invoke } from "@tauri-apps/api/core";
import type {
  AppSnapshot,
  DeviceInfo,
  DriverActionResult,
  DriverStatus,
  GestureMap,
  LogEntry,
  PlatformInfo,
  TrackpadSettings,
} from "./types";

export const api = {
  snapshot: () => invoke<AppSnapshot>("get_snapshot"),
  platform: () => invoke<PlatformInfo>("get_platform"),
  version: () => invoke<string>("app_version"),
  listDevices: () => invoke<DeviceInfo[]>("list_devices"),
  refreshDevices: () => invoke<DeviceInfo[]>("refresh_devices"),
  getSettings: () => invoke<TrackpadSettings>("get_settings"),
  setSettings: (settings: TrackpadSettings) =>
    invoke<void>("set_settings", { settings }),
  resetSettings: () => invoke<TrackpadSettings>("reset_settings"),
  getGestures: () => invoke<GestureMap>("get_gestures"),
  setGestures: (gestures: GestureMap) =>
    invoke<void>("set_gestures", { gestures }),
  driverStatus: () => invoke<DriverStatus>("get_driver_status"),
  installDriver: () => invoke<DriverActionResult>("install_driver"),
  uninstallDriver: () => invoke<DriverActionResult>("uninstall_driver"),
  installHelpers: () => invoke<DriverActionResult>("install_system_helpers"),
  getLogs: () => invoke<LogEntry[]>("get_logs"),
  clearLogs: () => invoke<void>("clear_logs"),
  appendLog: (level: string, source: string, message: string) =>
    invoke<void>("append_log", { level, source, message }),
};
