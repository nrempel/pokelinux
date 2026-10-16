// Explicit worker-local frame leaves; no control-side GL resource adoption.
#pragma once

#include "../FramePlan.hpp"
#include <EGL/egl.h>
#include <cstdint>
#include <optional>
#include <thread>

class CShader;

namespace Render::GL {
    class COwnerShmTexture;

    // These borrows exist only inside the graphics owner. They are not mailbox
    // payloads and must never be constructed from control-side GL resources.
    struct SOwnerFrameTarget {
        const SFrameTarget& description;
        uint32_t            framebuffer = 0;
    };

    struct SOwnerFrameShaderKey {
        uint8_t  fragment = UINT8_MAX;
        uint16_t features = 0;
        bool operator==(const SOwnerFrameShaderKey&) const = default;
    };

    struct SOwnerFrameProgram {
        CShader&             shader;
        SOwnerFrameShaderKey key;
    };

    enum class eOwnerFramePrimitiveStatus : uint8_t {
        COMMANDS_ISSUED,
        EMPTY,
        WRONG_OWNER_CONTEXT,
        INVALID_TARGET,
        INVALID_DRAW,
        RESOURCE_MISMATCH,
        SHADER_MISMATCH,
        UNSUPPORTED_MATERIAL,
        GL_ERROR,
    };

    // Only the creating graphics thread may use this object, while this exact
    // display/context is current. No method makes EGL current, compiles shaders,
    // imports/uploads textures, transfers GL ownership, or sends protocol events.
    // Programs are exclusively used by these raw-uniform primitives: do not mix
    // CShader cached setters or the old render path into their program cache.
    // Imported sampler state is also set directly on every draw; do not mix
    // the old cached ITexture parameter setters into this exclusive cache.
    // COMMANDS_ISSUED is deliberately not fence completion, commit or presentation.
    class COwnerFramePrimitives {
      public:
        COwnerFramePrimitives(EGLDisplay display, EGLContext context);
        COwnerFramePrimitives(const COwnerFramePrimitives&) = delete;
        COwnerFramePrimitives& operator=(const COwnerFramePrimitives&) = delete;
        COwnerFramePrimitives(COwnerFramePrimitives&&) = delete;
        COwnerFramePrimitives& operator=(COwnerFramePrimitives&&) = delete;

        // Pure key selection from copied material and the worker import format.
        // Missing/unsupported kinds return no key; CLEAR needs no shader.
        static std::optional<SOwnerFrameShaderKey> shaderKey(const SFrameDraw& draw, bool textureHasAlpha = true);

        eOwnerFramePrimitiveStatus drawClear(const SOwnerFrameTarget& target, const SFrameDraw& draw);
        eOwnerFramePrimitiveStatus drawRect(const SOwnerFrameTarget& target, const SFrameDraw& draw, const SOwnerFrameProgram& program);
        eOwnerFramePrimitiveStatus drawShm(const SOwnerFrameTarget& target, const SFrameDraw& draw, const PrSnapshot* frameSnapshot,
                                           const COwnerShmTexture& imported, const SOwnerFrameProgram& program);

      private:
        const EGLDisplay      m_display;
        const EGLContext      m_context;
        const std::thread::id m_owner;

        std::optional<eOwnerFramePrimitiveStatus> validate(const SOwnerFrameTarget& target, const SFrameDraw& draw) const;
    };
}
