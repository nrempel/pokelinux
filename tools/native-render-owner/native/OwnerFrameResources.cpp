#include "OwnerFrameResources.hpp"
#include <EGL/eglext.h>
#include <GLES3/gl32.h>
#include <array>
#include <bit>
#include <exception>
#include <fcntl.h>
#include <limits>
#include <string_view>
#include <thread>
#include <unistd.h>
#include <utility>

using namespace Render::GL;
namespace {
constexpr uint32_t AR24 = 0x34325241, XR24 = 0x34325258;
bool same(const PrShmDescription& a, const PrShmDescription& b) noexcept {
    return a.resource_id == b.resource_id && a.generation == b.generation && a.commit_sequence == b.commit_sequence && a.drm_format == b.drm_format &&
        a.width == b.width && a.height == b.height && a.stride == b.stride && a.bytes_per_pixel == b.bytes_per_pixel && a.reserved == b.reserved;
}
bool valid(const PrSnapshotView& view) noexcept {
    const auto& d = view.description;
    return view.pixels && d.resource_id && d.generation && d.commit_sequence && !d.reserved && d.width && d.height && d.width <= 16384 && d.height <= 16384 &&
        d.bytes_per_pixel == 4 && static_cast<uint64_t>(d.stride) >= static_cast<uint64_t>(d.width) * 4 &&
        view.pixel_bytes == static_cast<uint64_t>(d.stride) * d.height && view.pixel_bytes <= 64ULL * 1024 * 1024;
}
bool extension(const char* text, std::string_view wanted) noexcept {
    if (!text)
        return false;
    std::string_view remaining{text};
    while (!remaining.empty()) {
        const auto end = remaining.find(' ');
        if (remaining.substr(0, end) == wanted)
            return true;
        if (end == std::string_view::npos)
            return false;
        remaining.remove_prefix(end + 1);
    }
    return false;
}
struct Owner {
    const EGLDisplay display;
    const EGLContext context;
    const std::thread::id thread = std::this_thread::get_id();
    Owner(EGLDisplay d, EGLContext c) : display(d), context(c) {}
    bool current() const noexcept {
        // Refuse the wrong thread before touching thread-local EGL/GL state or
        // mutable resource fields. Methods never move a current context.
        return std::this_thread::get_id() == thread && display != EGL_NO_DISPLAY && context != EGL_NO_CONTEXT &&
            eglGetCurrentDisplay() == display && eglGetCurrentContext() == context;
    }
};
struct UnpackState {
    GLint binding = 0, pbo = 0, alignment = 0, rowLength = 0, skipPixels = 0, skipRows = 0;
    void capture() noexcept {
        glGetIntegerv(GL_TEXTURE_BINDING_2D, &binding);
        glGetIntegerv(GL_PIXEL_UNPACK_BUFFER_BINDING, &pbo);
        glGetIntegerv(GL_UNPACK_ALIGNMENT, &alignment);
        glGetIntegerv(GL_UNPACK_ROW_LENGTH, &rowLength);
        glGetIntegerv(GL_UNPACK_SKIP_PIXELS, &skipPixels);
        glGetIntegerv(GL_UNPACK_SKIP_ROWS, &skipRows);
    }
    void restore() const noexcept {
        glBindTexture(GL_TEXTURE_2D, static_cast<GLuint>(binding));
        glBindBuffer(GL_PIXEL_UNPACK_BUFFER, static_cast<GLuint>(pbo));
        glPixelStorei(GL_UNPACK_ALIGNMENT, alignment);
        glPixelStorei(GL_UNPACK_ROW_LENGTH, rowLength);
        glPixelStorei(GL_UNPACK_SKIP_PIXELS, skipPixels);
        glPixelStorei(GL_UNPACK_SKIP_ROWS, skipRows);
    }
};
}

