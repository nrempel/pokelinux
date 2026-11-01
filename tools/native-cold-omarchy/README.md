# Executed cold Omarchy/Sounds consumer

These consumers originate in the qualified x86 KVM cold-start slice in
[the report](../../docs/native-cold-omarchy-sounds.md). Later observer changes
carry their own evidence below.
They retain the selected PID1 service identities, actual compositor focus, native
window/capture measurements, independently measured silence and manager shutdown.
The normal shell launcher is observed as pending until the same retained process
execs the selected Quickshell image. Theme completion is a separate PID1 oneshot
exit observation with an empty cgroup.

The cohort also needs its pinned original runtime image, readonly VM backing,
matched compositor/SDK, compiled normal binaries and the unchanged selected
fixture dependencies. These files are reviewable execution snapshots, not yet a
self-contained preview installer. No phone image or Android production backend
is included. The report distinguishes the one permission-withheld case from
authorized playback, other prepared cases, ARM graphics and hardware gates.

The watcher carries each traversal parent's existing pidfd. A missing procfs
children read is skipped only after that exact pidfd signals exit; a live
process with a missing children interface remains an error. Observed handles
remain recorded for the existing closure checks. Disappearance does not prove
an empty subtree or vendor cleanup.

[Host controls](../../docs/evidence/native-cold-proc-disappearance.json) execute
the actual private and maintained watcher functions with real processes,
pidfds and original-parent waits. The Linux 6.8 host returns EOF from a held
children file after exit, so the scheduling hook substitutes a held stat file
for the same task to obtain actual kernel ESRCH. It does not reproduce the ARM
children-file kernel behavior or fabricate an exception. A separate negative
reads a deliberately absent proc entry while the bound task is live and must
propagate actual ENOENT; it does not claim the host lacks CONFIG_PROC_CHILDREN.
Cold25 supplies the genuine ARM children-open ESRCH evidence. These host
controls do not qualify a corrected GUI/ARM rerun.

Run the maintained source control into a fresh output directory; optional
`--private /absolute/frozen/installed_guest.py` also checks a selected installed
consumer with the same control:

```sh
python3 -B tools/native-cold-omarchy/tests/proc-children.py \
  --public /absolute/checkout/tools/native-cold-omarchy/guest.py \
  --output /absolute/fresh-proc-control
```
