# Omarchy Phone integration

Omarchy Phone is a native GTK/Wayland application, packaged for pacman. The desktop application talks to the paired Android bridge. Android Telecom and the vendor cellular services remain behind that boundary. The application is not a Hyprland plugin and does not require loading code into the compositor.

Install the local package in the target Arch environment:

```sh
sudo pacman -U omarchy-phone-0.1.0-1-any.pkg.tar
```

Open **Omarchy Phone** from the application launcher. Choose the `client.json` created by desktop enrollment. Keep its client private key private. The desktop entry also supports normal launcher shortcuts. Repeated launcher activation presents the existing window through the session D-Bus.

The app reads Omarchy's active `colors.toml` palette and reloads it within two seconds. It accepts bounded TOML with hexadecimal colors only. It uses its default palette if the file is absent or invalid. Both current state-directory and older config-directory theme paths are supported. It does not modify Omarchy's theme files.

Optional startup on Omarchy quattro: add this line to the user's existing `~/.config/hypr/autostart.lua`:

```lua
o.launch_on_start("omarchy-phone")
```

This opens the application window at login. Closing the window stops the desktop client; background incoming-call notifications and a desktop ringtone service are not implemented. Do not replace the user's autostart file. Earlier Omarchy versions can use different startup configuration; use that installed version's documented mechanism.

The working package supports incoming-call observations, Answer, Decline and End call, plus a number field and optional touch keypad for outgoing requests. Enable Desktop outgoing calls separately in Android Bridge settings. Outgoing calls require the default Android calling account and a fresh authorization offer. Only 3 to 32 digits with an optional leading plus are accepted; service codes and DTMF are separate capabilities. Contacts, call history, mute and audio-device selection remain separate work. The new native outgoing interface is undergoing simulator validation; the previous verified delivery covered incoming controls. An accepted bridge command is not proof of call completion: the display waits for a new Android observation.

The integration test passed for the delivered package. It installs the package with pacman in an ARM64 Arch/Hyprland guest, launches the installed desktop entry, changes the palette and uses virtual touchscreen input to control Android emulator calls. This tests the application interfaces; it is not a full Omarchy installation or a Pixel hardware test.

Sources: [Omarchy theming](https://github.com/omacom/omarchy/blob/quattro/docs/theming.md), [Omarchy dotfiles and startup](https://github.com/omacom/omarchy/blob/quattro/manual/31-dotfiles.md), [GTK application lifecycle](https://docs.gtk.org/gtk3/class.Application.html).
