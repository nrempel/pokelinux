#include <aidl/android/hardware/sensors/Event.h>
#include <type_traits>
using aidl::android::hardware::sensors::Event;
static_assert(Event::fixed_size::value);
static_assert(std::is_standard_layout_v<Event> && std::is_standard_layout_v<Event::EventPayload>);
static_assert(alignof(Event)<=8 && sizeof(Event)%8==0);
// Original compiler object metadata gives Rust only an opaque all-bit-pattern-valid element size.
extern "C" { char pokelinux_sensor_event_size[sizeof(Event)]={}; }
