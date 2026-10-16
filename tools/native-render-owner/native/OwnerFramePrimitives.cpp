#include "OwnerFramePrimitives.hpp"
#include "../OpenGL.hpp" // existing fullVerts/SVertex layout and shader definitions only
#include "OwnerFrameResources.hpp"
#include <algorithm>
#include <cmath>
#include <limits>

using namespace Render;
using namespace Render::GL;

static bool finiteValues(const auto& values) {
    return std::ranges::all_of(values, [](const auto value) { return std::isfinite(value); });
}

static bool finiteBox(const SFrameBox& box) {
    return std::isfinite(box.x) && std::isfinite(box.y) && std::isfinite(box.width) && std::isfinite(box.height) && std::isfinite(box.rotation);
}

static bool validCM(const SFrameCM& cm) {
    if (!cm.enabled)
        return !cm.needsTonemap && !cm.needsSDRmod;

    const std::array values = {cm.srcTFMin, cm.srcTFMax, cm.dstTFMin, cm.dstTFMax, cm.srcRefLuminance, cm.dstRefLuminance, cm.maxLuminance,
                               cm.dstMaxLuminance, cm.sdrSaturation, cm.sdrBrightness};
    return finiteValues(values) && finiteValues(cm.convertMatrix) && finiteValues(cm.dstPrimariesXYZ) && cm.srcTFMax > cm.srcTFMin && cm.dstTFMax > cm.dstTFMin &&
        cm.srcRefLuminance > 0 && cm.dstRefLuminance > 0 && cm.maxLuminance > 0 && cm.dstMaxLuminance > 0 && cm.sdrSaturation >= 0 && cm.sdrBrightness >= 0 &&
        cm.sourceTF <= sc<uint32_t>(std::numeric_limits<GLint>::max()) && cm.targetTF <= sc<uint32_t>(std::numeric_limits<GLint>::max()) &&
        (!cm.needsTonemap || (cm.tonemapMode >= 1 && cm.tonemapMode <= 3));
}

static bool colorOnlyMaterial(const SFrameMaterial& material) {
    return !material.cm.enabled && !material.cm.needsTonemap && !material.cm.needsSDRmod && !material.tintEnabled && !material.discardActive;
}

static bool sameDescription(const PrShmDescription& a, const PrShmDescription& b) {
    return a.resource_id == b.resource_id && a.generation == b.generation && a.commit_sequence == b.commit_sequence && a.drm_format == b.drm_format && a.width == b.width &&
        a.height == b.height && a.stride == b.stride && a.bytes_per_pixel == b.bytes_per_pixel && a.reserved == b.reserved;
}

// Preserve projectBoxToTarget's row-major/transpose=true convention while using
// the transpose=false upload required by GLES. No geometry is recomputed here.
static FrameMatrix projectionForGL(const FrameMatrix& matrix) {
    return {matrix[0], matrix[3], matrix[6], matrix[1], matrix[4], matrix[7], matrix[2], matrix[5], matrix[8]};
}

// Raw uniforms intentionally avoid CShader's setter cache/GLCALL diagnostics.
// The exact existing shader uniform locations and shader arithmetic are retained.
static void uniformInt(const CShader& shader, eShaderUniform name, GLint value) {
    glUniform1i(shader.getUniformLocation(name), value);
}

static void uniformFloat(const CShader& shader, eShaderUniform name, GLfloat value) {
    glUniform1f(shader.getUniformLocation(name), value);
}

static void uniformFloat2(const CShader& shader, eShaderUniform name, GLfloat x, GLfloat y) {
    glUniform2f(shader.getUniformLocation(name), x, y);
}

static void uniformFloat3(const CShader& shader, eShaderUniform name, const std::array<float, 3>& value) {
    glUniform3fv(shader.getUniformLocation(name), 1, value.data());
}

static void uniformFloat4(const CShader& shader, eShaderUniform name, const std::array<float, 4>& value) {
    glUniform4fv(shader.getUniformLocation(name), 1, value.data());
}

static void uniformMatrix(const CShader& shader, eShaderUniform name, const FrameMatrix& value) {
    glUniformMatrix3fv(shader.getUniformLocation(name), 1, GL_FALSE, value.data());
}

