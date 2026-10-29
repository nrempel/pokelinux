# Persistent wireless service acceptance

This is the desired integration contract, not an executed test or implemented
backend. No RED, GREEN, worker launch, radio mutation or phone qualification is
claimed by these files. The runtime consumer has not been written.

[CONTRACT.json](CONTRACT.json) requires the normal connectivity Main/actor/worker
to own one association and one fixed IPv4 profile across requester closure.
Independent hwsim station/AP readback and packets must agree before explicit
Disconnect removes the profile and radio association. Existing actual-boundary
consumers are listed for reuse; a new generic harness is not required.

Two source constraints drive the implementation: `service.rs` currently carries
only the created-veth Fact with a mandatory witnessed lifetime, and wireless
`associate_open_once` closes its observation sockets when it returns. An existing
wireless interface must not receive a fabricated lifetime, and IPv4-only refresh
must not stand in for retained radio observation.

The startup CLI and protected profile schema remain under review. Preserve the
existing action deadline; do not relax it to make a test pass. Open association
and the fixed documentation address are controlled tracer inputs. Credentials,
WPA, DHCP/DNS, Internet, Pixel driver startup and hardware remain open.
