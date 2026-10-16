// Link-only address probe. Never loaded or executed; no function definitions.
#ifndef POKELINUX_STARTUP_CUSTODY_TEST
#error Matching AQ test definition is required
#endif
#include <aquamarine/backend/StartupCustodyTest.hpp>
#include <type_traits>
using namespace Aquamarine::StartupCustodyTest;
static_assert(std::is_same_v<decltype(&probeStamp), SProbeStamp (*)() noexcept>);
static_assert(std::is_same_v<decltype(&forceAfterResourceScan), bool (*)(const SWeakGraphView&) noexcept>);
static_assert(std::is_same_v<decltype(&forceAfterImplementationStart), bool (*)(const SWeakGraphView&, bool) noexcept>);
static_assert(std::is_same_v<decltype(&observeChildDestructorEntry), void (*)(const Aquamarine::CDRMBackend*) noexcept>);
decltype(&probeStamp) aq_test_probe_stamp = &probeStamp;
decltype(&forceAfterResourceScan) aq_test_probe_scan = &forceAfterResourceScan;
decltype(&forceAfterImplementationStart) aq_test_probe_start = &forceAfterImplementationStart;
decltype(&observeChildDestructorEntry) aq_test_probe_destructor = &observeChildDestructorEntry;
decltype(&CProbeScope::bound) aq_test_probe_bound = &CProbeScope::bound;
decltype(&CProbeScope::current) aq_test_probe_current = &CProbeScope::current;