static void applyCM(const CShader& shader, const SFrameCM& cm) {
    uniformInt(shader, SHADER_SOURCE_TF, sc<GLint>(cm.sourceTF));
    uniformInt(shader, SHADER_TARGET_TF, sc<GLint>(cm.targetTF));
    uniformFloat2(shader, SHADER_SRC_TF_RANGE, cm.srcTFMin, cm.srcTFMax);
    uniformFloat2(shader, SHADER_DST_TF_RANGE, cm.dstTFMin, cm.dstTFMax);
    uniformFloat(shader, SHADER_SRC_REF_LUMINANCE, cm.srcRefLuminance);
    uniformFloat(shader, SHADER_DST_REF_LUMINANCE, cm.dstRefLuminance);
    uniformFloat(shader, SHADER_MAX_LUMINANCE, cm.maxLuminance);
    uniformFloat(shader, SHADER_DST_MAX_LUMINANCE, cm.dstMaxLuminance);
    uniformInt(shader, SHADER_TONEMAP_MODE, sc<GLint>(cm.tonemapMode));
    uniformFloat(shader, SHADER_SDR_SATURATION, cm.sdrSaturation);
    uniformFloat(shader, SHADER_SDR_BRIGHTNESS, cm.sdrBrightness);
    // CPU preparation copied the same column-major arrays as passCMUniforms.
    uniformMatrix(shader, SHADER_CONVERT_MATRIX, cm.convertMatrix);
    uniformMatrix(shader, SHADER_TARGET_PRIMARIES_XYZ, cm.dstPrimariesXYZ);
}

static bool validProgram(const SOwnerFrameProgram& program, const SOwnerFrameShaderKey& expected) {
    return program.key == expected && program.shader.program() != 0 && program.shader.getUniformLocation(SHADER_SHADER_VAO) > 0 &&
        program.shader.getUniformLocation(SHADER_SHADER_VBO) > 0 && program.shader.getUniformLocation(SHADER_PROJ) >= 0;
}

static void bindTarget(const SOwnerFrameTarget& target, bool blend) {
    glBindFramebuffer(GL_DRAW_FRAMEBUFFER, target.framebuffer);
    glViewport(0, 0, sc<GLsizei>(target.description.width), sc<GLsizei>(target.description.height));
    // Mirrors/MRT, stencil/effects and final output shaders are outside this
    // subset and must be rejected by CPU preparation before admission.
    const GLenum attachment = GL_COLOR_ATTACHMENT0;
    glDrawBuffers(1, &attachment);
    glDisable(GL_STENCIL_TEST);
    glDisable(GL_DEPTH_TEST);
    glDepthMask(GL_FALSE);
    glColorMask(GL_TRUE, GL_TRUE, GL_TRUE, GL_TRUE);
    glBlendEquation(GL_FUNC_ADD);
    glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA); // existing premultiplied contract
    if (blend)
        glEnable(GL_BLEND);
    else
        glDisable(GL_BLEND);
    glEnable(GL_SCISSOR_TEST);
}

static void applyRounding(const CShader& shader, const SFrameDraw& draw) {
    uniformFloat2(shader, SHADER_TOP_LEFT, sc<float>(draw.roundingBox.x), sc<float>(draw.roundingBox.y));
    uniformFloat2(shader, SHADER_FULL_SIZE, sc<float>(draw.roundingBox.width), sc<float>(draw.roundingBox.height));
    uniformFloat(shader, SHADER_RADIUS, draw.material.roundingPx);
    uniformFloat(shader, SHADER_ROUNDING_POWER, draw.material.roundingPower);
}

static void drawScissors(const SFrameDraw& draw) {
    // Already resolved using the original clip-vs-damage branch and transformed
    // to final GL target coordinates on control. Never intersect/transform again.
    for (const auto& rect : draw.scissors) {
        glScissor(rect.x1, rect.y1, rect.x2 - rect.x1, rect.y2 - rect.y1);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    }
}

COwnerFramePrimitives::COwnerFramePrimitives(EGLDisplay display, EGLContext context) : m_display(display), m_context(context), m_owner(std::this_thread::get_id()) {}