struct COwnerShmTexture::SImpl : Owner {
    PrSnapshot* snapshot = nullptr;
    PrSnapshotView view{};
    GLuint name = 0;
    bool ready = false;
    using Owner::Owner;
};
COwnerShmTexture::COwnerShmTexture(EGLDisplay display, EGLContext context) : m_impl(std::make_unique<SImpl>(display, context)) {}
COwnerShmTexture::~COwnerShmTexture() noexcept {
    // Destruction must never release a snapshot/GL name on a control thread or
    // imply cleanup after a refused/uncertain native operation.
    if (m_impl->snapshot || m_impl->name)
        std::terminate();
}
eOwnerResourceStatus COwnerShmTexture::upload(const PrSnapshot* snapshot) noexcept {
    auto& s = *m_impl;
    if (!s.current())
        return eOwnerResourceStatus::WRONG_OWNER_CONTEXT;
    if (s.snapshot || s.name)
        return eOwnerResourceStatus::BUSY;
    PrSnapshotView view{};
    if (pr_snapshot_view(snapshot, &view) != PR_OK || !valid(view))
        return eOwnerResourceStatus::INVALID_INPUT;
    if ((view.description.drm_format != AR24 && view.description.drm_format != XR24) || std::endian::native != std::endian::little)
        return eOwnerResourceStatus::UNSUPPORTED_FORMAT;
    // Save explicit clone custody before the first GL query/acquisition. The
    // caller's handle slot, visitor and producer mapping are never retained.
    s.snapshot = pr_snapshot_clone(snapshot);
    if (!s.snapshot)
        return eOwnerResourceStatus::ALLOCATION;
    if (pr_snapshot_view(s.snapshot, &s.view) != PR_OK || !valid(s.view) || !same(view.description, s.view.description) ||
        view.pixels != s.view.pixels || view.pixel_bytes != s.view.pixel_bytes)
        return eOwnerResourceStatus::RESOURCE_MISMATCH;
    if (glGetError() != GL_NO_ERROR)
        return eOwnerResourceStatus::GL_ERROR;
    UnpackState previous;
    previous.capture();
    if (glGetError() != GL_NO_ERROR)
        return eOwnerResourceStatus::GL_ERROR;
    glGenTextures(1, &s.name); // Store every acquisition immediately.
    if (!s.name || glGetError() != GL_NO_ERROR)
        return eOwnerResourceStatus::GL_ERROR;
    glBindTexture(GL_TEXTURE_2D, s.name);
    glBindBuffer(GL_PIXEL_UNPACK_BUFFER, 0);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glPixelStorei(GL_UNPACK_SKIP_PIXELS, 0);
    glPixelStorei(GL_UNPACK_SKIP_ROWS, 0);
    const auto& d = s.view.description;
    // AR24/XR24 bytes on supported little-endian targets are B,G,R,A/X.
    // GLES3 core swizzles avoid an optional BGRA upload extension and preserve
    // the actual immutable source bytes. XR24's padding channel is never alpha.
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_SWIZZLE_R, GL_BLUE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_SWIZZLE_B, GL_RED);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_SWIZZLE_A, d.drm_format == AR24 ? GL_ALPHA : GL_ONE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    if (d.stride % 4 == 0) {
        glPixelStorei(GL_UNPACK_ROW_LENGTH, static_cast<GLint>(d.stride / 4));
        glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA8, static_cast<GLsizei>(d.width), static_cast<GLsizei>(d.height), 0, GL_RGBA, GL_UNSIGNED_BYTE, s.view.pixels);
    } else {
        // A valid Rust snapshot can have a byte stride not divisible by four.
        // Upload whole rows explicitly rather than rounding/truncating stride.
        glPixelStorei(GL_UNPACK_ROW_LENGTH, 0);
        glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA8, static_cast<GLsizei>(d.width), static_cast<GLsizei>(d.height), 0, GL_RGBA, GL_UNSIGNED_BYTE, nullptr);
        for (uint32_t row = 0; row < d.height; ++row)
            glTexSubImage2D(GL_TEXTURE_2D, 0, 0, static_cast<GLint>(row), static_cast<GLsizei>(d.width), 1, GL_RGBA, GL_UNSIGNED_BYTE,
                            s.view.pixels + static_cast<uint64_t>(row) * d.stride);
    }
    const auto issuedError = glGetError();
    previous.restore();
    const auto restoreError = glGetError();
    if (issuedError != GL_NO_ERROR || restoreError != GL_NO_ERROR)
        return eOwnerResourceStatus::GL_ERROR;
    s.ready = true;
    return eOwnerResourceStatus::OK; // Command issue only, never GPU completion.
}
eOwnerResourceStatus COwnerShmTexture::describe(SOwnerTextureInfo& output) const noexcept {
    const auto& s = *m_impl;
    if (!s.current())
        return eOwnerResourceStatus::WRONG_OWNER_CONTEXT;
    if (!s.ready)
        return eOwnerResourceStatus::NOT_READY;
    output = {s.view.description, s.name, s.view.description.drm_format == AR24};
    return eOwnerResourceStatus::OK;
}
eOwnerResourceStatus COwnerShmTexture::matches(const PrSnapshot* snapshot) const noexcept {
    const auto& s = *m_impl;
    if (!s.current())
        return eOwnerResourceStatus::WRONG_OWNER_CONTEXT;
    if (!s.ready)
        return eOwnerResourceStatus::NOT_READY;
    PrSnapshotView view{};
    if (pr_snapshot_view(snapshot, &view) != PR_OK || !valid(view) || !same(s.view.description, view.description) ||
        s.view.pixels != view.pixels || s.view.pixel_bytes != view.pixel_bytes)
        return eOwnerResourceStatus::RESOURCE_MISMATCH;
    return eOwnerResourceStatus::OK;
}
eOwnerResourceStatus COwnerShmTexture::retire() noexcept {
    auto& s = *m_impl;
    if (!s.current())
        return eOwnerResourceStatus::WRONG_OWNER_CONTEXT;
    if (s.name) {
        if (glGetError() != GL_NO_ERROR)
            return eOwnerResourceStatus::GL_ERROR;
        glDeleteTextures(1, &s.name);
        if (glGetError() != GL_NO_ERROR)
            return eOwnerResourceStatus::GL_ERROR;
        s.name = 0;
    }
    if (s.snapshot)
        pr_snapshot_release(std::exchange(s.snapshot, nullptr));
    s.view = {};
    s.ready = false;
    return eOwnerResourceStatus::OK; // Local release; caller owns genuine fence facts.
}

