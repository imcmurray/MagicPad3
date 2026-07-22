#!/usr/bin/env bash
# MagicPad Companion — EndeavourOS / Arch Linux installer
#
# Installs:
#   • Runtime dependencies (pacman)
#   • App binary + desktop entry + icons (from GitHub release .deb, or local build)
#   • udev rules for Magic Trackpad
#   • Optional input-remapper profile + user systemd unit stub
#
# Usage:
#   ./scripts/install-endeavouros.sh              # full install (latest release)
#   ./scripts/install-endeavouros.sh --helpers    # udev/helpers only
#   ./scripts/install-endeavouros.sh --local      # use local tauri release build
#   ./scripts/install-endeavouros.sh --deb PATH   # install from a specific .deb
#   ./scripts/install-endeavouros.sh --user       # install app under ~/.local (no root for app)
#   ./scripts/install-endeavouros.sh --uninstall  # remove app + udev rule
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
RULE_DST="/etc/udev/rules.d/99-magic-trackpad.rules"

APP_ID="magicpad-companion"
APP_NAME="MagicPad Companion"
LIB_DIR_NAME="MagicPad Companion"

MODE="full"          # full | helpers | uninstall
SOURCE="release"     # release | local | deb
DEB_PATH=""
USER_INSTALL=0
SKIP_DEPS=0
WITH_REMAPPER=0

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
  )
  run_root pacman -S --needed --noconfirm "${pkgs[@]}"

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

  if [[ "$MODE" == "helpers" ]]; then
    local rule="$RULE_SRC_TREE"
    [[ -f "$rule" ]] || die "Run from a git checkout (missing packaging/linux)."
    install_helpers "$rule"
    log "Done (helpers only). Replug the trackpad, then open MagicPad Companion."
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

  echo ""
  log "Install complete."
  echo ""
  echo "  Next steps:"
  echo "    1. Replug the Magic Trackpad (or re-pair Bluetooth)"
  echo "    2. Start the app:  ${APP_ID}"
  echo "    3. Check Status tab for VID 05AC / PID 0324"
  echo ""
  echo "  Docs: ${REPO_URL}/blob/main/docs/linux-install.md"
  echo ""
}

main
