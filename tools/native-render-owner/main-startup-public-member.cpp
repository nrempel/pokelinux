// Source-only compile contract; never instantiate or execute a compositor.
#include <hyprland/src/Compositor.hpp>
#include <hyprland/src/SharedDefs.hpp>
#include <hyprland/src/debug/HyprCtl.hpp>
#include <functional>
#include <string>
#include <string_view>
#include <type_traits>

using ExistingInitServer = void (CCompositor::*)(std::string, int);
static_assert(std::is_same_v<decltype(&CCompositor::initServer), ExistingInitServer>);
using ExistingHyprCtlCallback = std::function<std::string(eHyprCtlOutputFormat, std::string)>;
static_assert(std::is_same_v<decltype(SHyprCtlCommand::fn), ExistingHyprCtlCallback>);
static_assert(std::is_member_function_pointer_v<decltype(&CHyprCtl::registerCommand)>);

// Deliberately absent from the unchanged current SDK. No feature-test fallback,
// private-member access, local declaration, overload or substitute implementation.
using OwnedStartupReply = std::string (CCompositor::*)(std::string_view);
static_assert(std::is_same_v<decltype(&CCompositor::ownedBackendStartupReply), OwnedStartupReply>);
