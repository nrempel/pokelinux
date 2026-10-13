# Owned native Messages client worker

The native Messages window owns one Qt thread for its direct authenticated Unix
socket exchange. Stop is checked inside send/receive polling, every RPC uses a
checked absolute CLOCK_BOOTTIME deadline, and received bytes are validated again
before exchange completion. A clock error refuses the request or result. The
thread is never detached or terminated. Destructor cleanup uses 20ms actual wait
slices with a 500ms boot-time budget and deletes the QThread only after successful
wait. If clock/exit observation remains uncertain, the GUI aborts before member
or thread destruction. With an unavailable clock it permits one 20ms actual wait;
otherwise it aborts. This prevents an implicit destructor wait from hiding an
unbounded cleanup. [Qt's wait contract](https://doc.qt.io/qt-6.8/qthread.html#wait)
provides the Linux completion synchronization, including thread-local cleanup.

Actual frozen-baseline reds exposed five behaviors: close waited about the full
RPC budget, an admission clock error still emitted a query, a completion clock
error published history, a clock error during held receive hung beyond the 3s
harness limit, and cleanup had no bounded exit-observation fence. The same seven
actual Qt 6.8.2/Linux socket checks now pass. Ordinary held close takes 20ms in the
selected run. Separate normal and late completion controls pass.

The strengthened completion tests call real recvmsg and receive 113 bytes through
a credentialled SOCK_SEQPACKET before injecting caller clock EIO or delaying the
return for 1100ms. Both corrected exchanges refuse publication. The wait-report
injection first completes the real Qt wait, records `actual_finished=1`, then
returns false; the GUI aborts before the post-destructor marker. A clock error
while exit is unobserved also aborts. These are explicit failure dispositions,
not proof of actual suspend or an uninterruptible kernel task. The wrappers exist
only in the test executable, with no production selector or private-access hook.

The server socket and production client thread run in one actual process in an
isolated container with no network; kernel credentials are checked on both ends.
UID0 is needed only for the production root-peer check. The packet is
authored synthetic local history. This focused test does not execute the service,
Binder, Omarchy renderer or phone hardware. Their installed composition is
recorded separately in [Messages service](native-messages-service.md).
The existing queued GUI apply has a request-generation fence; this proof qualifies
exchange admission/receipt completion, not arbitrary delayed GUI-delivery expiry
or renewed current OS read authority. The lifecycle actor owns current data release.

Reproduce with the prepared Qt image:

```sh
bash tools/native-messages-ui-worker-suite.sh red unique-red-label
bash tools/native-messages-ui-worker-suite.sh green unique-green-label
```

Red requires the ignored frozen baseline. Output labels refuse overwrite. The
initial output-mount PermissionError is retained as an unselected setup failure;
earlier weaker completion-observation runs remain historical. The selected red
harness differs from current source only in subsequently corrected scope/comment
text; its exact own-source snapshot is retained. [Clean provenance](evidence/native-messages-ui-worker-provenance.json)
pins the six current sources, selected executables and private evidence index.
No proprietary inputs or physical phone are involved.
