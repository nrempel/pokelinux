# Actual containment test lane

The Root-owned baseline `original-direct-app-red-1` executed before native-display
existed. A UID11010 app with the original direct Wayland ACL saw session-lock,
security-context-manager, screencopy, virtual-keyboard and layer-shell globals.
That desired absence assertion failed; the original ARM guest kernel executed
and all10 tracked children reaped. Later baseline replays are regression evidence.

The GREEN fixture never grants the app access to the original runtime or socket.
It connects through the native Root-owned protected group11010 listener and checks
actual allowed registry advertisement/binds and absence of privileged globals.
It collects the real original global IDs through a separately labeled trusted
UID1000 connection, then sends hostile bind requests as UID11010. Each requested
refusal must be an actual `wl_display.error`, not merely missing advertisement.

A separate accepted client stays connected after the creator transport has been
dropped and after the native owner drops its close-pipe writer. Actual refused
future connects establish listener closure; a fresh registry on the existing
connection remains filtered and a privileged bind still produces an original
server error. Local owner status/exit is not remote cleanup acknowledgement.

The optional normal Qt check uses the unchanged production ELF and six standard
arguments through the filtered listener. The Messages data endpoint is absent;
this does not qualify its actor, body, lifecycle grant, session policy or PAM.
Original compositor probe and actual focused SCM_RIGHTS handle, if successful,
qualify direct socket PID/UID/GID/pidfd identity rather than a client proxy.
Failure leaves an explicit Qt gate while the stronger registry/lifetime checks
remain independently assessed.

Every direct test child is tracked and reaped. The existing immutable production
VM backing remains read-only with snapshot=on and no NIC; no new clone, physical
phone, unrelated workload, credential or proprietary artifact is accessed.

The SDK read/dispatch drain can exceed the driver caller's outer budget. These
checks execute only the owned isolated protocol child; they do not qualify an
actor-safe poll or production supervision. Native Root memory/FD limits, responsive
control and actual bounded kill/reap ownership remain a separate gate.

`original-context-green-1` reached actual native ready, direct-path EACCES and
allowed binds plus filtered advertisement. Its hostile probe sent bind and sync
in separate writes: the original server closed before the second write, so the
probe raised BrokenPipe before reading the queued error. That test-driver failure
is preserved and all children reaped. The next probe batches bind and sync in one
bounded write; it still requires an actual `wl_display.error` and rejects EOF-only
or incomplete observations. No native library or original compositor change was
made for this probe correction.

`original-context-green-2` passed the actual registry/bind/lifetime assertions. Its
three forged original privileged globals returned real display errors; future
connections were refused with ECONNREFUSED after stop while the accepted client
remained filtered and rejected another privileged bind. The normal Qt check was
explicitly unqualified because this Arch image lacks `/usr/libexec`; the next
fixture creates its complete private install path. All children were reaped.

Historical `original-context-green-3` passed all 15 observation groups on the actual
original ARM kernel and reaped all 17 direct children. It used the unchanged
`original-context-build-3` helper. The normal production Qt ELF
`4aa50f980eab1100637d582a2752090f7545e1eb30630888753dfb9a7d2512c8`
was mapped as a native Messages window through the protected listener. The
original compositor reported its actual PID359, UID/GID11010 and a live owned
peer pidfd. The original source supplied one SCM_RIGHTS live pidfd for that exact
FocusSome metadata; the separately launcher-owned Qt pidfd was also live. After
context stop, the same Qt child remained alive and listed by the compositor.
This proves the direct connection identity boundary, without claiming Messages
actor/body/session authorization, production PAM containment or safe SDK polling.

The denylist advertisement check covers session-lock, security-context-manager,
screencopy, virtual-keyboard and layer-shell. Hostile actual original global ID
binds cover the first three; after closure the retained accepted probe again
forges session-lock and receives an actual error. These are 15 observation groups,
not 15 independent production security policies. The immutable run labels retain
all frozen sources, selections, results and cleanup records.

Selected `original-context-green-4` replays the unchanged guest/probe against
`original-context-build-4`, whose native Context additionally requires prepared
listening socket domain UNIX. The helper ELF SHA is
`b3e64ef7a8d02fe99a85caaa15173b01f70cb0b9d508a472cdba435c4ed57057`.
All 15 actual observation groups passed; all 17 distinct direct children reaped.
The unchanged normal production Qt ELF was native and mapped with PID362,
UID/GID11010; original socket-peer credentials and its live SCM_RIGHTS pidfd
matched the separately owned Qt child handle. The same Qt child stayed alive and
listed after context closure. The prior build3/run3 epoch and its original scope
remain preserved. Neither replay qualifies Messages authorization, PAM, actor-safe
SDK polling or the native production supervisor.

The next desired supervisor test remains separate and awaits Root's exact CLI,
control response and resource-bound contract. Its pre-implementation baseline
must hold the current protocol child with SIGSTOP and observe the failed desired
status/stop response, then explicitly kill and reap that actual child. The new
native parent must remain responsive while its SDK child is held, retain kill
ownership until actual reap, reject late Ready against the original absolute
five-second initialization deadline, deny on original source death, and enforce
explicit child memory/FD limits. This paragraph is test intent, not executed
RED/GREEN evidence or authorization to change the parent core.
