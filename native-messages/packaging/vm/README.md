# Explicit Messages VM history owner

Select the production `pokelinux-messages.service` and normal feature-empty
`messages-service`/`messages-stop` through the finite authenticated
`install-native-services` manifest. The daemon takes exactly three absolute
arguments: private runtime, raw receipts directory and Inbox directory. The unit
creates root:root0700 runtime/state directories. It owns the sole Projector and
has no app profile, lifecycle grant store or standalone app endpoint. No Binder
dependency or receiving selector is installed.

The graphical session independently selects
`{"mode":"owner","socket":"/run/pokelinux/messages/read-provider.sock"}`,
its fixed UID11010 app profile and normal shared appearance QML. Start the daemon
independently before that session; selection adds no service start, retry or
permission grant. Only the daemon knows database paths. Its private root peer
returns bounded authenticated copied history; the session actor checks current
app permission, principal, Source and the original deadline before publication.
Session cancellation/exit closes local reads without stopping background history.

The separate `messages-seed-fixture` is built only with `isolated-fixture`, kept
under an explicit fixture path and invoked once during disposable image
preparation with `NATIVE_MESSAGES_ISOLATED=1`. It precommits two authored
unqualified raw GSM occurrences. The production service is never built with that
feature. Require the Inbox database to remain absent and the Messages unit
stopped before retaining the prepared image.

The current installed-service probe uses private State/Read and normal typed Stop.
Its current three-argument successor has not been run through PID1 or ARM. The
earlier two cold boots qualify the historical six-argument service and its pinned
probe only; see [the historical result](../../../docs/native-installed-messages-history.md).
Host daemon/Remote components do not establish installed boot, app/window,
incoming SMS, power-loss recovery or physical hardware behavior.
