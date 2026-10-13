# Native lock admission qualification tools

These authored tools freeze private source/binary/input selections and run bounded
lock-before-original and healthy same-directory restart boundaries. The
[report](../../docs/native-lock-admission.md), [ingress proof](integration-proof.json)
and [restart proof](restart-proof.json) state separate executed evidence and limits.
No raw package, ELF, archive, trace, log or frame is part of the public bundle.

`build.py --output NEW --phase red|green` freezes the current sources and builds
host/ARM binaries offline with formatting/Clippy. It requires the exact pinned
Docker image, private cached Cargo dependencies and the retained workspace path
encoded in the script. Output paths must be new. Initial ingress proof is bound
to its historical pre-persistence sources; `host.py` includes that old root-false
control and is not a current durable-lock regression suite.

`plan.py --original-selection DIR --build DIR --tracer DIR --output NEW` freezes
the installed original-lock boundary with a signature-qualified ARM `strace`.
`run-selected.py` requires exact selection and command hashes, acquires the shared
readonly input lock and checks guest kernel/initrd/root inputs before and after
the actual owned run. `install.py --binary ELF` operates only inside the isolated
root-owned guest: it retains the exact original action before publishing the fixed
wrapper. It is not a production installation certificate.

`restart-plan.py --original DIR --build DIR --output NEW --phase red|green`
freezes the later healthy Stop/wait boundary with changed compositor/display/Source
and the same policy directory. `restart-run.py` requires selection and command
hashes and an unused output label. Its private readonly-backed qcow2 overlay is
bounded to 256 MiB; the Source 100 ms lease and original 500 ms timer remain unchanged.
`restart-host-run.py --build DIR --output NEW` runs six actual host process/file
controls in a disposable network-free container. `restart_boundary.py` and
`restart-host.py` are the corresponding frozen fixture bodies.

`proof.py` and `restart-proof.py` verify exact private epochs and emit only safe
metadata. Their argument parsers require the relevant red/green builds, actual ARM,
host and component selections; include every retained failure with `--failed`.
Do not relabel failed prerequisites as behavioral REDs, mutate frozen epochs or
substitute current source bytes for recorded pins. Fresh documentation changes do
not alter historical build provenance. Keep all substantial new outputs in private
RAM storage when disk capacity is constrained.

The restart GREEN qualifies fresh normal Qt-page denial after healthy replacement;
whole SIGKILL qualifies unclean-history refusal. Trusted unlock, PAM, secure frames,
same-Source continuation, production install, camera plus new-lock runtime, Pixel
kernel and physical devices remain unqualified. Use no physical phone.
