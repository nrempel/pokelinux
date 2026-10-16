# Sounds read-only polling and explicit input

The later [complete normal ARM tracer](native-audio-arm-ui.md) qualifies the
separately selected direct-stream UI/audio/theme path. Evidence and remaining
gates below retain this checkpoint's own scope.

Periodic status refreshes no longer disable output controls during a press.
Previously each250ms read-only RPC made `canChoose` false; a response arriving
mid-gesture could cancel the normal tap. Controls now retain the last trusted
available state during status. One completed explicit selection can wait for
that fresh response without losing its original deadline.

The client stores at most one unsent logical ID and ignores further selections
while it is queued. The prior applied route stays visible until actual send.
After the complete fresh status, availability and route membership are checked
again. Unknown/removed routes, unavailability, expiry or disconnection cancel
without mutation. Reconnect never replays the old selection. The effect reply
uses the original one-second CLOCK_BOOTTIME input deadline, including time spent
waiting for status.

Seven new real Unix-socket controls hold status through a120ms enabled interval,
queue one choice, reject a second, preserve the prior applied state and exercise
fresh-state cancellation, expiry, disconnect and original reply-budget refusal.
All seven fail against the previous implementation while the original11 cases
pass. Only client source changes for GREEN:18 behavioral cases plus Qt setup and
cleanup pass,20 total, with byte-identical tests. Strict host compilation passes;
QML, main and current schema3 appearance files are unchanged.

The runner records actual attach/container exits, stopped state, own-container
removal/absence and unchanged inputs. Its merged output is a regular file;
per-helper immutable cutoffs and PIPE EOF are not claimed by this older runner.
[Hash-only provenance](evidence/native-audio-readonly-input-provenance.json).

This checkpoint uses the direct-owner stream client selected by the normal ARM
tracer. Host socket success does not qualify normal GUI/graph/PCM composition or
current lifecycle permission. That tracer and the matching lifecycle migration
remain separate work. No physical phone was accessed.
