# Consumer integration

Use the [selection and qualification instructions](README.md). `consumer.py` is a
phase consumer of the concrete `green_adapter.py` and existing owned guest
namespace. It has no standalone entry point. Keep its functional conditions and
independent fixture bytes pinned while implementing or testing production code.

`envelope/guest.py` installs actual selected binaries and binds executable/process
identity, config ownership, native lifecycle receipts and PID1 outcomes before
accepting the case result. `operator_client.py` uses the real credentialed socket;
`fixture/src/bin/controller.rs` uses the independent real Binder peer. No owner
library shortcut, mock transport or production test API is selected.

Read the actual candidate's event/wire source before selecting its build. Readiness
requires both role-bound channels and durable launch markers. Permission requires
matching sync ACK, Main's matching Finish message to the writer, actual writer
wait0 and fresh original-deadline gates. Main remains alive while the writer exits.
Proc descriptor metadata is diagnostic, not independent peer-authentication proof.

`manager_stop_consumer.py` is the separate sixth case; consumer.py retains the
original five functional oracles unchanged. The manager case calls only the
adapter manager_stop entry, never socket Stop or StopAck, then shares the existing
native/PID1/unchanged-marker/refusal closure. Each case selects a fresh epoch.

ExecStop is checked through actual typed Manager.GetUnit/Service.ExecStop busctl
queries, not by normalizing an omitted systemctl property. Both extra owned CLI
waits retain the original show deadline and regular stdout sinks. See the primary
interface/output links in README.md; unavailable prerequisites fail setup.
