# Normal Main first-retirement consumer

The [historical13728-byte consumer](https://github.com/nrempel/pokelinux/blob/256c553c1eb7c8434711772fa815d2d5ab21cc00/tools/native-connectivity-worker/first-retirement/consumer.py)
was used unchanged for the native x86_64
[baseline RED and candidate GREEN](../../../docs/evidence/native-connectivity-first-retirement.json).
It observes a startup failure, then sends one later SIGTERM and requires Main's
closure JSON to preserve the first `startup_failure` reason and original cutoff.
The authored target is absent; NET_ADMIN is always outside the bounding set.
No backend/interface effect, GUI recovery or phone behavior is qualified.

The current `consumer.py` is a separately migrated four-path launcher and has
not executed. It keeps the original1s observation/3s closure bounds; its Main
cutoff assertion now checks `reported_until - 3s` within the parent entry
interval rather than equality with parent W. Source review establishes first
placement. The [separate static-entry pair](../../../docs/native-connectivity-installed-startup.md)
uses its own frozen consumer and is the only new CLI runtime result.

Copy `consumer.py` to a fresh runner directory with exact copies of
[`owned.py`](../../native-installed-services/owned.py) and
[`closure.py`](../../native-audio-preview/closure.py) beside it. The consumer
checks their hashes; the evidence records both. Select normal x86_64 Release
Main/worker through actual Cargo compiler-artifact JSON. Providers must be
root:root, executable regular files, nonsymlinks, single-link, without file
capabilities or group/other writes. Their paths and the fresh output parent must
be traversable by UID0 after capability reduction. Use separately verified
provider hashes and retain outer ownership through actual consumer wait/EOF.

```sh
sudo -n /usr/bin/env -i PATH=/usr/bin:/bin LANG=C LC_ALL=C \
  /usr/bin/setpriv --no-new-privs --bounding-set=-all,+setpcap \
  --inh-caps=-all --ambient-caps=-all --reuid=0 --regid=0 --clear-groups \
  /usr/bin/python3 -B /runner/consumer.py \
  --main /providers/pokelinux-connectivity-main --main-sha256 MAIN_SHA256 \
  --worker /providers/pokelinux-connectivity-service --worker-sha256 WORKER_SHA256 \
  --target-name pkl-diag-none --target-ifindex 2147483646 \
  --output /absolute/fresh/output
```

The consumer verifies the target is absent and retains the actual current
namespace. Initial SETPCAP lets Main perform its normal bounding-set reduction;
Main is observed cap0 after the worker's ReduceCapabilities/EPERM failure. Two
readonly Inspect replies precede the one SIGTERM in each recorded case. The
original1s work/3s closure budgets and16384-byte merged PIPE limit are fixed.
Only fully qualified observation and custody can establish RED; setup failures
stay unconfirmed. Results are persisted after actual Main wait/EOF/ECHILD and
before the final assertion. Output contains private host inventory and must not
be published. This source publication does not execute the consumer again.