std::optional<eOwnerFramePrimitiveStatus> COwnerFramePrimitives::validate(const SOwnerFrameTarget& target, const SFrameDraw& draw) const {
    if (std::this_thread::get_id() != m_owner || m_display == EGL_NO_DISPLAY || m_context == EGL_NO_CONTEXT || eglGetCurrentDisplay() != m_display ||
        eglGetCurrentContext() != m_context)
        return eOwnerFramePrimitiveStatus::WRONG_OWNER_CONTEXT;

    const auto& desc = target.description;
    if (target.framebuffer == 0 || desc.outputGeneration == 0 || desc.width == 0 || desc.height == 0 || desc.transformedWidth == 0 || desc.transformedHeight == 0 ||
        desc.width > sc<uint32_t>(std::numeric_limits<GLsizei>::max()) || desc.height > sc<uint32_t>(std::numeric_limits<GLsizei>::max()) || desc.outputTransform > 7 ||
        !std::isfinite(desc.scale) || desc.scale <= 0 || !finiteValues(desc.targetProjection) || !finiteValues(desc.scaleMatrix))
        return eOwnerFramePrimitiveStatus::INVALID_TARGET;

    if (draw.scissors.size() > FRAME_MAX_SCISSORS)
        return eOwnerFramePrimitiveStatus::INVALID_DRAW;
    if (draw.material.cm.enabled && !desc.shaderCMAvailable)
        return eOwnerFramePrimitiveStatus::UNSUPPORTED_MATERIAL;

    for (const auto& rect : draw.scissors) {
        if (rect.x1 < 0 || rect.y1 < 0 || rect.x2 <= rect.x1 || rect.y2 <= rect.y1 || sc<uint32_t>(rect.x2) > desc.width || sc<uint32_t>(rect.y2) > desc.height)
            return eOwnerFramePrimitiveStatus::INVALID_DRAW;
    }

    if (draw.kind != eFrameDrawKind::CLEAR && (!finiteBox(draw.box) || draw.box.width <= 0 || draw.box.height <= 0 || !finiteValues(draw.projection)))
        return eOwnerFramePrimitiveStatus::INVALID_DRAW;
    if (!std::isfinite(draw.material.alpha) || !std::isfinite(draw.material.roundingPx) || draw.material.roundingPx < 0 || !std::isfinite(draw.material.roundingPower) ||
        draw.material.roundingPower <= 0 || !std::isfinite(draw.material.discardOpacity) || !finiteValues(draw.material.tintRgb) || !validCM(draw.material.cm))
        return eOwnerFramePrimitiveStatus::INVALID_DRAW;
    if (draw.material.roundingPx > 0 &&
        (!finiteBox(draw.roundingBox) || draw.roundingBox.width <= 0 || draw.roundingBox.height <= 0 ||
         !finiteValues(std::array{sc<float>(draw.roundingBox.x), sc<float>(draw.roundingBox.y), sc<float>(draw.roundingBox.width), sc<float>(draw.roundingBox.height)})))
        return eOwnerFramePrimitiveStatus::INVALID_DRAW;

    return std::nullopt;
}

std::optional<SOwnerFrameShaderKey> COwnerFramePrimitives::shaderKey(const SFrameDraw& draw, bool textureHasAlpha) {
    ShaderFeatureFlags features = 0;
    if (draw.material.roundingPx > 0)
        features |= SH_FEAT_ROUNDING;

    if (draw.kind == eFrameDrawKind::RECT) {
        if (!colorOnlyMaterial(draw.material))
            return std::nullopt;
        return SOwnerFrameShaderKey{.fragment = SH_FRAG_QUAD, .features = features};
    }
    if (draw.kind != eFrameDrawKind::SHM || draw.material.wrapX > 1 || draw.material.wrapY > 1)
        return std::nullopt;

    if (textureHasAlpha && !draw.material.rgbx)
        features |= SH_FEAT_RGBA;
    if (draw.material.discardActive)
        features |= SH_FEAT_DISCARD;
    if (draw.material.tintEnabled)
        features |= SH_FEAT_TINT;
    if (draw.material.cm.enabled) {
        features |= SH_FEAT_CM;
        if (draw.material.cm.needsTonemap) {
            features |= SH_FEAT_TONEMAP;
            if (draw.material.cm.tonemapMode == 3)
                features |= SH_FEAT_ALT_TONEMAP;
        }
        if (draw.material.cm.needsSDRmod)
            features |= SH_FEAT_SDR_MOD;
    }
    return SOwnerFrameShaderKey{.fragment = SH_FRAG_SURFACE, .features = features};
}

