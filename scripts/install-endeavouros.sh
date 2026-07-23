#!/usr/bin/env bash
# MagicPad Companion — EndeavourOS / Arch Linux installer
#
# Usage:
#   ./scripts/install-endeavouros.sh              # install everything
#   ./scripts/install-endeavouros.sh uninstall    # remove app + helpers
#   ./scripts/install-endeavouros.sh --help
#
# That is all. Defaults:
#   • App from a local release build if present, otherwise latest GitHub .deb
#   • ~/.local if ~/.local/bin exists, otherwise /usr/local (never both)
#   • deps, udev, gesture daemon, checklist — always
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

MODE="install"   # install | uninstall
USER_INSTALL=0

log()  { printf '==> %s\n' "$*"; }
warn() { printf 'WARNING: %s\n' "$*" >&2; }
die()  { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
ok()   { printf '  [ok] %s\n' "$*"; }
fail() { printf '  [!!] %s\n' "$*"; }

usage() {
  cat <<EOF
MagicPad Companion installer (EndeavourOS / Arch)

  ./scripts/install-endeavouros.sh           Install app + udev + gestures
  ./scripts/install-endeavouros.sh uninstall Remove app + udev + gesture service
  ./scripts/install-endeavouros.sh --help    This help

No other flags. Re-run the installer anytime to repair/update.
Docs: ${REPO_URL}/blob/main/docs/linux-install.md
EOF
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    uninstall|--uninstall) MODE="uninstall"; shift ;;
    -h|--help|help) usage ;;
    *)
      die "Unknown option: $1

Just run:
  ./scripts/install-endeavouros.sh
  ./scripts/install-endeavouros.sh uninstall"
      ;;
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

# Exactly one prefix: ~/.local if that bin dir exists, else /usr/local.
resolve_prefix_mode() {
  if [[ -d "$(target_home)/.local/bin" ]]; then
    USER_INSTALL=1
    log "Install prefix: $(target_home)/.local"
  else
    USER_INSTALL=0
    log "Install prefix: /usr/local"
  fi
}

# Account membership in `input` (getent /etc/group + id user) — NOT session groups.
# Session groups lag after usermod until re-login; the daemon uses `sg input` so
# account membership is what matters.
account_in_input_group() {
  local u="${1:-$(target_user)}"
  # getent group input → input:x:GID:user1,user2
  if getent group input 2>/dev/null | awk -F: -v u="$u" '
      {
        n = split($4, m, ",");
        for (i = 1; i <= n; i++) if (m[i] == u) exit 0;
        exit 1
      }'; then
    return 0
  fi
  # id -nG <user> reflects account DB, not this process's supplementary groups
  if id -nG "$u" 2>/dev/null | tr ' ' '\n' | grep -qx input; then
    return 0
  fi
  return 1
}

ensure_input_group() {
  local u
  u="$(target_user)"
  if account_in_input_group "$u"; then
    log "User $u is in the input group (account membership)."
    return 0
  fi
  log "Adding $u to the 'input' group (required to read trackpad events)…"
  run_root usermod -aG input "$u"
  if account_in_input_group "$u"; then
    log "Added $u to input. Daemon uses 'sg input' so a full re-login is usually not required."
    warn "Session groups may still lag until you log out/in; Status UI is fine either way."
  else
    warn "usermod may have failed — check: getent group input"
  fi
}

