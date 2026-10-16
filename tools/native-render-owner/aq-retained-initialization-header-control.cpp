// Frozen syntax-only AQ public-header control. No instances, link or native call.
#include <aquamarine/backend/Backend.hpp>
#include <aquamarine/backend/Session.hpp>
#include <type_traits>
#include <vector>

template<class T> using Shared = Hyprutils::Memory::CSharedPointer<T>;
static_assert(std::is_class_v<Aquamarine::CBackend>);
using ExistingStart = bool (Aquamarine::CBackend::*)();
static_assert(std::is_same_v<decltype(&Aquamarine::CBackend::start), ExistingStart>);
using ExistingImplementations = const std::vector<Shared<Aquamarine::IBackendImplementation>>& (Aquamarine::CBackend::*)();
static_assert(std::is_same_v<decltype(&Aquamarine::CBackend::getImplementations), ExistingImplementations>);
