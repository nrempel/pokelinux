# Native radio integration: next execution gate

Status: proposed test bootstrap, not yet implemented or qualified. Production
radio traffic must use the native transport; Android exists here as an independent
synthetic test peer. No physical phone access is required.

## Test peer bootstrap

Use an instrumentation APK targeting its own test package. The native harness
starts it through the exact SDK37 IActivityManager interface and supplies a native
IInstrumentationWatcher. The APK passes its live synthetic Binder endpoint in a
status Bundle. The native watcher validates the expected test identity, retains
and queues the handle, acknowledges the watcher call, and returns. Registration
runs outside callback dispatch. Keep the peer alive until controlled shutdown.

Android documents [Binder values in Bundles](https://developer.android.com/reference/android/os/Bundle#putBinder(java.lang.String,%20android.os.IBinder))
and [instrumentation status delivery](https://developer.android.com/reference/android/app/Instrumentation#sendStatus(int,%20android.os.Bundle)).
Those APIs support this proposal; they do not prove execution on our image.
Pin and verify the test-only framework transaction ABI against the installed
image. Do not copy it into the production radio client or guess transaction IDs.

Record the image, client/peer UIDs, SELinux domains and Binder device. Keep SELinux
enforcing; no root or permissive-domain shortcut. This tests handle transfer and
transport, not production vendor service discovery or access policy.

## Required end-to-end tests

- Two distinct callbacks, registration ID and error propagation, original wire
  fields/flags, raw and lossless segmented requests at 2048, 2049 and 4096 bytes.
- Callback before registration reply, callback with no concurrent client request,
  and simultaneous response/indication delivery. Start the Binder pool first;
  hold no session lock across remote calls or blocking queue operations.
- Death before/after registration and submission; new callback endpoints on
  reconnect. Reused client IDs and serials must not let late old-session events
  alter the replacement session. Retain the death recipient strongly.
- Local inbox overflow and driver queue pressure as separate cases. Terminal
  receiver failure must wake the owner and resolve pending operations explicitly;
  one-way handler errors are not acknowledgements to the remote sender.
- Malformed remote parcels, metadata length mismatch, bounds, trailing data and
  unknown transactions. Verify bounded resource use through the actual adapter.
- Withheld synchronous registration reply: bounded session-owner shutdown and
  exclusion of late results. A watchdog does not cancel an in-flight Binder call.
- Both callback flag modes in the synthetic peer. Actual vendor sender flags are
  still unknown. The current receiver writes no success exception header for
  response codes 1/2; a synthetic synchronous caller expecting readException()
  must expose this difference. The original receiver also lacks that header;
  do not report this conditional case as a proven vendor incompatibility.

Registration original-proxy execution is now covered by
[the five passing cases](evidence/registration-provenance.json). Separate-process
native radio registration and all gates above remain open.

## Independent review

ChatGPT Pro review completed in Chat mode with Pro power selected, matching marker
PRO-BRIDGE-20261012-radio-42f7 and one context round. It recommended the
instrumentation bootstrap and the failure cases above. The first UI entry sent
only the marker; the complete brief was supplied and verified in the context
round. The review is advisory. Local source inspection independently confirms
weak death-recipient storage and the rsbinder 0.12 warning about late replies
after certain driver failures. The bootstrap still requires runtime evidence.

Passing this gate will establish emulator IPC and recovery only. ARM64 kernel
Binder, vendor permissions, modem/IMS behavior, GPU, camera and physical audio
remain separate qualifications.
