# Actual retained parent/worker channel consumer

Read [the contract, actual evidence and limits](../../docs/native-process-channel.md).
The current-control bin uses only the original process-owner API; the feature-gated
consumer calls the retained message API directly. No future-method shims or fake
kernel identity are provided. This final source includes a documented nonsemantic
Path/AsFd lint refactor after the original missing-interface RED and initial GREEN.

Build with the captured lock and the `retained-channel` feature. Execute each bin
with a distinct nonexistent private runtime path, using a bounded owning external
runner. Keep the per-group original work/cleanup bounds; do not reuse or remove
admission markers. The original qualification used a90-second managed container.
Rights-overflow, credentials, EAGAIN and child wait/EOF require a real supported
Linux kernel. User-mode QEMU alone does not qualify this process boundary.