struct COwnerNativeFence::SImpl : Owner {
    EGLSyncKHR sync = EGL_NO_SYNC_KHR;
    int fd = -1;
    SOwnerFenceTag tag{};
    uint64_t lastSequence = 0;
    bool ready = false, cleanupFailed = false;
    PFNEGLCREATESYNCKHRPROC create = nullptr;
    PFNEGLDUPNATIVEFENCEFDANDROIDPROC duplicate = nullptr;
    PFNEGLDESTROYSYNCKHRPROC destroy = nullptr;
    using Owner::Owner;
};
COwnerNativeFence::COwnerNativeFence(EGLDisplay display, EGLContext context) : m_impl(std::make_unique<SImpl>(display, context)) {}
COwnerNativeFence::~COwnerNativeFence() noexcept {
    if (m_impl->sync != EGL_NO_SYNC_KHR || m_impl->fd >= 0 || m_impl->cleanupFailed)
        std::terminate();
}
eOwnerResourceStatus COwnerNativeFence::issue(const SOwnerFenceTag& tag) noexcept {
    auto& s = *m_impl;
    if (!s.current())
        return eOwnerResourceStatus::WRONG_OWNER_CONTEXT;
    if (s.sync != EGL_NO_SYNC_KHR || s.fd >= 0 || s.cleanupFailed)
        return eOwnerResourceStatus::BUSY;
    if (!tag.admissionId || !tag.sceneSequence || !tag.securityGeneration || !tag.outputId || !tag.outputGeneration || !tag.fenceSequence || tag.fenceSequence <= s.lastSequence)
        return eOwnerResourceStatus::INVALID_INPUT;
    if (glGetError() != GL_NO_ERROR)
        return eOwnerResourceStatus::GL_ERROR;
    if (eglGetError() != EGL_SUCCESS)
        return eOwnerResourceStatus::EGL_ERROR;
    if (!extension(eglQueryString(s.display, EGL_EXTENSIONS), "EGL_ANDROID_native_fence_sync"))
        return eOwnerResourceStatus::EGL_ERROR;
    s.create = reinterpret_cast<PFNEGLCREATESYNCKHRPROC>(eglGetProcAddress("eglCreateSyncKHR"));
    s.duplicate = reinterpret_cast<PFNEGLDUPNATIVEFENCEFDANDROIDPROC>(eglGetProcAddress("eglDupNativeFenceFDANDROID"));
    s.destroy = reinterpret_cast<PFNEGLDESTROYSYNCKHRPROC>(eglGetProcAddress("eglDestroySyncKHR"));
    if (!s.create || !s.duplicate || !s.destroy)
        return eOwnerResourceStatus::EGL_ERROR;
    // Preserve the actual production ordering: native sync/null attributes,
    // then GL flush. Every partial acquisition stays with this retained owner.
    s.sync = s.create(s.display, EGL_SYNC_NATIVE_FENCE_ANDROID, nullptr);
    if (s.sync == EGL_NO_SYNC_KHR)
        return eOwnerResourceStatus::EGL_ERROR;
    s.tag = tag;
    s.lastSequence = tag.fenceSequence;
    glFlush();
    if (glGetError() != GL_NO_ERROR)
        return eOwnerResourceStatus::GL_ERROR;
    // Export transfers a separately owned FD. Save it before every fallible
    // check so an export/flag failure cannot drop native custody on unwind.
    s.fd = s.duplicate(s.display, s.sync);
    if (s.fd < 0 || eglGetError() != EGL_SUCCESS)
        return eOwnerResourceStatus::EGL_ERROR;
    const int flags = fcntl(s.fd, F_GETFD);
    if (flags < 0 || fcntl(s.fd, F_SETFD, flags | FD_CLOEXEC) < 0)
        return eOwnerResourceStatus::EGL_ERROR;
    const int confirmed = fcntl(s.fd, F_GETFD);
    if (confirmed < 0 || !(confirmed & FD_CLOEXEC))
        return eOwnerResourceStatus::EGL_ERROR;
    s.ready = true;
    return eOwnerResourceStatus::OK; // Issued/exported, never completion.
}
eOwnerResourceStatus COwnerNativeFence::borrowedFD(SOwnerFenceFD& output) const noexcept {
    const auto& s = *m_impl;
    if (!s.current())
        return eOwnerResourceStatus::WRONG_OWNER_CONTEXT;
    if (!s.ready || s.sync == EGL_NO_SYNC_KHR || s.fd < 0 || s.cleanupFailed)
        return eOwnerResourceStatus::NOT_READY;
    output = {s.fd, s.tag};
    return eOwnerResourceStatus::OK;
}
eOwnerResourceStatus COwnerNativeFence::retire() noexcept {
    auto& s = *m_impl;
    if (!s.current())
        return eOwnerResourceStatus::WRONG_OWNER_CONTEXT;
    if (s.cleanupFailed)
        return eOwnerResourceStatus::EGL_ERROR;
    if (s.sync != EGL_NO_SYNC_KHR) {
        if (!s.destroy || !s.destroy(s.display, s.sync))
            return eOwnerResourceStatus::EGL_ERROR;
        s.sync = EGL_NO_SYNC_KHR;
        s.ready = false;
    }
    if (s.fd >= 0) {
        // Linux close releases its slot even on EINTR; never retry a numeric FD
        // that could already be reused. Any failure remains an explicit fault.
        const int owned = std::exchange(s.fd, -1);
        if (close(owned) != 0) {
            s.cleanupFailed = true;
            return eOwnerResourceStatus::EGL_ERROR;
        }
    }
    s.ready = false;
    s.tag = {};
    return eOwnerResourceStatus::OK; // No wait, completion, presentation or GPU drain.
}
