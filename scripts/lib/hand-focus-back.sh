# Sourced, not run: by scripts/screenshot-scene.sh and
# scripts/check-fits-window.sh, the two scripts that open a real Godot window.
#
# Godot activates itself at launch, and its no-focus window then leaves no
# app in front at all. So whenever Godot is the front app, HIDE it (what
# Cmd-H does). macOS then gives focus back to the app that had it, which is
# the one thing a script cannot reliably name. Handing focus to a named app
# instead was tried: Godot took it back each time. Polled, because the
# activation lands a moment after launch, at a time this script cannot see.
# Nothing here needs the window to be visible: test_screenshot.gd and
# test_hud_plates.gd each draw it by hand.
hand_focus_back() {
  local godot_pid=$1 front
  while kill -0 "$godot_pid" 2>/dev/null; do
    front=$(lsappinfo info -only pid "$(lsappinfo front)" 2>/dev/null \
      | sed -n 's/.*"pid"=\([0-9]*\).*/\1/p')
    if [ "$front" = "$godot_pid" ]; then
      osascript -e "tell application \"System Events\" to set visible of \
        (first process whose unix id is $godot_pid) to false" >/dev/null 2>&1
    fi
    sleep 0.05
  done
}