eOwnerFramePrimitiveStatus COwnerFramePrimitives::drawClear(const SOwnerFrameTarget& target, const SFrameDraw& draw) {
    if (const auto failure = validate(target, draw))
        return *failure;
    if (draw.kind != eFrameDrawKind::CLEAR || !finiteValues(draw.convertedColor))
        return eOwnerFramePrimitiveStatus::INVALID_DRAW;
    if (!colorOnlyMaterial(draw.material) || draw.material.roundingPx != 0)
        return eOwnerFramePrimitiveStatus::UNSUPPORTED_MATERIAL;
    if (draw.scissors.empty())
        return eOwnerFramePrimitiveStatus::EMPTY;
    if (glGetError() != GL_NO_ERROR)
        return eOwnerFramePrimitiveStatus::GL_ERROR;

    bindTarget(target, false);
    for (const auto& rect : draw.scissors) {
        glScissor(rect.x1, rect.y1, rect.x2 - rect.x1, rect.y2 - rect.y1);
        // CPU preparation already applied the original clear color conversion.
        // Clear does not introduce the RECT premultiplication step.
        glClearBufferfv(GL_COLOR, 0, draw.convertedColor.data());
    }
    glDisable(GL_SCISSOR_TEST);
    return glGetError() == GL_NO_ERROR ? eOwnerFramePrimitiveStatus::COMMANDS_ISSUED : eOwnerFramePrimitiveStatus::GL_ERROR;
}

eOwnerFramePrimitiveStatus COwnerFramePrimitives::drawRect(const SOwnerFrameTarget& target, const SFrameDraw& draw, const SOwnerFrameProgram& program) {
    if (const auto failure = validate(target, draw))
        return *failure;
    if (draw.kind != eFrameDrawKind::RECT || !finiteValues(draw.convertedColor) || !finiteValues(draw.colorSRGB))
        return eOwnerFramePrimitiveStatus::INVALID_DRAW;
    const auto key = shaderKey(draw);
    if (!key)
        return eOwnerFramePrimitiveStatus::UNSUPPORTED_MATERIAL;
    if (!validProgram(program, *key) || program.shader.getUniformLocation(SHADER_COLOR) < 0)
        return eOwnerFramePrimitiveStatus::SHADER_MISMATCH;
    if (draw.scissors.empty())
        return eOwnerFramePrimitiveStatus::EMPTY;
    if (glGetError() != GL_NO_ERROR)
        return eOwnerFramePrimitiveStatus::GL_ERROR;

    bindTarget(target, draw.material.blend);
    glUseProgram(program.shader.program());
    uniformMatrix(program.shader, SHADER_PROJ, projectionForGL(draw.projection));
    // Both are already premultiplied on control, and the first is converted to
    // the copied target space. Do not multiply material.alpha a second time.
    uniformFloat4(program.shader, SHADER_COLOR, draw.convertedColor);
    uniformFloat4(program.shader, SHADER_COLOR_SRGB, draw.colorSRGB);
    applyRounding(program.shader, draw);
    glBindVertexArray(program.shader.getUniformLocation(SHADER_SHADER_VAO));
    drawScissors(draw);
    glBindVertexArray(0);
    glDisable(GL_SCISSOR_TEST);
    return glGetError() == GL_NO_ERROR ? eOwnerFramePrimitiveStatus::COMMANDS_ISSUED : eOwnerFramePrimitiveStatus::GL_ERROR;
}

