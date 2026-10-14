# Actual native authentication scope integration

These are the authored guest and delegated drivers, synthetic public-ABI
PAM module and wire header executed by the passing two-group scope selection.
The public delegated driver removes one extra terminal blank line; all other
bytes match execution. Provenance binds the executed and public hashes.
Their SHA256 pins are in the [provenance](../../docs/evidence/native-auth-scope-provenance.json).
They run inside the disposable Arch ARM7.1.6 guest; they are not host test commands.
The module is authored fixture code, never a production authentication service.
Its selected `descendant-block` mode forks/setsid/forks and pauses. The other
cleanup-hang mode and reserved typed-success role are unselected preparation.

The launcher supplies `/results/selection.json`, private writable results and
readonly `/workspace` inputs: matching fixed `auth-scope-fixture` and feature-gated
`pam-worker` ELFs, these C/header files, and the installed public-stack pin manifest.
No package/vendor binaries, credentials, raw runtime records or backing image are
included. `guest.py` changes only the disposable guest's dump policy, builds the
module and installs a separate root-owned test service. It launches `delegated.py`
through an explicit delegated transient service. The delegated driver moves only
itself into owner, provisions the workload sibling and optional private limits,
and executes the two fixed native attempts. Existing guest service teardown owns
poweroff. This is not production broker or namespace provisioning.

Both groups require exact real identities/pidfds, observed stopped supervisor T,
responsive local retirement, native recursive kill, actual worker/adopted waits,
global ECHILD, empty leaf, EOF and actual supervisor/parent waits0. Independent
SIGKILL or outer adoption cannot qualify GREEN. All five owned process deaths and
leaf samples are explicit. The second attempt starts only after full first closure
and keeps its fresh original15s deadline. These controlled stops do not simulate
an uninterruptible kernel task or actual system suspend.

The owning launcher supplies the shared readonly backing lock, bounded RAM-only
overlay, no-network/device VM, actual Docker CLI wait and independent fresh exact-
container absence/input closure. Running these drivers alone proves none of those
outer ownership checks. See the [checkpoint and open setup-error boundary](../../docs/native-auth-scope.md).
Original PAM composition, privileged-helper containment, production receipts,
restoration, matched Google kernel and hardware remain separate gates.