# Run a command as the desktop user (for systemctl --user, writing ~/.config)
as_user() {
  local u
  u="$(target_user)"
  if [[ "$(id -u)" -eq 0 && "$u" != "root" ]]; then
    local uid runtime home
    uid="$(id -u "$u")"
    home="$(getent passwd "$u" | cut -d: -f6)"
    runtime="/run/user/${uid}"
    if command -v sudo >/dev/null 2>&1; then
      sudo -u "$u" --preserve-env=WAYLAND_DISPLAY \
        env "HOME=$home" \
            "XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-$runtime}" \
            "XDG_CONFIG_HOME=${home}/.config" \
            "USER=$u" \
            "LOGNAME=$u" \
            "$@"
    else
      runuser -u "$u" -- env "HOME=$home" \
        "XDG_RUNTIME_DIR=${runtime}" "USER=$u" "$@"
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

# Single install root (never both ~/.local and /usr/local)
prefix_bin() {
  if [[ "$USER_INSTALL" -eq 1 ]]; then
    echo "$(target_home)/.local/bin"
  else
    echo "/usr/local/bin"
  fi
}

prefix_share() {
  if [[ "$USER_INSTALL" -eq 1 ]]; then
    echo "$(target_home)/.local/share"
  else
    echo "/usr/local/share"
  fi
}

prefix_lib() {
  if [[ "$USER_INSTALL" -eq 1 ]]; then
    echo "$(target_home)/.local/lib"
  else
    echo "/usr/local/lib"
  fi
}

# Paths that are *not* the chosen prefix — remove leftovers so only one copy remains
other_prefix_paths() {
  local home
  home="$(target_home)"
  if [[ "$USER_INSTALL" -eq 1 ]]; then
    # Installing to ~/.local → purge system copies
    printf '%s\n' \
      "/usr/local/bin/${APP_ID}" \
      "/usr/bin/${APP_ID}" \
      "/usr/local/lib/${LIB_DIR_NAME}" \
      "/usr/local/share/applications/${APP_ID}.desktop" \
      "/usr/local/share/applications/${APP_NAME}.desktop"
  else
    # Installing to /usr/local → purge user copies
    printf '%s\n' \
      "${home}/.local/bin/${APP_ID}" \
      "${home}/.local/lib/${LIB_DIR_NAME}" \
      "${home}/.local/share/applications/${APP_ID}.desktop" \
      "${home}/.local/share/applications/${APP_NAME}.desktop"
  fi
}

all_known_bins() {
  # Every place we might ever have put the binary (uninstall / leftover checks)
  local home
  home="$(target_home)"
  printf '%s\n' \
    "${home}/.local/bin/${APP_ID}" \
    "/usr/local/bin/${APP_ID}" \
    "/usr/bin/${APP_ID}"
}

# Soft remove — never abort install if leftover cleanup needs root we don't have
try_remove_path() {
  local p="$1"
  if [[ ! -e "$p" && ! -L "$p" ]]; then
    return 0
  fi
  log "Removing other-location leftover: $p"
  if [[ "$p" == /usr/* ]] || [[ "$p" == /etc/* ]]; then
    if [[ "$(id -u)" -eq 0 ]]; then
      rm -rf "$p" && return 0
    elif command -v sudo >/dev/null 2>&1; then
      # Interactive sudo when we have a TTY; non-interactive (-n) otherwise
      if [[ -t 0 ]] || [[ -t 2 ]]; then
        if sudo rm -rf "$p" 2>/dev/null; then
          return 0
        fi
      elif sudo -n rm -rf "$p" 2>/dev/null; then
        return 0
      fi
      warn "Could not remove $p (need: sudo rm -rf '$p')"
      return 1
    else
      warn "Could not remove $p (need root)"
      return 1
    fi
  else
    rm -rf "$p" 2>/dev/null || warn "Could not remove $p"
  fi
}

# After installing to the chosen prefix, delete any copy in the other tree.
remove_other_install() {
  local p home s
  while IFS= read -r p; do
    [[ -n "$p" ]] || continue
    try_remove_path "$p" || true
  done < <(other_prefix_paths)
  home="$(target_home)"
  if [[ "$USER_INSTALL" -eq 1 ]]; then
    for s in 32x32 128x128 256x256 256x256@2; do
      try_remove_path "/usr/local/share/icons/hicolor/${s}/apps/${APP_ID}.png" || true
    done
  else
    for s in 32x32 128x128 256x256 256x256@2; do
      try_remove_path "${home}/.local/share/icons/hicolor/${s}/apps/${APP_ID}.png" || true
    done
  fi
}

sg_or_direct_exec() {
  # Build an Exec= line that runs the binary under `sg input` when possible
  local exe="$1"
  local args="${2:---gestures}"
  if command -v sg >/dev/null 2>&1; then
    local sg_bin
    sg_bin="$(command -v sg)"
    # Prefer absolute sg for systemd
    [[ -x /usr/bin/sg ]] && sg_bin="/usr/bin/sg"
    echo "${sg_bin} input -c '${exe} ${args}'"
  else
    echo "${exe} ${args}"
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
  # Also kill any leftover foreground / autostart instances
  as_user pkill -f "${APP_ID} --gestures" 2>/dev/null || true

  local home cfg
  home="$(target_home)"
  cfg="${home}/.config"
  remove_path "${cfg}/systemd/user/${GESTURES_UNIT}"
  remove_path "${cfg}/autostart/magicpad-gestures.desktop"

  # Remove every known binary install path (avoid stale dual-install leftovers)
  local p
  while IFS= read -r p; do
    [[ -n "$p" ]] || continue
    remove_path "$p"
  done < <(all_known_bins | sort -u)

  # Both system and user resource trees
  for base in \
    "/usr/local" \
    "/usr" \
    "${home}/.local"
  do
    remove_path "${base}/lib/${LIB_DIR_NAME}"
    remove_path "${base}/share/applications/${APP_ID}.desktop"
    remove_path "${base}/share/applications/${APP_NAME}.desktop"
    for s in 32x32 128x128 256x256 256x256@2; do
      remove_path "${base}/share/icons/hicolor/${s}/apps/${APP_ID}.png"
    done
  done

  if [[ -f "$RULE_DST" ]]; then
    run_root rm -f "$RULE_DST"
    run_root udevadm control --reload-rules || true
    run_root udevadm trigger || true
    log "Removed udev rule $RULE_DST"
  fi
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "/usr/local/share/applications" 2>/dev/null || true
    update-desktop-database "${home}/.local/share/applications" 2>/dev/null || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "/usr/local/share/icons/hicolor" 2>/dev/null || true
    gtk-update-icon-cache -f -t "${home}/.local/share/icons/hicolor" 2>/dev/null || true
  fi
  log "Uninstall complete. Config under ${home}/.config/magicpad-companion was left in place."
}

# ── Gesture daemon (user systemd) ───────────────────────────────────────────
resolve_app_binary() {
  # Single preferred path first, then the other known locations (pre-cleanup leftovers)
  local candidates=(
    "$(prefix_bin)/${APP_ID}"
    "$(target_home)/.local/bin/${APP_ID}"
    "/usr/local/bin/${APP_ID}"
    "/usr/bin/${APP_ID}"
    "$ROOT/src-tauri/target/release/${APP_ID}"
  )
  local c seen=""
  for c in "${candidates[@]}"; do
    [[ -n "$c" ]] || continue
    case " $seen " in *" $c "*) continue ;; esac
    seen+=" $c"
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
  local home cfg json
  home="$(target_home)"
  cfg="${home}/.config/magicpad-companion"
  json="${cfg}/gestures.json"
  if [[ -f "$json" ]]; then
    log "Keeping existing gestures config: $json"
    return 0
  fi
  as_user mkdir -p "$cfg"
  # Matches app defaults: 3-tap screenshot, 4-tap unbound (set Custom e.g. flameshot),
  # pinch zoom, 4-swipe browser back/forward (v0.3.1–0.3.3)
  local tmp
  tmp="$(mktemp)"
  cat > "$tmp" <<'JSON'
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
    { "trigger": "four_finger_tap", "action": "none", "custom": null, "available": true },
    { "trigger": "pinch_in", "action": "zoom_out", "custom": null, "available": true },
    { "trigger": "pinch_out", "action": "zoom_in", "custom": null, "available": true }
  ],
  "backend": "libinput-daemon"
}
JSON
  if [[ "$(id -u)" -eq 0 ]]; then
    install -o "$(target_user)" -g "$(target_user)" -m 644 "$tmp" "$json"
  else
    install -m 644 "$tmp" "$json"
  fi
  rm -f "$tmp"
  log "Seeded default gestures → $json"
}

install_gesture_daemon() {
  log "Configuring multi-finger gesture daemon…"

  # Deps (only if missing — avoids sudo password prompt when already installed)
  if command -v pacman >/dev/null 2>&1; then
    local need_pkgs=()
    command -v libinput >/dev/null 2>&1 || need_pkgs+=(libinput-tools)
    command -v wtype >/dev/null 2>&1 || command -v xdotool >/dev/null 2>&1 || need_pkgs+=(wtype)
    if [[ ${#need_pkgs[@]} -gt 0 ]]; then
      run_root pacman -S --needed --noconfirm "${need_pkgs[@]}" 2>/dev/null \
        || warn "Could not install ${need_pkgs[*]} via pacman — install them manually."
    fi
  fi

  local u home uid runtime wayland exe unit_dir unit autostart_dir exec_line
  u="$(target_user)"
  home="$(target_home)"
  uid="$(target_uid)"
  # Prefer live session env when installing from an active graphical session
  if [[ -n "${XDG_RUNTIME_DIR:-}" && -d "${XDG_RUNTIME_DIR}" ]]; then
    runtime="$XDG_RUNTIME_DIR"
  else
    runtime="/run/user/${uid}"
  fi
  wayland="${WAYLAND_DISPLAY:-wayland-0}"

  if ! exe="$(resolve_app_binary)"; then
    warn "magicpad-companion binary not found — re-run: ./scripts/install-endeavouros.sh"
    return 1
  fi
  # Prefer absolute path for systemd
  exe="$(readlink -f "$exe" 2>/dev/null || echo "$exe")"
  log "Gesture daemon binary: $exe"

  if ! command -v libinput >/dev/null 2>&1; then
    warn "libinput CLI missing (package: libinput-tools). Daemon cannot start yet."
  fi
  if ! command -v wtype >/dev/null 2>&1 && ! command -v xdotool >/dev/null 2>&1; then
    warn "wtype missing (package: wtype). Daemon cannot inject keys yet."
  fi

  ensure_input_group
  seed_default_gestures_json

  unit_dir="${home}/.config/systemd/user"
  as_user mkdir -p "$unit_dir"
  unit="${unit_dir}/${GESTURES_UNIT}"

  # Use `sg input` so the daemon can open /dev/input even when the user
  # systemd session was started before usermod -aG input (no full re-login).
  exec_line="$(sg_or_direct_exec "$exe" "--gestures")"

  local tmpunit
  tmpunit="$(mktemp)"
  cat > "$tmpunit" <<EOF
[Unit]
Description=MagicPad Companion multi-finger gesture daemon
Documentation=${REPO_URL}
PartOf=graphical-session.target
After=graphical-session.target

[Service]
Type=simple
ExecStart=${exec_line}
Restart=on-failure
RestartSec=2
Environment=RUST_LOG=info
Environment=WAYLAND_DISPLAY=${wayland}
Environment=XDG_RUNTIME_DIR=${runtime}

[Install]
WantedBy=graphical-session.target
WantedBy=default.target
EOF
  if [[ "$(id -u)" -eq 0 ]]; then
    install -o "$u" -g "$u" -m 644 "$tmpunit" "$unit"
  else
    install -m 644 "$tmpunit" "$unit"
  fi
  rm -f "$tmpunit"

  autostart_dir="${home}/.config/autostart"
  as_user mkdir -p "$autostart_dir"
  local tmpdesk exec_autostart
  # Autostart also under sg input (same permission model as the unit)
  exec_autostart="$(sg_or_direct_exec "$exe" "--gestures")"
  tmpdesk="$(mktemp)"
  cat > "$tmpdesk" <<EOF
[Desktop Entry]
Type=Application
Name=MagicPad Gestures
Comment=Multi-finger trackpad gestures for MagicPad Companion
Exec=${exec_autostart}
X-GNOME-Autostart-enabled=true
Hidden=false
NoDisplay=true
EOF
  if [[ "$(id -u)" -eq 0 ]]; then
    install -o "$u" -g "$u" -m 644 "$tmpdesk" "${autostart_dir}/magicpad-gestures.desktop"
  else
    install -m 644 "$tmpdesk" "${autostart_dir}/magicpad-gestures.desktop"
  fi
  rm -f "$tmpdesk"

  log "Installed user unit → $unit"
  log "  ExecStart=${exec_line}"
  log "Installed autostart → ${autostart_dir}/magicpad-gestures.desktop"

  # Restart cleanly so an old binary / old unit without sg is replaced
  as_user systemctl --user stop "$GESTURES_UNIT" 2>/dev/null || true
  as_user pkill -f "${APP_ID} --gestures" 2>/dev/null || true

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
    warn "Daemon not active yet. Try: systemctl --user restart ${GESTURES_UNIT}"
    warn "If /dev/input is still denied: ensure getent group input lists $(target_user)"
  fi
}

# ── Dependencies ────────────────────────────────────────────────────────────
# True when the runtime stack we need is already present (skip sudo pacman).
deps_already_ok() {
  command -v pacman >/dev/null 2>&1 || return 1
  # Critical runtime + gesture tools
  pacman -Q webkit2gtk-4.1 gtk3 libappindicator-gtk3 librsvg xdg-utils \
    binutils tar curl libinput-tools wtype &>/dev/null
}

install_deps() {
  is_arch_family || warn "Not detected as Arch/EndeavourOS — pacman steps may fail."

  if deps_already_ok; then
    log "Runtime packages already installed — skipping pacman."
  else
    need_cmd pacman
    log "Installing runtime packages (pacman)…"
    local pkgs=(
      webkit2gtk-4.1
      gtk3
      libappindicator-gtk3
      librsvg
      xdg-utils
      binutils
      tar
      curl
      polkit
      libinput-tools
      wtype
    )
    # Soft-fail: user can re-run with sudo if packages are missing
    if ! run_root pacman -S --needed --noconfirm "${pkgs[@]}"; then
      warn "pacman failed (need sudo?). Re-run in a terminal if packages are missing."
      if ! command -v libinput >/dev/null 2>&1 || ! command -v wtype >/dev/null 2>&1; then
        die "Missing libinput-tools or wtype — install packages then re-run."
      fi
    fi
  fi

  # Gesture daemon needs /dev/input access (account membership; sg handles session lag)
  ensure_input_group
}

# Prefer local release build when present; otherwise download latest GitHub .deb.
obtain_package() {
  local bin="$ROOT/src-tauri/target/release/${APP_ID}"
  local deb_dir="$ROOT/src-tauri/target/release/bundle/deb"
  if [[ -d "$deb_dir" ]] && find "$deb_dir" -maxdepth 1 -type f -name '*.deb' 2>/dev/null | grep -q .; then
    use_local_build
    return
  fi
  if [[ -x "$bin" ]]; then
    use_local_build
    return
  fi
  download_latest_deb
}

# ── Helpers (udev, remapper profile, unit) ──────────────────────────────────
install_helpers() {
  local rule_src="$1"
  local home
  home="$(target_home)"

  [[ -f "$rule_src" ]] || die "udev rule missing: $rule_src"
  if [[ -f "$RULE_DST" ]] && cmp -s "$rule_src" "$RULE_DST" 2>/dev/null; then
    log "udev rule already up to date → $RULE_DST"
  else
    log "Installing udev rule → $RULE_DST"
    if run_root install -m 644 "$rule_src" "$RULE_DST" \
      && run_root udevadm control --reload-rules \
      && run_root udevadm trigger; then
      log "udev rules installed."
    elif [[ -f "$RULE_DST" ]]; then
      warn "Could not update udev rule (need sudo?) — existing rule left in place."
    else
      warn "Could not install udev rule (need sudo?). Re-run in a terminal with sudo access."
    fi
  fi

  ensure_input_group

  local profile_src="$PROFILE_SRC_TREE"
  if [[ -f "$profile_src" ]]; then
    local dest_dir="${home}/.config/input-remapper-2/presets/Magic Trackpad"
    local staged="${home}/.config/magicpad-companion/input-remapper"
    as_user mkdir -p "$dest_dir" "$staged"
    if [[ "$(id -u)" -eq 0 ]]; then
      install -o "$(target_user)" -g "$(target_user)" -m 644 "$profile_src" "$dest_dir/MagicPad.json"
      install -o "$(target_user)" -g "$(target_user)" -m 644 "$profile_src" "$staged/MagicPad.json"
    else
      install -m 644 "$profile_src" "$dest_dir/MagicPad.json"
      install -m 644 "$profile_src" "$staged/MagicPad.json"
    fi
    log "Staged input-remapper profile → $dest_dir/MagicPad.json"
  fi

  if [[ -f "$UNIT_SRC_TREE" ]]; then
    local unit_dir="${home}/.config/systemd/user"
    as_user mkdir -p "$unit_dir"
    if [[ "$(id -u)" -eq 0 ]]; then
      install -o "$(target_user)" -g "$(target_user)" -m 644 "$UNIT_SRC_TREE" \
        "$unit_dir/magicpad-companion.service"
    else
      install -m 644 "$UNIT_SRC_TREE" "$unit_dir/magicpad-companion.service"
    fi
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

  [[ -n "$url" ]] || die "No amd64 .deb on latest GitHub release. Build locally: npm run tauri -- build --bundles deb"
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

  die "No local build found under src-tauri/target/release/."
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

  # Ensure destination dirs exist (user tree needs no root)
  if [[ "$USER_INSTALL" -eq 1 ]]; then
    as_user mkdir -p "$(prefix_bin)" "$(prefix_lib)" \
      "$(prefix_share)/applications" "$(prefix_share)/icons/hicolor"
  fi

  log "Installing binary → $bin_dst"
  install_file 755 "$bin_src" "$bin_dst"

  # Never leave a second copy on PATH (the whole dual-install bug class)
  remove_other_install

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
    local lbin
    lbin="$(target_home)/.local/bin"
    case ":$PATH:" in
      *":${lbin}:"*) ;;
      *)
        warn "~/.local/bin is not on your PATH. Add to ~/.bashrc:"
        echo '  export PATH="$HOME/.local/bin:$PATH"'
        ;;
    esac
  fi

  if command -v "$APP_ID" >/dev/null 2>&1; then
    log "PATH resolves ${APP_ID} → $(command -v "$APP_ID")"
  fi

  log "App installed. Launch:  ${APP_ID}   or from the app menu."
}

# ── Post-install verify ─────────────────────────────────────────────────────
do_verify() {
  local u home issues=0
  u="$(target_user)"
  home="$(target_home)"

  echo ""
  log "MagicPad Companion — install checklist"
  echo ""

  # Binary — exactly one install location expected
  local bin="" preferred
  preferred="$(prefix_bin)/${APP_ID}"
  if [[ "$USER_INSTALL" -eq 1 ]]; then
    ok "prefix: $(target_home)/.local (user)"
  else
    ok "prefix: /usr/local (system)"
  fi
  if bin="$(resolve_app_binary)"; then
    ok "binary: $bin"
    if [[ "$bin" != "$preferred" && -x "$preferred" ]]; then
      fail "expected $preferred but found $bin"
      issues=$((issues + 1))
    fi
    local p extras=0
    while IFS= read -r p; do
      [[ -n "$p" && -e "$p" ]] || continue
      if [[ "$p" != "$bin" ]]; then
        fail "extra copy at $p — re-run: ./scripts/install-endeavouros.sh"
        extras=$((extras + 1))
        issues=$((issues + 1))
      fi
    done < <(all_known_bins | sort -u)
    if [[ "$extras" -eq 0 ]]; then
      ok "single install path only"
    fi
    if command -v "$APP_ID" >/dev/null 2>&1; then
      ok "PATH → $(command -v "$APP_ID")"
    else
      fail "not on PATH — open a new shell or add install dir to PATH"
      issues=$((issues + 1))
    fi
  else
    fail "magicpad-companion binary not found"
    issues=$((issues + 1))
  fi

  # Packages
  if command -v libinput >/dev/null 2>&1; then
    ok "libinput CLI (libinput-tools)"
  else
    fail "libinput missing — sudo pacman -S libinput-tools"
    issues=$((issues + 1))
  fi
  if command -v wtype >/dev/null 2>&1; then
    ok "wtype (key injection)"
  elif command -v xdotool >/dev/null 2>&1; then
    ok "xdotool (fallback key injection)"
  else
    fail "wtype missing — sudo pacman -S wtype"
    issues=$((issues + 1))
  fi
  if command -v sg >/dev/null 2>&1; then
    ok "sg (shadow-utils) — daemon runs under input group without re-login"
  else
    fail "sg missing — install shadow package"
    issues=$((issues + 1))
  fi

  # input group (account DB, not session)
  if account_in_input_group "$u"; then
    ok "user $u in input group (getent/id account)"
    if ! id -nG 2>/dev/null | tr ' ' '\n' | grep -qx input; then
      printf '       note: this shell session lacks input; daemon uses sg input so OK\n'
    fi
  else
    fail "user $u not in input group — sudo usermod -aG input $u"
    issues=$((issues + 1))
  fi

  # udev
  if [[ -f "$RULE_DST" ]]; then
    ok "udev rule $RULE_DST"
  else
    fail "udev rule missing — re-run installer"
    issues=$((issues + 1))
  fi

  # gestures config
  if [[ -f "${home}/.config/magicpad-companion/gestures.json" ]]; then
    ok "gestures.json present"
  else
    fail "gestures.json missing — re-run installer"
    issues=$((issues + 1))
  fi

  # unit + ExecStart has sg
  local unit="${home}/.config/systemd/user/${GESTURES_UNIT}"
  if [[ -f "$unit" ]]; then
    ok "systemd user unit $unit"
    if grep -q 'sg input' "$unit"; then
      ok "unit uses sg input (session-group lag safe)"
    else
      fail "unit missing sg input — re-run installer"
      issues=$((issues + 1))
    fi
    if grep -q -- '--gestures' "$unit"; then
      ok "unit ExecStart runs --gestures"
    fi
  else
    fail "gesture unit missing — re-run installer"
    issues=$((issues + 1))
  fi

  # autostart
  local auto="${home}/.config/autostart/magicpad-gestures.desktop"
  if [[ -f "$auto" ]]; then
    ok "XDG autostart entry"
    if grep -q 'sg input' "$auto"; then
      ok "autostart uses sg input"
    else
      fail "autostart without sg input — re-run installer"
      issues=$((issues + 1))
    fi
  else
    fail "autostart missing — re-run installer"
    issues=$((issues + 1))
  fi

  # daemon running
  if as_user systemctl --user is-active --quiet "$GESTURES_UNIT" 2>/dev/null; then
    ok "gesture daemon active (${GESTURES_UNIT})"
  else
    fail "gesture daemon not active — systemctl --user status ${GESTURES_UNIT}"
    issues=$((issues + 1))
  fi

  # trackpad present (best-effort)
  if command -v libinput >/dev/null 2>&1; then
    if libinput list-devices 2>/dev/null | grep -qi trackpad; then
      ok "libinput sees a trackpad"
    else
      printf '  [--] no trackpad in libinput list-devices (replug / re-pair BT?)\n'
    fi
  fi

  echo ""
  if [[ "$issues" -eq 0 ]]; then
    log "All checks passed."
  else
    warn "${issues} issue(s) found — re-run: ./scripts/install-endeavouros.sh"
  fi
  echo ""
  return "$issues"
}

print_next_steps() {
  echo ""
  log "Install complete."
  echo ""
  echo "  Next:"
  echo "    1. Replug the Magic Trackpad (or re-pair Bluetooth)"
  echo "    2. Start the app:  ${APP_ID}"
  echo ""
  echo "  Docs: ${REPO_URL}/blob/main/docs/linux-install.md"
  echo "  What's new: ${REPO_URL}/blob/main/docs/whats-new.md"
  echo ""
}

# ── Main ────────────────────────────────────────────────────────────────────
main() {
  echo ""
  echo " MagicPad Companion — EndeavourOS / Arch installer"
  echo " ${REPO_URL}"
  echo ""

  resolve_prefix_mode

  if [[ "$MODE" == "uninstall" ]]; then
    do_uninstall
    return
  fi

  # Always: deps → app → udev → gestures → checklist
  install_deps
  obtain_package
  install_app_from_extract

  local rule_from_pkg
  rule_from_pkg="$(find "$EXTRACT_ROOT" -name '99-magic-trackpad.rules' 2>/dev/null | head -1 || true)"
  if [[ -n "$rule_from_pkg" ]]; then
    install_helpers "$rule_from_pkg"
  elif [[ -f "$RULE_SRC_TREE" ]]; then
    install_helpers "$RULE_SRC_TREE"
  else
    warn "No udev rule found to install."
  fi

  install_gesture_daemon || true

  # If the GUI is already running, it keeps the old binary mapped — remind user
  if pgrep -x magicpad-compan >/dev/null 2>&1 || pgrep -x magicpad-companion >/dev/null 2>&1; then
    warn "MagicPad is still running — quit and reopen it to load the new UI."
  fi

  print_next_steps
  do_verify || true
}

main