eOwnerFramePrimitiveStatus COwnerFramePrimitives::drawShm(const SOwnerFrameTarget& target, const SFrameDraw& draw, const PrSnapshot* frameSnapshot,
                                                        const COwnerShmTexture& imported, const SOwnerFrameProgram& program) {
    if (const auto failure = validate(target, draw))
        return *failure;
    if (draw.kind != eFrameDrawKind::SHM || draw.resourceIndex >= FRAME_MAX_RESOURCES || !finiteValues(draw.uv))
        return eOwnerFramePrimitiveStatus::INVALID_DRAW;

    PrSnapshotView view{};
    SOwnerTextureInfo texture{};
    if (imported.matches(frameSnapshot) != eOwnerResourceStatus::OK || imported.describe(texture) != eOwnerResourceStatus::OK ||
        pr_snapshot_view(frameSnapshot, &view) != PR_OK || !view.pixels || !sameDescription(texture.description, view.description) ||
        draw.commitSequence != view.description.commit_sequence || view.pixel_bytes != sc<uint64_t>(view.description.stride) * view.description.height)
        return eOwnerFramePrimitiveStatus::RESOURCE_MISMATCH;

    // The opaque owner cache proves the same retained Rust allocation and full
    // descriptor. A caller-created GLuint/metadata pair cannot fabricate import.
    if (!texture.textureName || !glIsTexture(texture.textureName))
        return eOwnerFramePrimitiveStatus::RESOURCE_MISMATCH;
    const auto key = shaderKey(draw, texture.hasAlpha);
    if (!key)
        return eOwnerFramePrimitiveStatus::UNSUPPORTED_MATERIAL;
    if (!validProgram(program, *key) || program.shader.getUniformLocation(SHADER_TEX) < 0)
        return eOwnerFramePrimitiveStatus::SHADER_MISMATCH;
    if (draw.scissors.empty())
        return eOwnerFramePrimitiveStatus::EMPTY;
    if (glGetError() != GL_NO_ERROR)
        return eOwnerFramePrimitiveStatus::GL_ERROR;

    bindTarget(target, draw.material.blend);
    glActiveTexture(GL_TEXTURE0);
    // This exclusive owner leaf selects texture sampling explicitly. A stale
    // sampler object would override the per-draw filter/wrap parameters below.
    glBindSampler(0, 0);
    glBindTexture(GL_TEXTURE_2D, texture.textureName);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, draw.material.wrapX == 0 ? GL_CLAMP_TO_EDGE : GL_REPEAT);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, draw.material.wrapY == 0 ? GL_CLAMP_TO_EDGE : GL_REPEAT);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, draw.material.nearest ? GL_NEAREST : GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, draw.material.nearest ? GL_NEAREST : GL_LINEAR);
    glUseProgram(program.shader.program());
    uniformMatrix(program.shader, SHADER_PROJ, projectionForGL(draw.projection));
    uniformInt(program.shader, SHADER_TEX, 0);
    uniformFloat(program.shader, SHADER_ALPHA, std::clamp(draw.material.alpha, 0.F, 1.F));
    uniformInt(program.shader, SHADER_DISCARD_OPAQUE, draw.material.discardActive && draw.material.discardOpaque);
    uniformInt(program.shader, SHADER_DISCARD_ALPHA, draw.material.discardActive && draw.material.discardAlpha);
    uniformFloat(program.shader, SHADER_DISCARD_ALPHA_VALUE, draw.material.discardOpacity);
    uniformInt(program.shader, SHADER_APPLY_TINT, draw.material.tintEnabled);
    uniformFloat3(program.shader, SHADER_TINT, draw.material.tintRgb);
    applyRounding(program.shader, draw);
    if (draw.material.cm.enabled)
        applyCM(program.shader, draw.material.cm);

    glBindVertexArray(program.shader.getUniformLocation(SHADER_SHADER_VAO));
    glBindBuffer(GL_ARRAY_BUFFER, program.shader.getUniformLocation(SHADER_SHADER_VBO));
    // Keep previous draws' vertices alive exactly as renderTextureInternal does.
    glBufferData(GL_ARRAY_BUFFER, sizeof(fullVerts), nullptr, GL_DYNAMIC_DRAW);
    auto vertices = fullVerts;
    vertices[0].u = vertices[1].u = draw.uv[0];
    vertices[0].v = vertices[2].v = draw.uv[1];
    vertices[2].u = vertices[3].u = draw.uv[2];
    vertices[1].v = vertices[3].v = draw.uv[3];
    glBufferSubData(GL_ARRAY_BUFFER, 0, sizeof(vertices), vertices.data());
    drawScissors(draw);
    glBindVertexArray(0);
    glBindBuffer(GL_ARRAY_BUFFER, 0);
    glBindTexture(GL_TEXTURE_2D, 0);
    glDisable(GL_SCISSOR_TEST);
    return glGetError() == GL_NO_ERROR ? eOwnerFramePrimitiveStatus::COMMANDS_ISSUED : eOwnerFramePrimitiveStatus::GL_ERROR;
}
