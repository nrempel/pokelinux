#include <aquamarine/allocator/Swapchain.hpp>
#include <aquamarine/allocator/GBM.hpp>
#include <aquamarine/backend/DRM.hpp>
#include <aquamarine/backend/DisplayAttempt.hpp>
using ExistingSwapchain = decltype(&Aquamarine::CSwapchain::reconfigure);
using ExistingAllocation = decltype(&Aquamarine::CGBMBuffer::dmabuf);
using ExistingImport = decltype(&Aquamarine::CDRMFB::create);
using ExistingClose = decltype(&Aquamarine::CDRMFB::drop);
using ExistingDenyScope = decltype(&Aquamarine::CDisplayAttemptScope::bound);
