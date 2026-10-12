# Rust transport review

[Independent Pro consultation]((private review transcript)), marker `PRO-BRIDGE-20261012-b8c3`.

Verified Chat selected, composer Pro selected, one marked user turn, final marked
response, response actions present and generation stopped. No context rounds.
Only the dedicated review tab was closed. No credentials or phone data were sent.

The review supports the design, subject to implementation and integration tests:

- Complete TLS handshake with normal root/name validation, then verify the leaf
  DER SHA256 pin before queuing any application bytes. Disable resumption and
  early data explicitly.
- Start the budget before TCP connect. Recompute it for every underlying read or
  write, including rustls internal loops. Check completion even if TLS has data
  buffered. Use a suspend-inclusive clock on Linux.
- A write or flush is not an execution receipt. After the first application write
  attempt, any failure without a valid matching reply is unknown. Preserve a
  valid receipt even if later connection cleanup fails. Do not wait for EOF.
- Deserialize bounded JSON bytes directly into strict types; a preliminary
  `serde_json::Value` parse loses duplicate-key evidence. Validate nested fields,
  request identity, generation and allowed outcome/reason combinations.
- Prove client authentication at the Android endpoint. Merely supplying a client
  certificate cannot force a server to request it.

Codex verification: `runtime/src/tls.rs` explicitly completes the handshake before
pin checking, uses an enrollment-only root store, TLS 1.3, disabled resumption and
no early data. The production transport returns bounded response bytes, not a
trusted execution receipt. Typed Android reply decoding remains a required next
step. The emulator interoperability test parses JSON only to inspect known test
responses; it is not the production reply validator.

Linux CLOCK_BOOTTIME now drives deadline checks, including checks after I/O.
This prevents acceptance of a late response after resume. It does not guarantee
a blocked thread wakes while suspended. A helper test covers exact expiry and
large clock advancement; a real isolated-VM suspend test remains outstanding.
No host or physical phone suspension was attempted.

Additional fault coverage still required before the complete service can ship:
expired certificate and TLS-1.2-only peers, injected blocked write and interrupted
I/O, strict nested reply decoding, no-replay behavior through the actual daemon,
service cancellation/lifecycle, and installed Arch-to-Android commands.

Primary API references checked during implementation:
[StreamOwned](https://docs.rs/rustls/0.23.45/rustls/struct.StreamOwned.html),
[ClientConfig](https://docs.rs/rustls/0.23.45/rustls/client/struct.ClientConfig.html),
[TcpStream](https://doc.rust-lang.org/std/net/struct.TcpStream.html).
Pinned dependency source was used to verify PEM loading APIs.
