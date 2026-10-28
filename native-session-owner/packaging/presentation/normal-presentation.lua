-- Load from the selected normal Hyprland configuration before startup.
-- The literal command inherits the compositor's two presentation values.
hl.on("hyprland.start", function()
  hl.exec_cmd("/usr/libexec/pokelinux/presentation-environment publish")
end)
