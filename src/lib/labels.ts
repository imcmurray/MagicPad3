import type {
  ConnectionType,
  DriverState,
  GestureAction,
  GestureTrigger,
} from "./types";

export function connectionLabel(c: ConnectionType): string {
  switch (c) {
    case "usb_c":
      return "USB-C";
    case "usb":
      return "USB";
    case "bluetooth":
      return "Bluetooth";
    default:
      return "Unknown";
  }
}

export function driverStateLabel(s: DriverState): string {
  switch (s) {
    case "installed":
      return "Installed";
    case "not_installed":
      return "Not installed";
    case "outdated":
      return "Outdated";
    case "conflict":
      return "Conflict";
    case "not_applicable":
      return "Not applicable";
    default:
      return "Unknown";
  }
}

export function triggerLabel(t: GestureTrigger): string {
  return t
    .replace(/_/g, " ")
    .replace(/\b\w/g, (c) => c.toUpperCase())
    .replace("Three Finger", "3-finger")
    .replace("Four Finger", "4-finger");
}

export function actionLabel(a: GestureAction): string {
  const map: Record<GestureAction, string> = {
    none: "None",
    mission_control: "Overview / Mission Control",
    app_expose: "App expose",
    desktop_show: "Show desktop",
    next_desktop: "Next desktop",
    prev_desktop: "Previous desktop",
    browser_back: "Browser back",
    browser_forward: "Browser forward",
    switch_app: "Switch app",
    notification_center: "Notifications",
    volume_up: "Volume up",
    volume_down: "Volume down",
    media_play_pause: "Play / Pause",
    zoom_in: "Zoom in (Ctrl+=)",
    zoom_out: "Zoom out (Ctrl+-)",
    custom: "Custom",
  };
  return map[a] ?? a;
}

export const ALL_ACTIONS: GestureAction[] = [
  "none",
  "mission_control",
  "app_expose",
  "desktop_show",
  "next_desktop",
  "prev_desktop",
  "browser_back",
  "browser_forward",
  "switch_app",
  "notification_center",
  "volume_up",
  "volume_down",
  "media_play_pause",
  "zoom_in",
  "zoom_out",
  "custom",
];

export function hexId(n: number | null | undefined): string {
  if (n == null) return "—";
  return "0x" + n.toString(16).toUpperCase().padStart(4, "0");
}
