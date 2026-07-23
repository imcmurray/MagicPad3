#!/usr/bin/env bash
# MagicPad Companion — EndeavourOS / Arch Linux installer
#
# Installs:
#   • Runtime dependencies (pacman), including libinput-tools + wtype
#   • App binary + desktop entry + icons (from GitHub release .deb, or local build)
#   • udev rules for Magic Trackpad
#   • Multi-finger gesture daemon (user systemd: magicpad-gestures.service)
#   • Optional input-remapper profile
#
# Usage:
#   ./scripts/install-endeavouros.sh              # full install (latest release)
#   ./scripts/install-endeavouros.sh --helpers    # udev + gesture daemon only
#   ./scripts/install-endeavouros.sh --gestures   # gesture daemon only (app already installed)
#   ./scripts/install-endeavouros.sh --no-gestures  # full install without starting daemon
#   ./scripts/install-endeavouros.sh --local      # use local tauri release build
#   ./scripts/install-endeavouros.sh --deb PATH   # install from a specific .deb
#   ./scripts/install-endeavouros.sh --user       # install app under ~/.local
#   ./scripts/install-endeavouros.sh --uninstall  # remove app + udev + gesture service
#
# Docs: https://github.com/imcmurray/MagicPad3/blob/main/docs/linux-install.md

set -euo pipefail

REPO_OWNER="imcmurray"
REPO_NAME="MagicPad3"
REPO_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}"
RELEASES_API="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RULE_SRC_TREE="$ROOT/packaging/linux/99-magic-trackpad.rules"
PROFILE_SRC_TREE="$ROOT/packaging/linux/input-remapper-profiles/MagicPad.json"
UNIT_SRC_TREE="$ROOT/packaging/linux/magicpad-companion.service"
GESTURES_UNIT_SRC="$ROOT/packaging/linux/magicpad-gestures.service"
RULE_DST="/etc/udev/rules.d/99-magic-trackpad.rules"

APP_ID="magicpad-companion"
APP_NAME="MagicPad Companion"
LIB_DIR_NAME="MagicPad Companion"
GESTURES_UNIT="magicpad-gestures.service"

MODE="full"          # full | helpers | gestures | uninstall
SOURCE="release"     # release | local | deb
DEB_PATH=""
USER_INSTALL=0
SKIP_DEPS=0
WITH_REMAPPER=0
INSTALL_GESTURES=1   # 0 with --no-gestures

log()  { printf '==> %s\n' "$*"; }
warn() { printf 'WARNING: %s\n' "$*" >&2; }
die()  { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

usage() {
  awk 'NR==1{next} /^#/{sub(/^# ?/,""); print; next} {exit}' "$0"
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --helpers|--helpers-only) MODE="helpers"; shift ;;
    --gestures|--gestures-only) MODE="gestures"; shift ;;
    --no-gestures) INSTALL_GESTURES=0; shift ;;
    --local) SOURCE="local"; shift ;;
    --deb)
      SOURCE="deb"
      DEB_PATH="${2:-}"
      [[ -n "$DEB_PATH" ]] || die "--deb requires a path"
      shift 2
      ;;
    --user) USER_INSTALL=1; shift ;;
    --skip-deps) SKIP_DEPS=1; shift ;;
    --with-remapper) WITH_REMAPPER=1; shift ;;
    --uninstall) MODE="uninstall"; shift ;;
    -h|--help) usage ;;
    *) die "Unknown option: $1 (try --help)" ;;
  esac
done

# Target user for home dirs / systemd --user (when script is run via sudo)
target_user() {
  if [[ "$(id -u)" -eq 0 && -n "${SUDO_USER:-}" && "${SUDO_USER}" != "root" ]]; then
    echo "$SUDO_USER"
  else
    echo "${USER:-$(id -un)}"
  fi
}

target_home() {
  local u
  u="$(target_user)"
  getent passwd "$u" | cut -d: -f6
}

target_uid() {
  id -u "$(target_user)"
}

