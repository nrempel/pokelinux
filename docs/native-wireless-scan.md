# Native wireless discovery

This tracer exercises Linux wireless discovery below the existing local-network
Settings path. It does not associate, authenticate, configure an address, route
packets or provide Internet access. No physical phone has been accessed.

## Original-tool prerequisite

A fresh headless generic ARM guest loads its matching `mac80211_hwsim` modules
and creates two simulated radios. Original signed hostapd advertises one random
open test SSID on 2412 MHz. One original `iw` scan on the other radio discovers
that exact SSID, BSSID and frequency. The station remains unassociated, and
both interfaces remain without addresses.

The passing epoch ends with actual foreground AP SIGTERM/wait0/stdout EOF,
absence of its owned control endpoint, and removal of the simulated radios and
hwsim module. All 393 pre-existing package versions remain unchanged after the
three signed package additions. There are 15 direct host waits, three boot
waits, 84 guest waits and one poweroff wait, each with actual stdout EOF.
Thirty-three adopted children have actual successful terminal waits and final
`ECHILD`; their origins were not sampled. QEMU exits0, the exact container exits0
and is removed, the same Docker daemon is retained, and selected inputs are
unchanged. These are local fixture and process facts, not vendor cleanup.

Earlier source-only revisions and failed preparation epochs stay separate.
Failures include an exact distro version string, integer versus zero-offset
frequency formatting, and a channel1 filter that initially included the 6GHz
band. None is a native Rust behavioral RED. The final selected-frequency
parser was checked against retained actual output before the passing fresh VM.

## Rust boundary

The desired consumer requires the actual worker to finish under the original
five-second BOOTTIME deadline with a complete scan and the AP's exact BSS.
An ACK alone, stale dump, missing timestamp, malformed result, late child or
unqualified cleanup cannot satisfy it. The actual normal ARM refusing baseline
returns code 2 with exact unsupported JSON in 54.869ms, then the desired discovery
assertion fails after qualified AP/endpoint/radio/module/process cleanup.
This is a genuine native integration RED, separate from the original-tool pass.
Its 15 host, three boot, 85 guest and one poweroff waits have actual stdout EOF;
33 adopted terminal waits succeed, then ECHILD. QEMU exits0, while the expected
desired-assertion failure makes the container exit 1; its exact identity is
removed and inputs remain unchanged. The outer qualification pass does not
mean desired discovery passed.

The completing implementation passes 13 host component tests, formatting,
strict Clippy and normal host/ARM Release builds. A separate compiled 12-pass /
one-failure classification RED catches setup ENOENT becoming unsupported;
that same test passes after restricting unavailability to recognized netlink
preflight results. Missing setup remains failed. This component evidence is
separate from native kernel discovery. The first compile/lint failure and
unexecuted preparation snapshots remain retained; none is promoted to GREEN.

The portable boundary is generic netlink `nl80211`. A scan notification has no
generic per-request cookie, so this tracer requires newly created, exclusively
administered simulated radios. Interface names, ifindex, wdev, FLUSH and
namespace inventories alone do not establish exclusive ownership or causality.
The optional kernel BSS last-seen BOOTTIME timestamp describes received frames
with about 10ms accuracy. The consumer's in-operation-span check is a strict
hwsim acceptance gate, not a guarantee for every physical driver.

## Actual native ARM discovery passes

The unchanged desired consumer passes with the normal completing ARM Release
worker. The binary receives only ifindex, frequency and the original cutoff;
it receives no expected SSID or BSSID. It discovers the independently advertised
nonce AP through real generic-netlink requests, kernel scan notifications and
a complete BSS dump. There is no original `iw` scan in this native epoch.

The operation takes 47.276ms. Original arming through actual worker wait and
stdout EOF takes 102.045ms under the unchanged five-second limit. The one 365-byte
completed JSON result carries the exact BSSID/SSID/frequency and a kernel
last-seen BOOTTIME within the actual operation span. These are single controlled
TCG-guest observations, not performance benchmarks or physical RF measurements.

The consumer's one desired assertion runs after qualified worker/AP wait and
EOF, control-endpoint absence, module/radio removal and unchanged 393 package
versions. All 15 host, three boot, 85 guest and one poweroff waits/EOF qualify;
33 adopted terminal waits succeed and final ECHILD passes. QEMU and the exact
container exit 0, all handles close, the container is removed, the same daemon
is retained, and selected inputs remain unchanged. Reaping is not vendor cleanup.

The [curated provenance](evidence/native-wireless-scan-provenance.json) binds
source, normal binaries, original tools, genuine refusing RED, component QA and
the later native GREEN without publishing private runtime artifacts.
[Component API and ownership contract](../native-wireless-scan/README.md),
[unchanged consumer and refusing source](../tools/native-wireless-scan/README.md).

## Separate unavailable control passes

The exact same completing ARM binary runs once in a fresh generic ARM guest
with only loopback and no wireless phy. The fixture observes loopback's actual
ifindex and type before invocation; it does not assume an index. The worker
returns code 2 and exact unsupported JSON, with matching selectors and original
cutoff. Original arming through actual wait/stdout EOF takes 155.782ms within 5s.
No AP, radio creation, original `iw` scan or package installation is admitted.

Loopback/namespace identity and the absence of wireless interfaces persist.
The module inventory gains `cfg80211` and `rfkill`; this is not a zero-effects
claim. All 393 package versions remain unchanged. There are 15 host, three boot,
four guest and one poweroff actual waits/stdout EOFs, no adopted child, and final
ECHILD. QEMU and the exact container exit 0 with closed handles, removal, the
same daemon and unchanged selected inputs. Permission failure, malformed output,
expiry, setup failure and uncertain state cannot pass this control.

## Pixel and remaining gates

The matched Pixel common source exposes standard `nl80211` and vendor commands.
That does not establish that its actual WLAN module can scan without vendor
initialization. Existing authenticated config and module-index records must
identify that module and its source, initialization, firmware and loading
requirements. Generic ARM discovery qualifies no matched Google-kernel Wi-Fi
backend, association, credentials, suspend/wake, deployment or physical phone.
Zero phone models are hardware-qualified.

Primary interface references: [Linux nl80211 specification](https://www.kernel.org/doc/html/next/networking/netlink_spec/nl80211.html),
[cfg80211 timestamps](https://docs.kernel.org/next/driver-api/80211/cfg80211.html),
[pinned public scan notifications and BSS dump](https://kernel.googlesource.com/pub/scm/linux/kernel/git/torvalds/linux/+/ea5751ccd665a2fd1b24f9af81f6167f0718c5f6/net/wireless/nl80211.c),
and [matched Pixel common interface](https://android.googlesource.com/kernel/common/+/91d825611104e75e1a9e0e0f6b25580eddb68828/net/wireless/nl80211.c).
The public source reference is not proof of exact generic-runtime build membership.
