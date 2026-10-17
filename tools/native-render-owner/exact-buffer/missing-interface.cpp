#include <aquamarine/allocator/Swapchain.hpp>
#include <aquamarine/allocator/GBM.hpp>
#include <aquamarine/backend/DRM.hpp>
using ExactRequest = Aquamarine::SExactBufferRequest;
using ExactConfigure = decltype(&Aquamarine::CSwapchain::reconfigureExact);
using ExactAllocation = decltype(&Aquamarine::CGBMBuffer::exactAllocation);
using ExactImport = decltype(&Aquamarine::CDRMFB::createExact);
using ExactImportReceipt = decltype(&Aquamarine::CDRMFB::exactImport);
using ExactClose = decltype(&Aquamarine::CDRMFB::closeExact);
