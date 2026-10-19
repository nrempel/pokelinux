# Matched Pixel wireless boundary

The retained frankel CP3A.261005.005 4K records identify an external wireless
module path: vendor `cfg80211.ko`, `mac80211.ko` and two DHD candidates,
`bcmdhd4383.ko` and `bcmdhd4390.ko`. This is a static factory catalog checkpoint.
No module, firmware, vendor code or physical phone was executed.

The [generic ARM Rust discovery tracer](native-wireless-scan.md) remains separate.
Its successful hwsim scan does not select or qualify either Pixel driver.

## Existing records and identity

The existing whole-archive authenticated selection pins the factory archive to
`3721e6accc2e87d851ee7966d794318078ec74c7cc32343daa10968264d6bb62`.
This review inherits that authentication; it does not download or authenticate
the complete archive again, verify AVB, or extract another image or module.

Read-only streaming hashes independently recheck the selected boot/system/vendor
images and decompressed common Image. The retained embedded config and four
plaintext module indexes match the earlier pinned audit and the existing
[public factory metadata](../native-boot-inputs/evidence/factory-metadata.json).
All 12 selected inputs are unchanged. Individual module sizes/hashes/vermagic
below come from the retained module catalog, not new module-byte inspection.

The common config records CFG80211 unset, WLAN=y and RFKILL=m. MAC80211 has no
entry in that config; its setting is not inferred. The external module catalog
and dependencies must therefore be considered alongside the common config when
identifying wireless support. Common-source interface definitions alone do not
establish the compiled Pixel control path.

| Catalog entry | Dependency facts | Vendor load-list line |
| --- | --- | --- |
| `cfg80211.ko` | System `rfkill.ko` | 16 |
| `mac80211.ko` | System `libarc4.ko`, vendor `cfg80211.ko`, system `rfkill.ko` | 17 |
| `bcmdhd4383.ko` | `google-pcie.ko`, vendor `cfg80211.ko`, system `rfkill.ko` | 115 |
| `bcmdhd4390.ko` | Same three dependency names | 116 |

Both DHD entries appearing in a load list does not establish which driver binds
actual hardware, successful initialization, an accessible station interface or
working scanning. The DHD dependency records do not list `mac80211.ko`.

All four selected module catalog records carry
`6.6.127-android15-8-gee206ffe8752-ab16254410-4k` vermagic; the common Image's
recorded release is `6.6.127-android15-8-g91d825611104-ab16252310-4k`.
Actual symbol versions, build provenance and profile loading still need
qualification. Keep the selected factory assets together.

## Source and native backend gate

Google publishes candidate [BCM4383](https://android.googlesource.com/kernel/google-modules/wlan/bcmdhd/bcm4383/)
and [BCM4390](https://android.googlesource.com/kernel/google-modules/wlan/bcmdhd/bcm4390/)
repositories. Repository names and branch availability do not bind either
retained ELF to an exact producing revision or build manifest. No candidate's
scan or initialization implementation is promoted to matched-device evidence.

The next boundary is to identify the actual hardware/DT/PCIe selection and exact
module source, then trace firmware/board/calibration inputs, power/reset/rfkill
and driver startup through scan/event/BSS operations. The optional BSS timestamp
and ownership assumptions used by hwsim need separate qualification for that
backend. Association, credentials, IP/routing/Internet, suspend/wake and every
physical-device gate remain open. Zero phone models are hardware-qualified.

The [curated provenance](evidence/native-pixel-wireless-boundary-provenance.json)
contains authored facts and hashes. Raw factory inputs, module indexes/catalog,
module bytes, private review receipts and proprietary firmware stay private.
