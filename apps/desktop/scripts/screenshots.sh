#!/usr/bin/env bash
#
# screenshots.sh — Automated screenshot capture for the PortZero desktop app.
#
# Captures screenshots of each page using macOS `screencapture` and the
# portzero:// deep link scheme to navigate between views.
#
# Prerequisites:
#   - The PortZero desktop app must be built and installed in /Applications
#     (or running via `pnpm tauri dev` with deep links registered)
#   - The PortZero daemon should be running with at least one app registered
#     so the screenshots show real data.
#
# Usage:
#   ./scripts/screenshots.sh                  # default output: ../web/public/screenshots/
#   ./scripts/screenshots.sh /tmp/shots       # custom output directory
#   PORTZERO_APP=my-app ./scripts/screenshots.sh  # use a specific app name
#
set -euo pipefail

# --- Configuration -----------------------------------------------------------

OUTPUT_DIR="${1:-$(cd "$(dirname "$0")/../../web/public/screenshots" && pwd)}"
DELAY="${SCREENSHOT_DELAY:-2}"       # seconds to wait after navigation
APP_NAME="${PORTZERO_APP:-}"         # app name for app-detail screenshot
WINDOW_TITLE="PortZero"
BUNDLE_ID="dev.portzero.dashboard"

# Pages to capture: (filename  deep-link-path)
# For overview we open portzero:// which maps to the root
PAGES=(
  "overview          /"
  "traffic           /traffic"
  "mocks             /mocks"
  "settings          /settings"
)

# --- Helpers -----------------------------------------------------------------

info()  { printf "\033[0;36m→\033[0m %s\n" "$*"; }
ok()    { printf "\033[0;32m✓\033[0m %s\n" "$*"; }
warn()  { printf "\033[0;33m!\033[0m %s\n" "$*"; }
fail()  { printf "\033[0;31m✗\033[0m %s\n" "$*" >&2; exit 1; }

get_window_id() {
  # Get the CGWindowID of the PortZero window using CGWindowListCopyWindowInfo
  osascript -e "
    tell application \"System Events\"
      set wList to every window of (first process whose bundle identifier is \"${BUNDLE_ID}\")
      if (count of wList) > 0 then
        return id of item 1 of wList
      end if
    end tell
  " 2>/dev/null || echo ""
}

activate_window() {
  osascript -e "
    tell application id \"${BUNDLE_ID}\"
      activate
    end tell
  " 2>/dev/null || true
}

navigate() {
  local path="$1"
  # Use the portzero:// deep link to navigate
  open "portzero://${path}"
}

capture() {
  local name="$1"
  local outfile="${OUTPUT_DIR}/${name}.png"

  # Use screencapture in window mode (-l) if we have a window ID,
  # otherwise fall back to interactive window capture (-w).
  local wid
  wid=$(get_window_id)

  if [[ -n "$wid" ]]; then
    screencapture -l "$wid" -o "$outfile"
  else
    # Fallback: capture the frontmost window
    screencapture -w -o "$outfile"
  fi

  if [[ -f "$outfile" ]]; then
    ok "Captured ${name} → $(basename "$outfile")"
  else
    warn "Failed to capture ${name}"
  fi
}

# --- Main --------------------------------------------------------------------

info "PortZero Screenshot Automation"
info "Output directory: ${OUTPUT_DIR}"
echo

mkdir -p "$OUTPUT_DIR"

# Check the app is running
if ! pgrep -f "$BUNDLE_ID" >/dev/null 2>&1; then
  info "PortZero app not running, attempting to launch..."
  open -b "$BUNDLE_ID" 2>/dev/null || open -a "PortZero" 2>/dev/null || \
    fail "Could not launch PortZero. Build it first with: cd apps/desktop && pnpm tauri build"
  sleep 3
fi

activate_window

# Detect an app name if not provided
if [[ -z "$APP_NAME" ]]; then
  # Try to get the first app from the daemon
  APP_NAME=$(curl -s http://localhost:1337/api/apps 2>/dev/null \
    | grep -o '"name":"[^"]*"' \
    | head -1 \
    | sed 's/"name":"//;s/"//' || true)
fi

# Capture each page
for entry in "${PAGES[@]}"; do
  name=$(echo "$entry" | awk '{print $1}')
  path=$(echo "$entry" | awk '{print $2}')

  info "Navigating to ${name}..."
  navigate "$path"
  sleep "$DELAY"
  activate_window
  sleep 0.5
  capture "$name"
done

# Capture app detail if we have an app name
if [[ -n "$APP_NAME" ]]; then
  info "Navigating to app detail (${APP_NAME})..."
  navigate "apps/${APP_NAME}"
  sleep "$DELAY"
  activate_window
  sleep 0.5
  capture "app-detail"
else
  warn "No app running — skipping app-detail screenshot. Set PORTZERO_APP=<name> or start an app first."
fi

echo
ok "Done! Screenshots saved to: ${OUTPUT_DIR}"
ls -1 "${OUTPUT_DIR}"/*.png 2>/dev/null | while read -r f; do
  printf "   %s (%s)\n" "$(basename "$f")" "$(du -h "$f" | awk '{print $1}')"
done
