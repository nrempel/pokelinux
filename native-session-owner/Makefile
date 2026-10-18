.PHONY: install install-normal-session install-native-services
install:
	test -n "$(SESSION_OWNER_BIN)" && test -x "$(SESSION_OWNER_BIN)"
	test -n "$(SOUNDS_LAUNCH_BIN)" && test -x "$(SOUNDS_LAUNCH_BIN)"
	test -n "$(MESSAGES_LAUNCH_BIN)" && test -x "$(MESSAGES_LAUNCH_BIN)"
	test -n "$(SETTINGS_LAUNCH_BIN)" && test -x "$(SETTINGS_LAUNCH_BIN)"
	install -Dm755 -- "$(SESSION_OWNER_BIN)" "$(DESTDIR)/usr/libexec/pokelinux/pokelinux-session-owner"
	install -Dm755 -- "$(SOUNDS_LAUNCH_BIN)" "$(DESTDIR)/usr/libexec/pokelinux/sounds-launch"
	install -Dm755 -- "$(MESSAGES_LAUNCH_BIN)" "$(DESTDIR)/usr/libexec/pokelinux/messages-launch"
	install -Dm755 -- "$(SETTINGS_LAUNCH_BIN)" "$(DESTDIR)/usr/libexec/pokelinux/settings-launch"
	install -Dm644 -- ../native-messages-ui/org.omarchy.Messages.desktop "$(DESTDIR)/usr/share/applications/org.omarchy.Messages.desktop"
	install -Dm644 -- ../native-audio-ui/org.omarchy.Sounds.desktop "$(DESTDIR)/usr/share/applications/org.omarchy.Sounds.desktop"
	install -Dm644 -- ../native-settings-ui/org.omarchy.Settings.desktop "$(DESTDIR)/usr/share/applications/org.omarchy.Settings.desktop"
	install -Dm644 -- packaging/pokelinux-session-owner.service "$(DESTDIR)/usr/lib/systemd/system/pokelinux-session-owner.service"
	install -Dm644 -- packaging/session.example.json "$(DESTDIR)/usr/share/pokelinux/session-owner/session.example.json"
	install -Dm644 -- packaging/apps.example.json "$(DESTDIR)/usr/share/pokelinux/session-owner/apps.example.json"

# Additional explicit deployment input; never activate examples or host services.
install-normal-session:
	test -n "$(DESTDIR)" && test -n "$(SESSION_DEPLOYMENT_ROOT)" && test -n "$(SESSION_DEPLOYMENT_MANIFEST)"
	python3 packaging/install_deployment.py --root "$(SESSION_DEPLOYMENT_ROOT)" --manifest "$(SESSION_DEPLOYMENT_MANIFEST)" --destdir "$(DESTDIR)"

# Explicit native-services bundle, including caller-selected configuration where needed.
# The shared stager rejects a live-root DESTDIR and never activates services.
install-native-services:
	test -n "$(DESTDIR)" && test -n "$(NATIVE_SERVICES_ROOT)" && test -n "$(NATIVE_SERVICES_MANIFEST)"
	python3 packaging/install_deployment.py --root "$(NATIVE_SERVICES_ROOT)" --manifest "$(NATIVE_SERVICES_MANIFEST)" --destdir "$(DESTDIR)"
