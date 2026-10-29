-- Append to the selected normal Hyprland configuration before startup.
-- Keep the focused phone app in the work area, preserving shell reservations.
hl.config({
  misc = {
    on_focus_under_fullscreen = 1,
    exit_window_retains_fullscreen = true,
  },
})

hl.window_rule({
  name = "phone-app-available-area",
  match = { class = "^org[.]omarchy[.](Sounds|Permissions)$" },
  fullscreen_state = "1 1",
})
