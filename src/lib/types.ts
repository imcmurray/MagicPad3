export type ConnectionType = "usb_c" | "usb" | "bluetooth" | "unknown";
export type PlatformKind = "windows" | "linux" | "macos" | "other";
export type DriverState =
  | "unknown"
  | "not_installed"
  | "installed"
  | "outdated"
  | "conflict"
  | "not_applicable";

export interface BatteryInfo {
  percent: number | null;
  charging: boolean | null;
  source: string;
}

export interface DeviceInfo {
  id: string;
  name: string;
  model: string;
  vid: number | null;
  pid: number | null;
  connection: ConnectionType;
  connected: boolean;
  battery: BatteryInfo | null;
  driverBound: boolean | null;
  path: string | null;
  notes: string[];
}

export interface TrackpadSettings {
  pointerSpeed: number;
  acceleration: number;
  tapToClick: boolean;
  naturalScroll: boolean;
  pinchSensitivity: number;
  dragLock: boolean;
  forceThreshold: number;
  rightClickTwoFinger: boolean;
  horizontalScroll: boolean;
}

export type GestureTrigger =
  | "three_finger_swipe_left"
  | "three_finger_swipe_right"
  | "three_finger_swipe_up"
  | "three_finger_swipe_down"
  | "three_finger_tap"
  | "four_finger_swipe_left"
  | "four_finger_swipe_right"
  | "four_finger_swipe_up"
  | "four_finger_swipe_down"
  | "four_finger_tap"
  | "pinch_in"
  | "pinch_out";

export type GestureAction =
  | "none"
  | "mission_control"
  | "app_expose"
  | "desktop_show"
  | "next_desktop"
  | "prev_desktop"
  | "browser_back"
  | "browser_forward"
  | "switch_app"
  | "notification_center"
  | "volume_up"
  | "volume_down"
  | "media_play_pause"
  | "zoom_in"
  | "zoom_out"
  | "screenshot"
  | "custom";

export interface GestureBinding {
  trigger: GestureTrigger;
  action: GestureAction;
  custom: string | null;
  available: boolean;
}

export interface GestureMap {
  bindings: GestureBinding[];
  backend: string;
}

export interface DriverStatus {
  state: DriverState;
  name: string;
  version: string | null;
  detail: string;
  canInstall: boolean;
  canUninstall: boolean;
  recommendedSource: string | null;
  conflicts: string[];
}

export interface DriverActionResult {
  success: boolean;
  message: string;
  logLines: string[];
  needsReboot: boolean;
}

export interface PlatformInfo {
  kind: PlatformKind;
  osName: string;
  osVersion: string | null;
  arch: string;
  desktop: string | null;
  capabilities: string[];
}

export interface LogEntry {
  id: string;
  timestamp: string;
  level: string;
  source: string;
  message: string;
}

export interface AppSnapshot {
  platform: PlatformInfo;
  devices: DeviceInfo[];
  settings: TrackpadSettings;
  gestures: GestureMap;
  driver: DriverStatus;
}

export interface GestureDaemonStatus {
  available: boolean;
  running: boolean;
  libinputOk: boolean;
  wtypeOk: boolean;
  inputGroup: boolean;
  unitInstalled: boolean;
  message: string;
}

export type ThemeMode = "system" | "light" | "dark";
export type NavId = "status" | "settings" | "gestures" | "driver" | "logs" | "help";
