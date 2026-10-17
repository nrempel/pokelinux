#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
#include "render/Texture.hpp"
#include "render/SHMSnapshot.hpp"
#include "helpers/math/Math.hpp"
#include <drm_fourcc.h>
#include <array>
#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <stdexcept>
#include <thread>
#include <utility>
#include <fcntl.h>
#include <sys/mman.h>
#include <unistd.h>

namespace {
constexpr uint32_t WIDTH = 5, HEIGHT = 3, STRIDE = 32;
constexpr size_t BYTES = STRIDE * HEIGHT;
using Bytes = std::array<uint8_t, BYTES>;
struct Failure { const char* name; int code; };
void setup(bool value, const char* name) { if (!value) throw Failure{name, 77}; }
void require(bool value, const char* name) { if (!value) throw Failure{name, 1}; }
void pass(const char* name) { std::printf("CPU_TEXTURE_PASS %s\n", name); }
Bytes pattern(uint8_t seed) {
    Bytes data{};
    for (uint32_t y = 0; y < HEIGHT; ++y) {
        for (uint32_t x = 0; x < WIDTH; ++x) {
            const size_t i = static_cast<size_t>(y) * STRIDE + x * 4;
            data[i] = static_cast<uint8_t>(seed + x * 17 + y * 3);
            data[i + 1] = static_cast<uint8_t>(seed ^ (x * 29 + y * 11));
            data[i + 2] = static_cast<uint8_t>(255 - seed - x * 7 - y * 19);
            data[i + 3] = 255;
        }
        for (uint32_t x = WIDTH * 4; x < STRIDE; ++x)
            data[static_cast<size_t>(y) * STRIDE + x] = static_cast<uint8_t>(seed ^ 0xA5U ^ x ^ (y * 13));
    }
    return data;
}
bool same(const PrShmDescription& a, const PrShmDescription& b) {
    return a.resource_id == b.resource_id && a.generation == b.generation && a.commit_sequence == b.commit_sequence &&
        a.drm_format == b.drm_format && a.width == b.width && a.height == b.height && a.stride == b.stride &&
        a.bytes_per_pixel == b.bytes_per_pixel && a.reserved == b.reserved;
}
void exact(const Render::CSHMSnapshot& snapshot, const PrShmDescription& expected, const Bytes& data, const char* name) {
    const auto view = snapshot.view();
    require(view && view->pixels && view->pixel_bytes == BYTES && same(view->description, expected), name);
    require(std::memcmp(view->pixels, data.data(), BYTES) == 0, name);
}
struct Producer {
    int fd = -1;
    uint8_t* pixels = nullptr;
    Bytes expected{};
    explicit Producer(uint8_t seed) : expected(pattern(seed)) {
        fd = memfd_create("pokelinux-real-cpu-texture", MFD_CLOEXEC);
        if (fd < 0)
            throw Failure{"memfd_create", 77};
        const int flags = fcntl(fd, F_GETFD);
        if (flags < 0 || !(flags & FD_CLOEXEC) || ftruncate(fd, static_cast<off_t>(BYTES))) {
            (void)close(fd);
            fd = -1;
            throw Failure{"memfd_shape_or_CLOEXEC", 77};
        }
        void* mapping = mmap(nullptr, BYTES, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
        if (mapping == MAP_FAILED) {
            (void)close(fd);
            fd = -1;
            throw Failure{"producer_mmap", 77};
        }
        pixels = static_cast<uint8_t*>(mapping);
        std::memcpy(pixels, expected.data(), BYTES);
    }
    ~Producer() {
        if (pixels) (void)munmap(pixels, BYTES);
        if (fd >= 0) (void)close(fd);
    }
    Producer(const Producer&) = delete;
    Producer& operator=(const Producer&) = delete;
    void dispose() {
        setup(pixels && fd >= 0, "producer_disposal_state");
        std::memset(pixels, 0xEE, BYTES);
        setup(munmap(pixels, BYTES) == 0, "producer_munmap");
        pixels = nullptr;
        const int old = std::exchange(fd, -1);
        setup(close(old) == 0, "producer_close");
        errno = 0;
        setup(fcntl(old, F_GETFD) == -1 && errno == EBADF, "producer_actually_closed");
    }
};
Render::CSHMSnapshot capture(Producer& producer, uint32_t format = DRM_FORMAT_ARGB8888, uint32_t bpp = 4) {
    PrShmDescription description{};
    description.drm_format = format;
    description.width = WIDTH;
    description.height = HEIGHT;
    description.stride = STRIDE;
    description.bytes_per_pixel = bpp;
    Render::CSHMSnapshot result;
    setup(Render::CSHMSnapshot::capture(description, producer.pixels, BYTES, result) == PR_OK, "actual_Rust_capture");
    const auto view = result.view();
    setup(view && view->pixels != producer.pixels && view->pixel_bytes == BYTES && view->description.resource_id &&
          view->description.generation && view->description.commit_sequence, "actual_owned_Rust_snapshot");
    return result;
}
const Render::CSHMSnapshot& borrowed(Render::ITexture& texture) {
    const auto* snapshot = Render::borrowCPUTextureSnapshot(texture);
    require(snapshot != nullptr, "creator_thread_typed_borrow");
    return *snapshot; // synchronous; never retain this reference across replacement/destruction
}
template<class Fn> void refusedLogic(Fn&& call, const char* name) {
    bool refused = false;
    try { call(); } catch (const std::logic_error&) { refused = true; }
    require(refused, name);
}
template<class Fn> void refusedParameter(Fn&& call, const char* name) {
    bool refused = false;
    try { call(); } catch (const std::invalid_argument&) { refused = true; }
    require(refused, name);
}
void metadata(Render::ITexture& texture, uint32_t format, bool opaque, Render::eTextureType type) {
    require(texture.ok() && !texture.isDMA() && texture.m_texID == 0 && texture.m_isSynchronous &&
            texture.m_size == Vector2D{5.0, 3.0} && texture.m_drmFormat == format && texture.m_opaque == opaque && texture.m_type == type &&
            texture.m_transform == HYPRUTILS_TRANSFORM_NORMAL && !texture.m_imageDescription,
            "actual_CPU_texture_metadata_without_GPU_identity");
}
} // namespace

int main() {
    (void)std::setvbuf(stdout, nullptr, _IONBF, 0);
    try {
        Producer first(0x23);
        auto firstInput = capture(first);
        const auto firstDescription = firstInput.view()->description;
        SP<Render::ITexture> texture;
        require(Render::createCPUTexture(firstInput, false, texture) == PR_OK && texture, "actual_CPU_factory_success");
        firstInput.reset();
        first.dispose();
        metadata(*texture, DRM_FORMAT_ARGB8888, false, Render::TEXTURE_RGBA);
        exact(borrowed(*texture), firstDescription, first.expected, "factory_owns_exact_padded_96_bytes_after_producer_disposal");
        Render::CSHMSnapshot admittedOldClone = borrowed(*texture); // actual separately owned Rust handle
        require(texture->magFilter == GL_LINEAR && texture->minFilter == GL_LINEAR, "default_linear_sampler_metadata");
        pass("actual_Rust_pixels_after_producer_disposal");

        SP<Render::ITexture> argbOpaque;
        require(Render::createCPUTexture(admittedOldClone, true, argbOpaque) == PR_OK && argbOpaque, "ARGB_explicit_opaque_factory");
        metadata(*argbOpaque, DRM_FORMAT_ARGB8888, true, Render::TEXTURE_RGBA);
        exact(borrowed(*argbOpaque), firstDescription, first.expected, "ARGB_explicit_opacity_does_not_rewrite_pixels");
        Producer xrProducer(0x47);
        auto xrInput = capture(xrProducer, DRM_FORMAT_XRGB8888);
        const auto xrDescription = xrInput.view()->description;
        SP<Render::ITexture> xr;
        require(Render::createCPUTexture(xrInput, false, xr) == PR_OK && xr, "XR24_factory_explicit_false_opacity");
        xrInput.reset();
        xrProducer.dispose();
        metadata(*xr, DRM_FORMAT_XRGB8888, false, Render::TEXTURE_RGBX);
        exact(borrowed(*xr), xrDescription, xrProducer.expected, "XR24_preserves_all_pixel_and_padding_bytes");
        pass("explicit_opacity_independent_of_AR24_XR24_type");

        texture->setTexParameter(GL_TEXTURE_MAG_FILTER, GL_NEAREST);
        texture->setTexParameter(GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        require(texture->magFilter == GL_NEAREST && texture->minFilter == GL_LINEAR && texture->m_texID == 0, "actual_sampler_metadata_change");
        refusedParameter([&] { texture->setTexParameter(GL_TEXTURE_MIN_FILTER, GL_NEAREST_MIPMAP_NEAREST); }, "mipmap_filter_refused");
        refusedParameter([&] { texture->setTexParameter(GL_TEXTURE_SWIZZLE_R, GL_BLUE); }, "native_swizzle_refused");
        const auto creatorTid = gettid();
        pid_t foreignTid = 0;
        bool foreignBorrowRefused = false, foreignSamplerRefused = false;
        SP<Render::ITexture> foreignOwnedTexture;
        Render::CSHMSnapshot foreignOwnedWitness;
        PrShmDescription foreignOwnedDescription{};
        int32_t foreignFactoryCode = PR_INVALID;
        std::thread::id retiredCppIdentity{};
        auto inputForForeign = admittedOldClone; // separately owned real Rust handle
        auto* const rawTexture = texture.get();
        std::thread foreign([&, ownedInput = std::move(inputForForeign)] {
            foreignTid = gettid();
            retiredCppIdentity = std::this_thread::get_id();
            foreignBorrowRefused = Render::borrowCPUTextureSnapshot(*rawTexture) == nullptr;
            try { rawTexture->setTexParameter(GL_TEXTURE_MAG_FILTER, GL_LINEAR); }
            catch (const std::logic_error&) { foreignSamplerRefused = true; }
            foreignFactoryCode = Render::createCPUTexture(ownedInput, true, foreignOwnedTexture);
            if (foreignFactoryCode == PR_OK && foreignOwnedTexture) {
                const auto* actual = Render::borrowCPUTextureSnapshot(*foreignOwnedTexture);
                if (actual) {
                    const auto view = actual->view();
                    if (view) {
                        foreignOwnedDescription = view->description;
                        foreignOwnedWitness = *actual; // genuine clone made on creator thread
                    }
                }
            }
        });
        foreign.join();
        require(foreignTid > 0 && foreignTid != creatorTid && foreignBorrowRefused && foreignSamplerRefused &&
                texture->magFilter == GL_NEAREST && texture->minFilter == GL_LINEAR, "actual_creator_thread_refusals_preserve_metadata");
        exact(borrowed(*texture), firstDescription, first.expected, "sampler_changes_preserve_owned_pixels_and_identity");
        require(foreignFactoryCode == PR_OK && foreignOwnedTexture && foreignOwnedWitness &&
                same(foreignOwnedDescription, firstDescription), "real_foreign_thread_CPU_factory_and_owned_snapshot");
        exact(foreignOwnedWitness, firstDescription, first.expected, "actual_foreign_CPU_pixels_before_output_refusal");
        auto foreignHandleBefore = foreignOwnedTexture;
        // First creator has exited, but its real CPU object is still alive.
        // A successor must not reactivate it even if std::thread::id is reused.
        pid_t successorTid = 0;
        bool successorCppIdentityReused = false;
        bool successorBorrowRefused = false, successorSamplerRefused = false;
        int32_t successorFactoryCode = PR_OK;
        Render::CSHMSnapshot successorInput = borrowed(*xr);
        auto* const retiredOwnerTexture = foreignOwnedTexture.get();
        std::thread successor([&, ownedInput = std::move(successorInput)] {
            successorTid = gettid();
            successorCppIdentityReused = std::this_thread::get_id() == retiredCppIdentity;
            successorBorrowRefused = Render::borrowCPUTextureSnapshot(*retiredOwnerTexture) == nullptr;
            try { retiredOwnerTexture->setTexParameter(GL_TEXTURE_MAG_FILTER, GL_NEAREST); }
            catch (const std::logic_error&) { successorSamplerRefused = true; }
            successorFactoryCode = Render::createCPUTexture(ownedInput, false, foreignOwnedTexture);
        });
        successor.join();
        require(successorTid > 0 && successorTid != creatorTid && successorTid != foreignTid &&
                successorBorrowRefused && successorSamplerRefused && successorFactoryCode == PR_INVALID &&
                foreignOwnedTexture == foreignHandleBefore, "retired_creator_never_reactivated_by_successor_thread");
        require(retiredOwnerTexture->m_size == Vector2D{5.0, 3.0} && retiredOwnerTexture->m_drmFormat == DRM_FORMAT_ARGB8888 &&
                retiredOwnerTexture->m_type == Render::TEXTURE_RGBA && retiredOwnerTexture->m_opaque &&
                retiredOwnerTexture->m_isSynchronous && retiredOwnerTexture->m_texID == 0 &&
                retiredOwnerTexture->magFilter == GL_LINEAR && retiredOwnerTexture->minFilter == GL_LINEAR &&
                retiredOwnerTexture->m_transform == HYPRUTILS_TRANSFORM_NORMAL && !retiredOwnerTexture->m_imageDescription,
                "successor_refusals_preserve_retired_owner_metadata");
        exact(foreignOwnedWitness, firstDescription, first.expected, "successor_refusals_preserve_actual_owned_snapshot");
        std::printf("CPU_TEXTURE_OBSERVATION successor_tid=%d retired_creator_tid=%d actual_thread_joined=true cpp_identity_reused=%s\n",
                    successorTid, foreignTid, successorCppIdentityReused ? "true" : "false");
        // Different valid XR24 input and explicit opacityfalse would expose an
        // illegal in-place replacement of the foreign AR24/opacitytrue object.
        require(Render::createCPUTexture(borrowed(*xr), false, foreignOwnedTexture) == PR_INVALID &&
                foreignOwnedTexture == foreignHandleBefore, "foreign_owner_output_refusal_preserves_identical_handle");
        require(foreignOwnedTexture->m_size == Vector2D{5.0, 3.0} && foreignOwnedTexture->m_drmFormat == DRM_FORMAT_ARGB8888 &&
                foreignOwnedTexture->m_type == Render::TEXTURE_RGBA && foreignOwnedTexture->m_opaque &&
                foreignOwnedTexture->m_isSynchronous && foreignOwnedTexture->m_texID == 0 &&
                foreignOwnedTexture->magFilter == GL_LINEAR && foreignOwnedTexture->minFilter == GL_LINEAR &&
                foreignOwnedTexture->m_transform == HYPRUTILS_TRANSFORM_NORMAL && !foreignOwnedTexture->m_imageDescription,
                "foreign_owner_refusal_preserves_actual_metadata");
        require(Render::borrowCPUTextureSnapshot(*foreignOwnedTexture) == nullptr, "foreign_owner_borrow_still_refused_after_join");
        // Main cannot reborrow the exited creator's texture. These bytes are
        // observed through its genuine separately owned creator-thread clone.
        exact(foreignOwnedWitness, firstDescription, first.expected, "foreign_owner_refusal_preserves_owned_pixels_and_identity");
        foreignOwnedTexture.reset();
        foreignHandleBefore.reset(); // actual CPU destructor on main: Rust release only
        exact(foreignOwnedWitness, firstDescription, first.expected, "foreign_CPU_destructor_leaves_owned_snapshot_alive");
        foreignOwnedWitness.reset();
        std::printf("CPU_TEXTURE_OBSERVATION creator_tid=%d foreign_tid=%d actual_thread_joined=true\n", creatorTid, foreignTid);
        pass("sampler_metadata_and_creator_thread_refusals");

        auto oldTexture = texture;
        Producer replacement(0x91);
        auto newInput = capture(replacement);
        const auto replacementDescription = newInput.view()->description;
        require(Render::createCPUTexture(newInput, false, texture) == PR_OK && texture && texture != oldTexture,
                "factory_replacement_is_a_new_immutable_CPU_object");
        newInput.reset();
        replacement.dispose();
        require(replacementDescription.resource_id > firstDescription.resource_id && replacementDescription.generation > firstDescription.generation &&
                replacementDescription.commit_sequence > firstDescription.commit_sequence, "fresh_actual_import_identity");
        metadata(*texture, DRM_FORMAT_ARGB8888, false, Render::TEXTURE_RGBA);
        exact(borrowed(*texture), replacementDescription, replacement.expected, "replacement_owns_new_exact_bytes");
        exact(borrowed(*oldTexture), firstDescription, first.expected, "old_CPU_object_remains_immutable");
        oldTexture.reset();
        exact(admittedOldClone, firstDescription, first.expected, "admitted_old_Rust_clone_survives_old_object_release");
        pass("fresh_factory_replacement_keeps_old_admitted_clone_stable");

        const CRegion partialDamage{CBox{1.0, 0.0, 1.0, 1.0}};
        const CRegion emptyDamage;
        refusedLogic([&] { texture->bind(); }, "native_bind_refused");
        refusedLogic([&] { texture->unbind(); }, "native_unbind_refused");
        refusedLogic([&] { texture->allocate(Vector2D{7.0, 4.0}, DRM_FORMAT_ARGB8888); }, "native_allocate_refused");
        refusedLogic([&] { texture->update(DRM_FORMAT_ARGB8888, first.expected.data(), STRIDE, partialDamage); }, "legacy_partial_update_refused");
        refusedLogic([&] { texture->update(DRM_FORMAT_ARGB8888, first.expected.data(), STRIDE, emptyDamage); }, "legacy_empty_update_explicitly_refused");
        exact(borrowed(*texture), replacementDescription, replacement.expected, "all_legacy_refusals_preserve_snapshot_and_identity");
        metadata(*texture, DRM_FORMAT_ARGB8888, false, Render::TEXTURE_RGBA);
        const auto beforeRefusal = texture;
        Render::CSHMSnapshot empty;
        require(Render::createCPUTexture(empty, true, texture) == PR_INVALID && texture == beforeRefusal, "empty_factory_refusal_preserves_output");
        Producer unsupported(0x61);
        auto unsupportedInput = capture(unsupported, DRM_FORMAT_RGB565, 2);
        require(Render::createCPUTexture(unsupportedInput, false, texture) == PR_INVALID && texture == beforeRefusal,
                "unsupported_real_SHM_format_factory_refusal_preserves_output");
        unsupportedInput.reset();
        unsupported.dispose();
        exact(borrowed(*texture), replacementDescription, replacement.expected, "factory_refusals_preserve_current_pixels");
        texture.reset();
        argbOpaque.reset();
        xr.reset();
        exact(admittedOldClone, firstDescription, first.expected, "separately_owned_snapshot_survives_texture_release");
        admittedOldClone.reset();
        pass("legacy_native_and_unsupported_factory_refusals");
        std::printf("CPU_TEXTURE_ALL_PASS groups=5\n");
        return 0;
    } catch (const Failure& failure) {
        std::printf("CPU_TEXTURE_%s %s\n", failure.code == 77 ? "SETUP_UNAVAILABLE" : "FAIL", failure.name);
        return failure.code;
    } catch (const std::exception& error) {
        std::printf("CPU_TEXTURE_SETUP_UNAVAILABLE exception=%s\n", error.what());
        return 77;
    } catch (...) {
        std::printf("CPU_TEXTURE_SETUP_UNAVAILABLE nonstandard_exception\n");
        return 77;
    }
}
