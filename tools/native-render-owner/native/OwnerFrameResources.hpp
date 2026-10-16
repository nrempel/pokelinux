#pragma once
// Explicit worker-owned immutable SHM and native-fence resources.
#include "../pokelinux_render.h"
#include <EGL/egl.h>
#include <cstdint>
#include <memory>

namespace Render::GL {
    enum class eOwnerResourceStatus : uint8_t {
        OK, WRONG_OWNER_CONTEXT, INVALID_INPUT, RESOURCE_MISMATCH,
        UNSUPPORTED_FORMAT, BUSY, ALLOCATION, GL_ERROR, EGL_ERROR, NOT_READY,
    };
    struct SOwnerTextureInfo {
        PrShmDescription description{};
        uint32_t textureName = 0; // worker-local diagnostic borrow, never transport/adoption authority
        bool hasAlpha = false;
    };
    struct SOwnerFenceTag {
        uint64_t admissionId=0,sceneSequence=0,securityGeneration=0;
        uint64_t outputId=0,outputGeneration=0,fenceSequence=0;
        bool operator==(const SOwnerFenceTag&) const = default;
    };
    struct SOwnerFenceFD {
        int32_t fd=-1; // borrowed until retire; duplicate explicitly for independent custody
        SOwnerFenceTag tag{};
    };
    // Empty constructors acquire no GL/EGL/snapshot/FD custody. Record current
    // construction thread plus explicit display/context, never make current.
    // Native acquisitions occur only after this object is retained by its owner.
    // All operations first refuse wrong thread/context without touching mutable
    // resource state or output arguments. Objects never copy/move between owners.
    class COwnerShmTexture {
      public:
        COwnerShmTexture(EGLDisplay display,EGLContext context);
        ~COwnerShmTexture() noexcept; // CPU-only after retire; destroying live custody is a lifetime violation
        COwnerShmTexture(const COwnerShmTexture&)=delete;
        COwnerShmTexture& operator=(const COwnerShmTexture&)=delete;
        COwnerShmTexture(COwnerShmTexture&&)=delete;
        COwnerShmTexture& operator=(COwnerShmTexture&&)=delete;
        // Borrow valid actual Rust snapshot; retain ONE explicit clone BEFORE GL.
        // Never retain the caller's handle slot/view/callback/client mapping.
        // Only AR24/XR24 initially. Preserve exact stride/padding interpretation;
        // save/set/restore touched texture/unpack/PBO state. Existing live import
        // returns BUSY. Partial failure retains clone/name for owner cleanup.
        eOwnerResourceStatus upload(const PrSnapshot* snapshot) noexcept;
        // Current owner only. Copy actual descriptor/name/type after successful
        // command issue, not GPU completion. Failure leaves output unchanged.
        eOwnerResourceStatus describe(SOwnerTextureInfo& output) const noexcept;
        // Compare actual Rust views: whole descriptor, pixel count AND same live
        // immutable allocation as retained clone. Same-allocation clone accepts;
        // copied identical metadata on a different allocation rejects.
        eOwnerResourceStatus matches(const PrSnapshot* snapshot) const noexcept;
        // Idempotent on empty; correct owner/context required even then. Does not
        // wait for GPU completion. Caller must retain until its genuine fence fact.
        // Partial/failed cleanup retains remaining custody; never fake retirement.
        eOwnerResourceStatus retire() noexcept;
      private:
        struct SImpl;
        std::unique_ptr<SImpl> m_impl;
    };
    class COwnerNativeFence {
      public:
        COwnerNativeFence(EGLDisplay display,EGLContext context);
        ~COwnerNativeFence() noexcept; // CPU-only after retire; no destructive auto-unwind
        COwnerNativeFence(const COwnerNativeFence&)=delete;
        COwnerNativeFence& operator=(const COwnerNativeFence&)=delete;
        COwnerNativeFence(COwnerNativeFence&&)=delete;
        COwnerNativeFence& operator=(COwnerNativeFence&&)=delete;
        // All copied tag fields nonzero. One live fence; BUSY does not mutate it.
        // Reissue after retire requires strictly increasing fenceSequence.
        // create native fence/null attrs -> glFlush -> export FD, in that order.
        // Store acquired sync/FD immediately; partial failures remain owner-held.
        // Enforce CLOEXEC. Success only means issued/exported, not completion.
        eOwnerResourceStatus issue(const SOwnerFenceTag& tag) noexcept;
        eOwnerResourceStatus borrowedFD(SOwnerFenceFD& output) const noexcept;
        eOwnerResourceStatus retire() noexcept;
      private:
        struct SImpl;
        std::unique_ptr<SImpl> m_impl;
    };
}
