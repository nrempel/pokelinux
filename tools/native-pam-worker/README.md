# Actual native PAM guest integration

`guest.py` is the exact authored driver executed by the passing native PAM
component selection, SHA256
`58ab8835e227c219279b85f8f1f4212769925f76edb2edc15c466b824ecb41ae`.
It runs only inside the selected disposable Arch ARM7.1.6 guest. It changes that
guest's temporary account credential, installs the original password service and
sets its native dump-policy prerequisite. It is not a host-side test command.

The driver consumes `/results/selection.json` and three readonly `/workspace`
inputs: the selected default-production `pam-worker`, the original public
`omarchy-lock-password` service and `selected-package-stack-pins.json`. The latter
pins the installed original PAM/helper/configuration files; no package binaries,
credentials, raw runtime records or backing image are included here. The owning
launcher supplies the frozen selection, private writable results and bounded
disposable overlay over the shared readonly backing. Existing guest service
teardown owns poweroff. Original30/80/110+5/120second conversation/guest/QEMU/outer
bounds and independent actual Docker CLI/CID closure belong to that selected
launcher; running this driver alone does not establish host/container ownership.

Two real private-pipe conversations require wrong→Denied and matching→Authenticated,
actual hidden prompts/PAM maps, protected roles before secret supply, ACK, actual
worker wait0, EOF/death and final subreaper ECHILD without adoptions. Exit0 alone
is never authentication. Credentials stay in guest memory/private pipes. The
raw guest result is private evidence; only bounded reviewed provenance is public.
See [checkpoint](../../docs/native-pam-worker.md) and
[proof](../../docs/evidence/native-pam-worker-provenance.json). No generic helper
cgroup cancellation, actor restoration, Google-kernel or physical-phone result
is claimed by this component.