# Run a command as the desktop user (for systemctl --user, writing ~/.config)
as_user() {
  local u
  u="$(target_user)"
  if [[ "$(id -u)" -eq 0 && "$u" != "root" ]]; then
    local uid runtime
    uid="$(id -u "$u")"
    runtime="/run/user/${uid}"
    if command -v sudo >/dev/null 2>&1; then
      sudo -u "$u" --preserve-env=WAYLAND_DISPLAY \
        env "HOME=$(getent passwd "$u" | cut -d: -f6)" \
            "XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-$runtime}" \
            "XDG_CONFIG_HOME=$(getent passwd "$u" | cut -d: -f6)/.config" \
            "$@"
    else
      runuser -u "$u" -- env "HOME=$(getent passwd "$u" | cut -d: -f6)" \
        "XDG_RUNTIME_DIR=${runtime}" "$@"
    fi
  else
    "$@"
  fi
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "Missing required command: $1"
}

is_arch_family() {
  [[ -f /etc/arch-release ]] || [[ -f /etc/endeavouros-release ]] || grep -qiE 'arch|endeavouros' /etc/os-release 2>/dev/null
}

run_root() {
  if [[ "$(id -u)" -eq 0 ]]; then
    "$@"
  elif command -v sudo >/dev/null 2>&1; then
    sudo "$@"
  elif command -v pkexec >/dev/null 2>&1; then
    pkexec "$@"
  else
    die "Need root for: $* (install sudo or run as root)"
  fi
}

prefix_bin() {
  if [[ "$USER_INSTALL" -eq 1 ]]; then
    echo "${HOME}/.local/bin"
  else
    echo "/usr/local/bin"
  fi
}

prefix_share() {
  if [[ "$USER_INSTALL" -eq 1 ]]; then
    echo "${HOME}/.local/share"
  else
    echo "/usr/local/share"
  fi
}

prefix_lib() {
  if [[ "$USER_INSTALL" -eq 1 ]]; then
    echo "${HOME}/.local/lib"
  else
    echo "/usr/local/lib"
  fi
}

