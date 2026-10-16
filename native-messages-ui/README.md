# Native Messages UI

The normal UID11010 QtQuick app reads bounded local history through the lifecycle
actor and receives schema3 appearance through `PhoneAppearanceClient` and the
installed `PhoneAppearance.qml`. Main and CMake select that shared typed path;
no HOME theme-file reader or fallback is selected. Appearance availability does
not grant Messages access, and Messages denial remains distinct from unavailability.

The fixed installed profile supplies six absolute arguments in this order:

| Argument | Meaning |
| --- | --- |
|1|Private UID11010 HOME|
|2|Private UID11010 runtime|
|3|Wayland path; the actor supplies its fresh restricted per-app listener|
|4|Authenticated public appearance socket|
|5|Session actor `app.sock`|
|6|Protected installed appearance-QML directory|

Normal main requires exactly these six arguments and real/effective UID11010.
The desktop entry calls the fixed native `messages-launch` client at
`/run/pokelinux/session/launch-messages.sock`; the request supplies no grant or
executable. The session owner's explicit history mode supplies the existing
Projector; the normal target does not attach an incoming backend or send messages.

```sh
cmake -S native-messages-ui -B messages-build -DCMAKE_BUILD_TYPE=Release
cmake --build messages-build
DESTDIR="$PWD/messages-stage" cmake --install messages-build --prefix /usr
```

CMake installs the normal app, desktop file and shared `PhoneAppearance.qml`.
`MESSAGES_ISOLATED_OBSERVER` defaults off; its capture controls belong only to
explicit disposable qualification builds.

The pinned appearance epoch builds/stages the normal x86 ELF with Qt6.8.2,
Release and warnings as errors. Actual UID11010 production clients, installed
QML and authored root SEQPACKET peers qualify palette updates, unavailability,
Messages denial clearing and literal text delegates without a window. Three
frozen RED invocations share the same missing-QML prerequisite; they are not
three independent defects. The matching GREEN passes those exact cases and four
normal-main argc/UID64/65 controls. Only those pre-QGui refusal paths execute in
normal main; its successful six-argument GUI path remains unexecuted. Prior29
appearance cases are reused by exact shared-source pins, not rerun.

See the [session composition report](../docs/native-messages-session-composition.md)
for the separate current-owner release completion, host components and historical
evidence boundaries.
Normal launcher/current actor/per-app Wayland/real Source/grant→read/revoke/
source-loss GUI composition remains open, as do ARM availability and phone gates.
Any future visual run must qualify the actual [Pixel10 display profile](../docs/native-phone-display-profile.md)
and native capture dimensions before presenting a screenshot.
