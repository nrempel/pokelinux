# Installed owner Stop and cold Source failure

The unit now sends SIGTERM to the Rust owner through KillMode=mixed, without a
second ExecStop client. Rust owns the existing actor Stop and child drain.
Systemd retains its final cgroup kill after Main exit or the unchanged5s stop
limit. [Upstream semantics](https://raw.githubusercontent.com/systemd/systemd/main/man/systemd.kill.xml)
define that signal order; process retirement does not establish vendor cleanup.

Two actual cold native x86 KVM epochs on Harbour select the same optimized owner,
normal upstream compositor,498-file matching SDK,205-file installed bundle,
196 locked Omarchy blobs, Pixel10 profile and original100ms Source lease. The
frozen consumer is byte-identical; only selected unit metadata differs. The
compiled Rust source snapshot retains its historical unit; the operational unit
is separately authenticated metadata, without a claimed Rust rebuild.

| Observation | Current-unit baseline | Selected mixed unit |
| --- | --- | --- |
| Actual PID1 notify state | active/running | active/running |
| Source first accepted sequence | 1 | 1 |
| Sample age at terminal cached expiry |104.932153ms|110.312815ms|
| Actual manager Stop command | exit0 | exit0 |
| Matching owner Main / unit result | exit0 / success | exit0 / success |
| Retained Main dead / exact cgroup empty | yes / yes | yes / yes |
| Request through final exit-status observation |50.428412ms|35.039281ms|
| Full cold shell/app/PCM aggregate | failed | failed |

These are individual VM observations, not phone performance measurements.
Manager Stop is requested while the exact retained Main is live and PID1 reports
notify active/running, after authenticated Source loss. No signal syscall trace,
native IPC acknowledgement or native/vendor cleanup is invented. The consumer's
positive healthy-full-startup/manager-Stop predicate remains false in both epochs.

The baseline shell's retained pidfd dies; the same invocation later records
exit255 and its existing journal reports Wayland connection refusal. The candidate
loses the shell's full original argv during identity capture. In both epochs,
the actor retains the first Source sample and its terminal cached-sample expiry.
This identifies an actual startup failure; it does not identify which compositor
operation caused the gap. In the baseline, Source expiry precedes the shell's
reported startup/refusal, so shell causation must not be inferred.

The observer treats queued jobs as pending and authenticates the notify owner
while activating, without promoting readiness. One held system-bus connection
references all four units before target start and releases them after final
matching invocation/status/cgroup observations. Pre-owner authenticated service
finishes cannot issue a manager Stop that cascades early. No runtime success
predicate, Source producer, lease, startup deadline or retry policy changes.

Each epoch independently closes148 actual direct waits and155 PIPE EOFs, with
one separate QMP socket EOF and eight separate RefUnit/UnrefUnit method returns.
QEMU and container CLI exit0, exact owned container absence on the same daemon,
selected inputs and original closure cutoffs all pass. Both overall functional
results remain failed. [Hash-only provenance](evidence/native-session-manager-stop-provenance.json)
binds these observations; raw private logs and binaries are not published.

The earlier native-IPC-first Stop result recorded Main0 followed by redundant
ExecStop1. These new manager-first results are not relabeled as reproducing that
unit RED. The one-shot Rust signal implementation has its own prior actual
SIGTERM RED→GREEN record. Next identify the cold Source gap using the retained
compositor's existing evidence, then qualify healthy full-shell/Sounds/Stop.
ARM, live combined themes, Messages and every physical Pixel gate remain open.
No physical phone is accessed.

## Later passive startup diagnostics

Two observer-only epochs now retain the activating owner's actual compositor
pidfd and13 completed configuration events, with valid committed ring prefixes.
Configuration completion precedes the first accepted Source sample. One later
real216-byte sequence2 is rejected for a116.863834ms producer sample gap; its
sample-to-validation interval is37.458825ms. This does not identify the blocking
operation or justify another readiness gate.

No instance log was captured. The pinned upstream [configuration default](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp#L629)
disables file logging; missing logs supply no cause. Both aggregates remain
failed. Their respective148/149 direct waits and155/156 PIPE EOFs, separate QMP
EOF, exact container/input closure and original bounds pass. See the separate
[diagnostic provenance](evidence/native-cold-source-gap-provenance.json).