install_file() {
  # install_file MODE SRC DST
  local mode="$1" src="$2" dst="$3"
  if [[ "$dst" == /usr/* ]] || [[ "$dst" == /etc/* ]]; then
    run_root install -D -m "$mode" "$src" "$dst"
  else
    install -D -m "$mode" "$src" "$dst"
  fi
}

copy_tree() {
  local src="$1" dst="$2"
  if [[ "$dst" == /usr/* ]] || [[ "$dst" == /etc/* ]]; then
    run_root mkdir -p "$dst"
    run_root cp -a "$src"/. "$dst"/
  else
    mkdir -p "$dst"
    cp -a "$src"/. "$dst"/
  fi
}

remove_path() {
  local p="$1"
  if [[ -e "$p" ]] || [[ -L "$p" ]]; then
    if [[ "$p" == /usr/* ]] || [[ "$p" == /etc/* ]]; then
      run_root rm -rf "$p"
    else
      rm -rf "$p"
    fi
    log "Removed $p"
  fi
}

# ── Uninstall ──────────────────────────────────────────────────────────────
do_uninstall() {
  log "Uninstalling MagicPad Companion…"

  # Stop gesture daemon first
  if as_user systemctl --user is-enabled --quiet "$GESTURES_UNIT" 2>/dev/null \
    || as_user systemctl --user is-active --quiet "$GESTURES_UNIT" 2>/dev/null; then
    log "Stopping gesture daemon…"
    as_user systemctl --user disable --now "$GESTURES_UNIT" 2>/dev/null || true
  fi
  local home cfg
  home="$(target_home)"
  cfg="${home}/.config"
  remove_path "${cfg}/systemd/user/${GESTURES_UNIT}"
  remove_path "${cfg}/autostart/magicpad-gestures.desktop"

  remove_path "$(prefix_bin)/${APP_ID}"
  remove_path "$(prefix_lib)/${LIB_DIR_NAME}"
  remove_path "$(prefix_share)/applications/${APP_ID}.desktop"
  remove_path "$(prefix_share)/applications/${APP_NAME}.desktop"
  for s in 32x32 128x128 256x256 256x256@2; do
    remove_path "$(prefix_share)/icons/hicolor/${s}/apps/${APP_ID}.png"
  done
  if [[ -f "$RULE_DST" ]]; then
    run_root rm -f "$RULE_DST"
    run_root udevadm control --reload-rules || true
    run_root udevadm trigger || true
    log "Removed udev rule $RULE_DST"
  fi
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$(prefix_share)/applications" 2>/dev/null || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$(prefix_share)/icons/hicolor" 2>/dev/null || true
  fi
  log "Uninstall complete. Config under ~/.config/magicpad-companion was left in place."
}

# ── Gesture daemon (user systemd) ───────────────────────────────────────────
resolve_app_binary() {
  local candidates=(
    "$(prefix_bin)/${APP_ID}"
    "/usr/local/bin/${APP_ID}"
    "${HOME}/.local/bin/${APP_ID}"
    "/usr/bin/${APP_ID}"
    "$ROOT/src-tauri/target/release/${APP_ID}"
  )
  local c
  for c in "${candidates[@]}"; do
    if [[ -x "$c" ]]; then
      echo "$c"
      return 0
    fi
  done
  if command -v "$APP_ID" >/dev/null 2>&1; then
    command -v "$APP_ID"
    return 0
  fi
  return 1
}

seed_default_gestures_json() {
  local home cfg dir json
  home="$(target_home)"
  cfg="${home}/.config/magicpad-companion"
  dir="$cfg"
  json="${dir}/gestures.json"
  if [[ -f "$json" ]]; then
    log "Keeping existing gestures config: $json"
    return 0
  fi
  mkdir -p "$dir"
  # Matches GestureMap::default + pinch→zoom (v0.3.1+)
  cat > "$json" <<'JSON'
{
  "bindings": [
    { "trigger": "three_finger_swipe_left", "action": "prev_desktop", "custom": null, "available": true },
    { "trigger": "three_finger_swipe_right", "action": "next_desktop", "custom": null, "available": true },
    { "trigger": "three_finger_swipe_up", "action": "mission_control", "custom": null, "available": true },
    { "trigger": "three_finger_swipe_down", "action": "app_expose", "custom": null, "available": true },
    { "trigger": "three_finger_tap", "action": "screenshot", "custom": null, "available": true },
    { "trigger": "four_finger_swipe_left", "action": "browser_back", "custom": null, "available": true },
    { "trigger": "four_finger_swipe_right", "action": "browser_forward", "custom": null, "available": true },
    { "trigger": "four_finger_swipe_up", "action": "desktop_show", "custom": null, "available": true },
    { "trigger": "four_finger_swipe_down", "action": "notification_center", "custom": null, "available": true },
    { "trigger": "four_finger_tap", "action": "screenshot", "custom": null, "available": true },
    { "trigger": "pinch_in", "action": "zoom_out", "custom": null, "available": true },
    { "trigger": "pinch_out", "action": "zoom_in", "custom": null, "available": true }
  ],
  "backend": "libinput-daemon"
}
JSON
  # Fix ownership if we wrote as root into the user's home
  if [[ "$(id -u)" -eq 0 ]]; then
    local u
    u="$(target_user)"
    chown -R "$u:$u" "$cfg" 2>/dev/null || true
  fi
  log "Seeded default gestures → $json"
}

install_gesture_daemon() {
  log "Configuring multi-finger gesture daemon…"

  # Deps (may already be installed by install_deps)
  if [[ "$SKIP_DEPS" -eq 0 ]] && command -v pacman >/dev/null 2>&1; then
    run_root pacman -S --needed --noconfirm libinput-tools wtype 2>/dev/null \
      || warn "Could not install libinput-tools/wtype via pacman — install them manually."
  fi

  local u home uid runtime wayland exe unit_dir unit autostart_dir
  u="$(target_user)"
  home="$(target_home)"
  uid="$(target_uid)"
  runtime="${XDG_RUNTIME_DIR:-/run/user/${uid}}"
  wayland="${WAYLAND_DISPLAY:-wayland-0}"

  if ! exe="$(resolve_app_binary)"; then
    warn "magicpad-companion binary not found — install the app first, then re-run:"
    warn "  ./scripts/install-endeavouros.sh --gestures"
    return 1
  fi
  log "Gesture daemon binary: $exe"

  if ! command -v libinput >/dev/null 2>&1; then
    warn "libinput CLI missing (package: libinput-tools). Daemon cannot start yet."
  fi
  if ! command -v wtype >/dev/null 2>&1 && ! command -v xdotool >/dev/null 2>&1; then
    warn "wtype missing (package: wtype). Daemon cannot inject keys yet."
  fi

  # input group
  if ! id -nG "$u" 2>/dev/null | tr ' ' '\n' | grep -qx input; then
    log "Adding $u to the 'input' group (required to read trackpad events)…"
    run_root usermod -aG input "$u"
    warn "Log out and back in so 'input' group membership applies, then:"
    warn "  systemctl --user restart ${GESTURES_UNIT}"
  else
    log "User $u is in the input group."
  fi

  seed_default_gestures_json

  unit_dir="${home}/.config/systemd/user"
  mkdir -p "$unit_dir"
  unit="${unit_dir}/${GESTURES_UNIT}"

  cat > "$unit" <<EOF
[Unit]
Description=MagicPad Companion multi-finger gesture daemon
Documentation=${REPO_URL}
PartOf=graphical-session.target
After=graphical-session.target

[Service]
Type=simple
ExecStart=${exe} --gestures
Restart=on-failure
RestartSec=2
Environment=RUST_LOG=info
Environment=WAYLAND_DISPLAY=${wayland}
Environment=XDG_RUNTIME_DIR=${runtime}

[Install]
WantedBy=graphical-session.target
WantedBy=default.target
EOF

  autostart_dir="${home}/.config/autostart"
  mkdir -p "$autostart_dir"
  cat > "${autostart_dir}/magicpad-gestures.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=MagicPad Gestures
Comment=Multi-finger trackpad gestures for MagicPad Companion
Exec=${exe} --gestures
X-GNOME-Autostart-enabled=true
Hidden=false
NoDisplay=true
EOF

  if [[ "$(id -u)" -eq 0 ]]; then
    chown -R "$u:$u" "${home}/.config/systemd" "${home}/.config/autostart" \
      "${home}/.config/magicpad-companion" 2>/dev/null || true
  fi

  log "Installed user unit → $unit"
  log "Installed autostart → ${autostart_dir}/magicpad-gestures.desktop"

  # Enable + start under the user session when possible
  if as_user systemctl --user daemon-reload 2>/dev/null; then
    if as_user systemctl --user enable --now "$GESTURES_UNIT" 2>/dev/null; then
      log "Gesture daemon enabled and started (${GESTURES_UNIT})"
    else
      warn "Could not start ${GESTURES_UNIT} yet (no user session bus?)."
      warn "After login run:  systemctl --user enable --now ${GESTURES_UNIT}"
    fi
  else
    warn "systemctl --user not available in this context."
    warn "After graphical login run:"
    warn "  systemctl --user daemon-reload"
    warn "  systemctl --user enable --now ${GESTURES_UNIT}"
  fi

  if as_user systemctl --user is-active --quiet "$GESTURES_UNIT" 2>/dev/null; then
    log "Status: gesture daemon is active"
  else
    warn "Daemon not active yet — usually needs a re-login after joining 'input' group."
  fi
}

# ── Dependencies ────────────────────────────────────────────────────────────
install_deps() {
  [[ "$SKIP_DEPS" -eq 1 ]] && { log "Skipping dependency install (--skip-deps)"; return; }
  is_arch_family || warn "Not detected as Arch/EndeavourOS — pacman steps may fail."

  need_cmd pacman
  log "Installing runtime packages (pacman)…"
  local pkgs=(
    webkit2gtk-4.1
    gtk3
    libappindicator-gtk3
    librsvg
    xdg-utils
    # extract release .deb
    binutils
    tar
    curl
    # optional but useful
    polkit
    # multi-finger gesture daemon (MagicPad --gestures)
    libinput-tools
    wtype
  )
  run_root pacman -S --needed --noconfirm "${pkgs[@]}"

  # Gesture daemon needs /dev/input access
  local u="${SUDO_USER:-$USER}"
  if [[ -n "$u" && "$u" != "root" ]] && ! id -nG "$u" 2>/dev/null | tr ' ' '\n' | grep -qx input; then
    log "Adding $u to the 'input' group (required for trackpad gestures)…"
    run_root usermod -aG input "$u"
    warn "Log out and back in so the 'input' group membership applies."
  fi

  if [[ "$WITH_REMAPPER" -eq 1 ]]; then
    log "Installing input-remapper…"
    if pacman -Si input-remapper &>/dev/null; then
      run_root pacman -S --needed --noconfirm input-remapper
    else
      warn "input-remapper not in enabled repos (try AUR: yay -S input-remapper)"
    fi
  fi
}

# ── Helpers (udev, remapper profile, unit) ──────────────────────────────────
install_helpers() {
  local rule_src="$1"

  [[ -f "$rule_src" ]] || die "udev rule missing: $rule_src"
  log "Installing udev rule → $RULE_DST"
  run_root install -m 644 "$rule_src" "$RULE_DST"
  run_root udevadm control --reload-rules
  run_root udevadm trigger
  log "udev rules installed."

  if ! id -nG "${SUDO_USER:-$USER}" 2>/dev/null | tr ' ' '\n' | grep -qx input \
    && ! id -nG "$USER" | tr ' ' '\n' | grep -qx input; then
    local u="${SUDO_USER:-$USER}"
    warn "User '$u' is not in the 'input' group."
    log "To add (then log out/in):  sudo usermod -aG input $u"
    if [[ -t 0 ]]; then
      read -r -p "Add $u to 'input' group now? [y/N] " ans || true
      if [[ "${ans:-}" =~ ^[Yy]$ ]]; then
        run_root usermod -aG input "$u"
        log "Added. Log out and back in for group membership to apply."
      fi
    fi
  else
    log "User is already in the input group."
  fi

  local profile_src="$PROFILE_SRC_TREE"
  if [[ -f "$profile_src" ]]; then
    local dest_dir="${XDG_CONFIG_HOME:-$HOME/.config}/input-remapper-2/presets/Magic Trackpad"
    mkdir -p "$dest_dir"
    cp "$profile_src" "$dest_dir/MagicPad.json"
    mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/magicpad-companion/input-remapper"
    cp "$profile_src" "${XDG_CONFIG_HOME:-$HOME/.config}/magicpad-companion/input-remapper/MagicPad.json"
    log "Staged input-remapper profile → $dest_dir/MagicPad.json"
  fi

  if [[ -f "$UNIT_SRC_TREE" ]]; then
    local unit_dir="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
    mkdir -p "$unit_dir"
    cp "$UNIT_SRC_TREE" "$unit_dir/magicpad-companion.service"
    log "Staged user unit → $unit_dir/magicpad-companion.service (disabled by default)"
  fi
}

# ── Obtain package tree ─────────────────────────────────────────────────────
# Sets EXTRACT_ROOT to a directory containing usr/bin, usr/share, usr/lib
EXTRACT_ROOT=""
cleanup_extract() {
  if [[ -n "${EXTRACT_TMP:-}" && -d "${EXTRACT_TMP:-}" ]]; then
    rm -rf "$EXTRACT_TMP"
  fi
}
trap cleanup_extract EXIT

extract_deb() {
  local deb="$1"
  [[ -f "$deb" ]] || die "DEB not found: $deb"
  need_cmd ar
  need_cmd tar

  EXTRACT_TMP="$(mktemp -d -t magicpad-install.XXXXXX)"
  (
    cd "$EXTRACT_TMP"
    # Handle spaces in filename via copy
    cp "$deb" ./pkg.deb
    ar x pkg.deb
    local data
    data="$(echo data.tar.*)"
    tar xf $data
  )
  EXTRACT_ROOT="$EXTRACT_TMP"
  [[ -x "$EXTRACT_ROOT/usr/bin/${APP_ID}" ]] || die "DEB missing usr/bin/${APP_ID}"
  log "Extracted package from $(basename "$deb")"
}

download_latest_deb() {
  need_cmd curl
  log "Fetching latest release metadata…"
  local json
  json="$(curl -fsSL "$RELEASES_API")" || die "Failed to query GitHub releases (network?)"

  local url name
  url="$(printf '%s' "$json" | python3 -c '
import json,sys
rel=json.load(sys.stdin)
for a in rel.get("assets",[]):
    n=a.get("name","")
    if n.endswith(".deb") and "amd64" in n.lower():
        print(a["browser_download_url"]); print(n); break
' 2>/dev/null || true)"

  # Fallback without python: crude grep
  if [[ -z "$url" ]]; then
    url="$(printf '%s' "$json" | grep -oE 'https://[^"]+_amd64\.deb' | head -1 || true)"
    name="$(basename "$url")"
  else
    name="$(printf '%s\n' "$url" | sed -n '2p')"
    url="$(printf '%s\n' "$url" | sed -n '1p')"
  fi

  [[ -n "$url" ]] || die "No amd64 .deb asset found on latest release. Build with --local or pass --deb PATH."
  log "Downloading $name …"
  EXTRACT_TMP="$(mktemp -d -t magicpad-install.XXXXXX)"
  local deb_file="$EXTRACT_TMP/download.deb"
  curl -fL --progress-bar -o "$deb_file" "$url" || die "Download failed: $url"
  extract_deb "$deb_file"
}

use_local_build() {
  local bin="$ROOT/src-tauri/target/release/${APP_ID}"
  local deb_dir="$ROOT/src-tauri/target/release/bundle/deb"

  # Prefer newest local .deb by mtime
  local deb=""
  if [[ -d "$deb_dir" ]]; then
    deb="$(find "$deb_dir" -maxdepth 1 -type f -name '*.deb' -printf '%T@ %p\n' 2>/dev/null \
      | sort -nr | head -1 | cut -d' ' -f2- || true)"
  fi
  if [[ -n "$deb" && -f "$deb" ]]; then
    log "Using local DEB: $deb"
    extract_deb "$deb"
    return
  fi

  if [[ -x "$bin" ]]; then
    log "Using local binary: $bin (no DEB — icons/desktop may be minimal)"
    EXTRACT_TMP="$(mktemp -d -t magicpad-install.XXXXXX)"
    EXTRACT_ROOT="$EXTRACT_TMP"
    mkdir -p "$EXTRACT_ROOT/usr/bin" \
      "$EXTRACT_ROOT/usr/lib/${LIB_DIR_NAME}" \
      "$EXTRACT_ROOT/usr/share/applications" \
      "$EXTRACT_ROOT/usr/share/icons/hicolor/128x128/apps"
    cp "$bin" "$EXTRACT_ROOT/usr/bin/${APP_ID}"
    chmod 755 "$EXTRACT_ROOT/usr/bin/${APP_ID}"
    # packaging resources for in-app helper install
    if [[ -d "$ROOT/packaging/linux" ]]; then
      mkdir -p "$EXTRACT_ROOT/usr/lib/${LIB_DIR_NAME}/_up_/packaging"
      cp -a "$ROOT/packaging/linux" "$EXTRACT_ROOT/usr/lib/${LIB_DIR_NAME}/_up_/packaging/"
    fi
    if [[ -f "$ROOT/src-tauri/icons/128x128.png" ]]; then
      cp "$ROOT/src-tauri/icons/128x128.png" \
        "$EXTRACT_ROOT/usr/share/icons/hicolor/128x128/apps/${APP_ID}.png"
    fi
    cat > "$EXTRACT_ROOT/usr/share/applications/${APP_ID}.desktop" <<EOF
[Desktop Entry]
Categories=Utility;
Comment=Apple Magic Trackpad companion for Windows and Linux
Exec=${APP_ID}
StartupWMClass=${APP_ID}
Icon=${APP_ID}
Name=${APP_NAME}
Terminal=false
Type=Application
EOF
    return
  fi

  die "No local build found. Run: npm run tauri -- build --bundles deb   or omit --local to download a release."
}

# ── Install app files ───────────────────────────────────────────────────────
install_app_from_extract() {
  [[ -n "$EXTRACT_ROOT" && -d "$EXTRACT_ROOT" ]] || die "Nothing extracted to install"

  local bin_src="$EXTRACT_ROOT/usr/bin/${APP_ID}"
  local lib_src="$EXTRACT_ROOT/usr/lib/${LIB_DIR_NAME}"
  local bin_dst="$(prefix_bin)/${APP_ID}"
  local lib_dst="$(prefix_lib)/${LIB_DIR_NAME}"
  local share_dst
  share_dst="$(prefix_share)"

  log "Installing binary → $bin_dst"
  install_file 755 "$bin_src" "$bin_dst"

  if [[ -d "$lib_src" ]]; then
    log "Installing resources → $lib_dst"
    if [[ "$lib_dst" == /usr/* ]]; then
      run_root rm -rf "$lib_dst"
      run_root mkdir -p "$(dirname "$lib_dst")"
      run_root cp -a "$lib_src" "$lib_dst"
    else
      rm -rf "$lib_dst"
      mkdir -p "$(dirname "$lib_dst")"
      cp -a "$lib_src" "$lib_dst"
    fi
  fi

  # Icons
  if [[ -d "$EXTRACT_ROOT/usr/share/icons" ]]; then
    log "Installing icons…"
    while IFS= read -r -d '' png; do
      local rel="${png#"$EXTRACT_ROOT/usr/share/icons/"}"
      install_file 644 "$png" "${share_dst}/icons/${rel}"
    done < <(find "$EXTRACT_ROOT/usr/share/icons" -type f -name '*.png' -print0 2>/dev/null)
  fi

  # Desktop entry (normalize name to magicpad-companion.desktop)
  local desk_src=""
  if [[ -f "$EXTRACT_ROOT/usr/share/applications/${APP_ID}.desktop" ]]; then
    desk_src="$EXTRACT_ROOT/usr/share/applications/${APP_ID}.desktop"
  elif [[ -f "$EXTRACT_ROOT/usr/share/applications/${APP_NAME}.desktop" ]]; then
    desk_src="$EXTRACT_ROOT/usr/share/applications/${APP_NAME}.desktop"
  fi
  if [[ -n "$desk_src" ]]; then
    local desk_dst="${share_dst}/applications/${APP_ID}.desktop"
    log "Installing desktop entry → $desk_dst"
    # Ensure Exec is bare command name so PATH works for both prefixes
    local tmpdesk
    tmpdesk="$(mktemp)"
    sed -e "s|^Exec=.*|Exec=${APP_ID}|" \
        -e "s|^Icon=.*|Icon=${APP_ID}|" \
        "$desk_src" > "$tmpdesk"
    if [[ "$USER_INSTALL" -eq 1 ]]; then
      # User install: absolute path is more reliable
      sed -i "s|^Exec=.*|Exec=${bin_dst}|" "$tmpdesk"
    fi
    install_file 644 "$tmpdesk" "$desk_dst"
    rm -f "$tmpdesk"
  fi

  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${share_dst}/applications" 2>/dev/null || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "${share_dst}/icons/hicolor" 2>/dev/null || true
  fi

  # Ensure ~/.local/bin is on PATH hint
  if [[ "$USER_INSTALL" -eq 1 ]]; then
    case ":$PATH:" in
      *":$HOME/.local/bin:"*) ;;
      *)
        warn "~/.local/bin is not on your PATH. Add to ~/.bashrc:"
        echo '  export PATH="$HOME/.local/bin:$PATH"'
        ;;
    esac
  fi

  log "App installed. Launch:  ${APP_ID}   or from the app menu."
}

# ── Main ────────────────────────────────────────────────────────────────────
main() {
  echo ""
  echo " MagicPad Companion — EndeavourOS / Arch installer"
  echo " ${REPO_URL}"
  echo ""

  if [[ "$MODE" == "uninstall" ]]; then
    do_uninstall
    return
  fi

  if [[ "$MODE" == "gestures" ]]; then
    install_gesture_daemon || true
    log "Done (gesture daemon). Check: systemctl --user status ${GESTURES_UNIT}"
    return
  fi

  if [[ "$MODE" == "helpers" ]]; then
    local rule="$RULE_SRC_TREE"
    [[ -f "$rule" ]] || die "Run from a git checkout (missing packaging/linux)."
    install_helpers "$rule"
    if [[ "$INSTALL_GESTURES" -eq 1 ]]; then
      install_gesture_daemon || true
    fi
    log "Done (helpers). Replug the trackpad, then open MagicPad Companion."
    return
  fi

  # Full install
  install_deps

  case "$SOURCE" in
    release) download_latest_deb ;;
    local)   use_local_build ;;
    deb)     extract_deb "$DEB_PATH" ;;
  esac

  install_app_from_extract

  # Prefer udev rule from package resources, fall back to tree
  local rule_from_pkg
  rule_from_pkg="$(find "$EXTRACT_ROOT" -name '99-magic-trackpad.rules' 2>/dev/null | head -1 || true)"
  if [[ -n "$rule_from_pkg" ]]; then
    install_helpers "$rule_from_pkg"
  elif [[ -f "$RULE_SRC_TREE" ]]; then
    install_helpers "$RULE_SRC_TREE"
  else
    warn "No udev rule found to install."
  fi

  if [[ "$INSTALL_GESTURES" -eq 1 ]]; then
    install_gesture_daemon || true
  else
    log "Skipped gesture daemon (--no-gestures)."
  fi

  echo ""
  log "Install complete."
  echo ""
  echo "  Next steps:"
  echo "    1. Log out/in if you were just added to the 'input' group"
  echo "    2. Replug the Magic Trackpad (or re-pair Bluetooth)"
  echo "    3. Start the app:  ${APP_ID}"
  echo "    4. Gestures: systemctl --user status ${GESTURES_UNIT}"
  echo "       (3-finger swipe L/R = workspaces; pinch = zoom)"
  echo ""
  echo "  Docs: ${REPO_URL}/blob/main/docs/linux-install.md"
  echo ""
}

main
