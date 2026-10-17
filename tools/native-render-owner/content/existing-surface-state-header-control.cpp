// Frozen existing class/state controls; syntax only, no instances or execution.
#include <hyprland/src/Compositor.hpp>
#include <hyprland/src/SharedDefs.hpp>
#include <hyprland/src/debug/HyprCtl.hpp>
#include <hyprland/src/protocols/types/SurfaceState.hpp>
#include <functional>
#include <string>
#include <string_view>
#include <type_traits>

using ExistingInitServer = void (CCompositor::*)(std::string, int);
static_assert(std::is_same_v<decltype(&CCompositor::initServer), ExistingInitServer>);
using ExistingHyprCtlCallback = std::function<std::string(eHyprCtlOutputFormat, std::string)>;
static_assert(std::is_same_v<decltype(SHyprCtlCommand::fn), ExistingHyprCtlCallback>);
static_assert(std::is_member_function_pointer_v<decltype(&CHyprCtl::registerCommand)>);

static_assert(std::is_class_v<SSurfaceState>);
static_assert(std::is_same_v<decltype(SSurfaceState::buffer), CHLBufferReference>);
static_assert(std::is_same_v<decltype(SSurfaceState::shmSnapshot), Render::CSHMSnapshot>);
using ExistingSurfaceUpdateFrom = void (SSurfaceState::*)(SSurfaceState&);
static_assert(std::is_same_v<decltype(&SSurfaceState::updateFrom), ExistingSurfaceUpdateFrom>);
using ExistingSurfaceReset = void (SSurfaceState::*)();
static_assert(std::is_same_v<decltype(&SSurfaceState::reset), ExistingSurfaceReset>);
