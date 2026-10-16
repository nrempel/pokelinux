// CPU custody integration preparation. Link the real codec/wrapper and actual
// Rust capture/visitor library; no replacement ABI definitions or renderer.
#include "FramePlan.hpp"
#include "FramePlanCodec.hpp"
#include "SHMSnapshot.hpp"
#include "pokelinux_render_frame.h"

#include <algorithm>
#include <array>
#include <bit>
#include <cstdint>
#include <iostream>
#include <limits>
#include <memory>
#include <thread>
#include <utility>
#include <vector>

struct SFailure { const char* reason; };
static void require(bool condition, const char* reason) {
    if (!condition)
        throw SFailure{reason};
}

static bool sameFloat(float a, float b) {
    return std::bit_cast<uint32_t>(a) == std::bit_cast<uint32_t>(b);
}

static bool sameDouble(double a, double b) {
    return std::bit_cast<uint64_t>(a) == std::bit_cast<uint64_t>(b);
}

template<size_t N>
static void checkArray(const std::array<float, N>& expected, const float (&actual)[N]) {
    for (size_t i = 0; i < N; ++i)
        require(sameFloat(expected[i], actual[i]), "float array changed or transposed");
}

static void checkDescription(const PrShmDescription& expected, const PrShmDescription& actual) {
    require(expected.resource_id == actual.resource_id && expected.generation == actual.generation &&
                expected.commit_sequence == actual.commit_sequence && expected.drm_format == actual.drm_format && expected.width == actual.width &&
                expected.height == actual.height && expected.stride == actual.stride && expected.bytes_per_pixel == actual.bytes_per_pixel &&
                expected.reserved == actual.reserved,
            "SHM descriptor changed");
}

static void checkBox(const Render::SFrameBox& expected, const PrFrameBoxV1& actual) {
    require(sameDouble(expected.x, actual.x) && sameDouble(expected.y, actual.y) && sameDouble(expected.width, actual.width) &&
                sameDouble(expected.height, actual.height) && sameDouble(expected.rotation, actual.rotation),
            "copied box changed");
}

static void checkCM(const Render::SFrameCM& expected, const PrFrameCMV1& actual) {
    require(((actual.flags & PR_CM_ENABLED) != 0) == expected.enabled && ((actual.flags & PR_CM_TONEMAP) != 0) == expected.needsTonemap &&
                ((actual.flags & PR_CM_SDR_MOD) != 0) == expected.needsSDRmod && (actual.flags & ~uint32_t{7}) == 0,
            "CM flags changed");
    require(expected.sourceTF == actual.source_tf && expected.targetTF == actual.target_tf && expected.tonemapMode == actual.tonemap_mode,
            "CM enums changed");
    require(sameFloat(expected.srcTFMin, actual.src_tf_min) && sameFloat(expected.srcTFMax, actual.src_tf_max) &&
                sameFloat(expected.dstTFMin, actual.dst_tf_min) && sameFloat(expected.dstTFMax, actual.dst_tf_max) &&
                sameFloat(expected.srcRefLuminance, actual.src_ref_luminance) && sameFloat(expected.dstRefLuminance, actual.dst_ref_luminance) &&
                sameFloat(expected.maxLuminance, actual.max_luminance) && sameFloat(expected.dstMaxLuminance, actual.dst_max_luminance) &&
                sameFloat(expected.sdrSaturation, actual.sdr_saturation) && sameFloat(expected.sdrBrightness, actual.sdr_brightness),
            "CM scalar changed");
    checkArray(expected.convertMatrix, actual.convert_matrix);
    checkArray(expected.dstPrimariesXYZ, actual.dst_primaries_xyz);
}

static void checkMaterial(const Render::SFrameMaterial& expected, const PrFrameMaterialV1& actual) {
    require(sameFloat(expected.alpha, actual.alpha) && sameFloat(expected.roundingPx, actual.rounding_px) &&
                sameFloat(expected.roundingPower, actual.rounding_power) && sameFloat(expected.discardOpacity, actual.discard_opacity),
            "material scalar changed");
    require(((actual.flags & PR_MAT_NEAREST) != 0) == expected.nearest && ((actual.flags & PR_MAT_BLEND) != 0) == expected.blend &&
                ((actual.flags & PR_MAT_RGBX) != 0) == expected.rgbx && ((actual.flags & PR_MAT_TINT) != 0) == expected.tintEnabled &&
                ((actual.flags & PR_MAT_DISCARD_ACTIVE) != 0) == expected.discardActive &&
                ((actual.flags & PR_MAT_DISCARD_OPAQUE) != 0) == expected.discardOpaque &&
                ((actual.flags & PR_MAT_DISCARD_ALPHA) != 0) == expected.discardAlpha && (actual.flags & ~uint32_t{127}) == 0,
            "material flags changed");
    require(expected.wrapX == actual.wrap_x && expected.wrapY == actual.wrap_y && actual.reserved == 0, "wrap/reserved changed");
    checkArray(expected.tintRgb, actual.tint_rgb);
    checkCM(expected.cm, actual.cm);
}

struct SSnapshotDeleter {
    void operator()(PrSnapshot* snapshot) const noexcept { pr_snapshot_release(snapshot); }
};
using Snapshot = std::unique_ptr<PrSnapshot, SSnapshotDeleter>;

struct SExpected {
    Render::SFrameTarget target;
    uint64_t scene = 0, security = 0;
    bool commit = false, fullRepaint = false;
    std::vector<Render::SFrameRect> damage, finalDamage;
    std::vector<Render::SFrameDraw> draws; // value metadata only, no snapshot owners
    std::array<PrShmDescription, 2> descriptions{};
    std::array<std::vector<uint8_t>, 2> bytes;
    std::array<Snapshot, 2> transfers;
    const char* error = nullptr;
    bool visited = false;
};

static void checkRects(const std::vector<Render::SFrameRect>& expected, const PrFrameBorrowV1& view, uint32_t first, uint32_t count) {
    require(count == expected.size() && first <= view.rect_count && count <= view.rect_count - first, "rectangle range changed");
    for (size_t i = 0; i < expected.size(); ++i) {
        const auto& a = expected[i];
        const auto& b = view.rects[first + i];
        require(a.x1 == b.x1 && a.y1 == b.y1 && a.x2 == b.x2 && a.y2 == b.y2, "GL scissor changed");
    }
}

static void checkPixels(const PrSnapshotView& view, const PrShmDescription& description, const std::vector<uint8_t>& bytes) {
    checkDescription(description, view.description);
    require(view.pixel_bytes == bytes.size() && view.pixel_bytes == uint64_t{description.stride} * description.height, "stride/byte length changed");
    require(view.pixels && std::equal(bytes.begin(), bytes.end(), view.pixels), "immutable pixels or row padding changed");
}

static int32_t visitCaptured(const PrFrameBorrowV1* view, void* stackToken) noexcept {
    auto& expected = *static_cast<SExpected*>(stackToken);
    try {
        require(view && !expected.visited, "visitor missing/repeated");
        expected.visited = true;
        require(view->abi_version == PR_FRAME_ABI_V1 && view->struct_size == sizeof(PrFrameBorrowV1) && view->reserved == 0 && view->reserved2 == 0,
                "frame ABI/reserved changed");
        require(view->scene_sequence == expected.scene && view->security_generation == expected.security, "full-width frame identity changed");
        require(((view->flags & PR_FRAME_COMMIT) != 0) == expected.commit && ((view->flags & PR_FRAME_FULL_REPAINT) != 0) == expected.fullRepaint &&
                    (view->flags & ~uint32_t{3}) == 0,
                "frame flags changed");
        const auto& t = view->target;
        require(t.output_id == expected.target.outputId && t.output_generation == expected.target.outputGeneration && t.width == expected.target.width &&
                    t.height == expected.target.height && t.transformed_width == expected.target.transformedWidth &&
                    t.transformed_height == expected.target.transformedHeight && t.output_transform == expected.target.outputTransform &&
                    sameDouble(t.scale, expected.target.scale) &&
                    ((t.flags & PR_TARGET_SHADER_CM_AVAILABLE) != 0) == expected.target.shaderCMAvailable && (t.flags & ~uint32_t{1}) == 0,
                "target values changed");
        checkArray(expected.target.targetProjection, t.target_projection);
        checkArray(expected.target.scaleMatrix, t.scale_matrix);
        require(view->draw_count == expected.draws.size() && view->resource_count == expected.descriptions.size(), "draw/resource count changed");
        uint32_t nextRect = 0;
        require(view->damage_first == nextRect, "damage flatten order changed");
        checkRects(expected.damage, *view, view->damage_first, view->damage_count);
        nextRect += view->damage_count;
        require(view->final_damage_first == nextRect, "final damage flatten order changed");
        checkRects(expected.finalDamage, *view, view->final_damage_first, view->final_damage_count);
        nextRect += view->final_damage_count;
        for (size_t i = 0; i < expected.draws.size(); ++i) {
            const auto& a = expected.draws[i];
            const auto& b = view->draws[i];
            const uint32_t kind = a.kind == Render::eFrameDrawKind::CLEAR ? uint32_t{PR_DRAW_CLEAR} :
                a.kind == Render::eFrameDrawKind::RECT ? uint32_t{PR_DRAW_RECT} : uint32_t{PR_DRAW_SHM};
            require(b.kind == kind && a.resourceIndex == b.resource_index && a.surfaceId == b.surface_id && a.commitSequence == b.commit_sequence,
                    "draw identity/index changed");
            require(b.scissor_first == nextRect, "draw scissor flatten order changed");
            checkRects(a.scissors, *view, b.scissor_first, b.scissor_count);
            nextRect += b.scissor_count;
            checkBox(a.box, b.box);
            checkBox(a.roundingBox, b.rounding_box);
            checkArray(a.projection, b.projection);
            checkArray(a.uv, b.uv);
            checkArray(a.convertedColor, b.converted_color);
            checkArray(a.colorSRGB, b.color_srgb);
            checkMaterial(a.material, b.material);
        }
        require(nextRect == view->rect_count, "global rectangle sum changed");
        for (size_t i = 0; i < expected.descriptions.size(); ++i) {
            const auto& resource = view->resources[i];
            checkDescription(expected.descriptions[i], resource.description);
            PrSnapshotView pixels{};
            require(pr_snapshot_view(resource.snapshot, &pixels) == PR_OK, "real captured snapshot view unavailable");
            checkPixels(pixels, expected.descriptions[i], expected.bytes[i]);
            expected.transfers[i].reset(pr_snapshot_clone(resource.snapshot));
            require(bool(expected.transfers[i]), "real snapshot clone failed");
        }
        return PR_FRAME_OK;
    } catch (const SFailure& failure) {
        expected.error = failure.reason;
        return PR_FRAME_INVALID;
    } catch (...) {
        expected.error = "unexpected exception inside synchronous visitor";
        return PR_FRAME_INVALID;
    }
}

static int32_t visitorSentinel(const PrFrameBorrowV1*, void* stackToken) noexcept {
    *static_cast<bool*>(stackToken) = true;
    return 73; // Exact callback return must propagate, not become successful capture.
}

static Render::SFrameDraw makeDraw(Render::eFrameDrawKind kind, uint32_t index) {
    Render::SFrameDraw draw;
    draw.kind = kind;
    draw.box = {17.25, 23.5, 99.75, 101.125, 0.125};
    draw.roundingBox = {16.25, 22.5, 103.75, 105.125, -0.0};
    draw.projection = {0.03125f, -0.0f, 0.0625f, 0.125f, 0.25f, 0.5f, 0.75f, 0.875f, 1.0f};
    draw.uv = {0.125f, 0.25f, 0.75f, 0.875f};
    draw.convertedColor = {0.125f, 0.25f, 0.375f, 0.75f};
    draw.colorSRGB = {0.0625f, 0.1875f, 0.3125f, 0.75f};
    draw.material.alpha = 0.75f;
    draw.material.roundingPx = 6.25f;
    draw.material.roundingPower = 2.2f;
    draw.material.nearest = (index % 2) != 0;
    draw.material.blend = (index % 2) == 0;
    draw.material.rgbx = index == 3;
    draw.material.tintEnabled = true;
    draw.material.wrapX = static_cast<uint8_t>(index % 2);
    draw.material.wrapY = static_cast<uint8_t>((index + 1) % 2);
    draw.material.discardActive = index >= 2;
    draw.material.discardOpaque = index == 2;
    draw.material.discardAlpha = index == 3;
    draw.material.discardOpacity = 0.125f;
    draw.material.tintRgb = {0.25f, 0.5f, 0.75f};
    auto& cm = draw.material.cm;
    cm.enabled = true;
    cm.needsTonemap = true;
    cm.needsSDRmod = true;
    cm.sourceTF = 2;
    cm.targetTF = 1;
    cm.tonemapMode = 1;
    cm.srcTFMin = 0.125f;
    cm.srcTFMax = 1000.0f;
    cm.dstTFMin = 0.25f;
    cm.dstTFMax = 500.0f;
    cm.srcRefLuminance = 100.0f;
    cm.dstRefLuminance = 80.0f;
    cm.maxLuminance = 1200.0f;
    cm.dstMaxLuminance = 600.0f;
    cm.convertMatrix = {1.0f, 0.125f, 0.25f, 0.375f, 1.0f, 0.5f, 0.625f, 0.75f, 1.0f};
    cm.dstPrimariesXYZ = {0.875f, 0.75f, 0.625f, 0.5f, 0.375f, 0.25f, 0.125f, -0.0f, 1.0f};
    cm.sdrSaturation = 1.125f;
    cm.sdrBrightness = 0.875f;
    draw.scissors = {{static_cast<int32_t>(index), 2, 90, 100}, {5, 6, 70, 80}};
    draw.surfaceId = 0x10203040ULL + index;
    return draw;
}

static Render::SFramePlan makePlan(SExpected& expected) {
    Render::SFramePlan plan;
    plan.sceneSequence = 0xabcdef0000000023ULL;
    plan.securityGeneration = 0xfedcba0000000045ULL; // authored CPU fixture, not authentic runtime authority
    plan.target.outputId = 17;
    plan.target.outputGeneration = 19;
    plan.target.width = 1080;
    plan.target.height = 2424;
    plan.target.transformedWidth = 2424;
    plan.target.transformedHeight = 1080;
    plan.target.outputTransform = 1;
    plan.target.scale = 1.5;
    plan.target.shaderCMAvailable = true;
    plan.target.targetProjection = {1.0f, 0.125f, 0.25f, 0.375f, 0.5f, 0.625f, 0.75f, 0.875f, 1.0f};
    plan.target.scaleMatrix = {1.5f, -0.0f, 0.125f, 0.25f, 1.5f, 0.375f, 0.5f, 0.625f, 1.0f};
    plan.commit = true;
    plan.fullRepaint = true;
    plan.damage = {{0, 0, 1080, 2424}, {9, 10, 101, 202}};
    plan.finalDamage = {{1, 2, 1079, 2422}};
    for (size_t i = 0; i < expected.bytes.size(); ++i) {
        PrShmDescription description{};
        description.drm_format = i == 0 ? 0x34325241u : 0x34325258u; // AR24 / XR24
        description.width = i == 0 ? 3 : 2;
        description.height = i == 0 ? 2 : 3;
        description.stride = i == 0 ? 20 : 12;
        description.bytes_per_pixel = 4;
        auto& bytes = expected.bytes[i];
        bytes.resize(size_t{description.stride} * description.height);
        for (size_t n = 0; n < bytes.size(); ++n)
            bytes[n] = static_cast<uint8_t>((n * 37 + i * 53 + 11) & 0xff);
        auto producerBytes = bytes;
        Render::CSHMSnapshot snapshot;
        require(Render::CSHMSnapshot::capture(description, producerBytes.data(), producerBytes.size(), snapshot) == PR_OK, "actual wrapper capture failed");
        const auto view = snapshot.view();
        require(view.has_value(), "actual wrapper view failed");
        description = view->description; // Genuine control-store immutable import identity.
        expected.descriptions[i] = description;
        plan.resources.push_back({description, std::move(snapshot)});
        std::fill(producerBytes.begin(), producerBytes.end(), 0xee); // Original producer bytes changed before frame capture.
    }
    plan.draws.push_back(makeDraw(Render::eFrameDrawKind::CLEAR, 0));
    plan.draws.push_back(makeDraw(Render::eFrameDrawKind::RECT, 1));
    plan.draws.push_back(makeDraw(Render::eFrameDrawKind::SHM, 2));
    plan.draws.push_back(makeDraw(Render::eFrameDrawKind::SHM, 3));
    for (size_t i = 0; i < expected.descriptions.size(); ++i) {
        auto& draw = plan.draws[i + 2];
        draw.resourceIndex = static_cast<uint32_t>(i);
        draw.commitSequence = expected.descriptions[i].commit_sequence;
    }
    expected.target = plan.target;
    expected.scene = plan.sceneSequence;
    expected.security = plan.securityGeneration;
    expected.commit = plan.commit;
    expected.fullRepaint = plan.fullRepaint;
    expected.damage = plan.damage;
    expected.finalDamage = plan.finalDamage;
    expected.draws = plan.draws;
    return plan;
}

static void checkTransferredRefusal() {
    std::unique_ptr<PrSnapshotStore, decltype(&pr_snapshot_store_release)> store(pr_snapshot_store_create(4096), pr_snapshot_store_release);
    require(bool(store), "test store unavailable");
    const PrShmDescription description{701, 702, 703, 0x34325241u, 3, 2, 20, 4, 0};
    const std::vector<uint8_t> pixels(40, 0x91);
    PrSnapshot* raw = nullptr;
    require(pr_snapshot_capture(store.get(), &description, pixels.data(), pixels.size(), &raw) == PR_OK && raw, "test transfer capture failed");
    require(pr_snapshot_store_used(store.get()) == pixels.size(), "test budget not charged");
    Render::CSHMSnapshot prior;
    PrShmDescription priorDescription{};
    priorDescription.drm_format = 0x34325241u;
    priorDescription.width = 3;
    priorDescription.height = 2;
    priorDescription.stride = 20;
    priorDescription.bytes_per_pixel = 4;
    require(Render::CSHMSnapshot::capture(priorDescription, pixels.data(), pixels.size(), prior) == PR_OK, "prior wrapper capture failed");
    const auto priorView = prior.view();
    require(priorView.has_value(), "prior view absent");
    auto mismatch = description;
    ++mismatch.commit_sequence;
    require(Render::CSHMSnapshot::takeTransferred(std::exchange(raw, nullptr), mismatch, prior) == PR_INVALID, "mismatch transfer accepted");
    require(!raw && pr_snapshot_store_used(store.get()) == 0, "refused incoming handle was not released");
    const auto after = prior.view();
    require(after.has_value(), "refusal replaced old result");
    checkPixels(*after, priorView->description, pixels);
    require(Render::CSHMSnapshot::takeTransferred(nullptr, description, prior) == PR_INVALID, "null transfer accepted");
    checkDescription(priorView->description, prior.view()->description);
}

int main() {
    try {
        SExpected expected;
        auto plan = makePlan(expected);
        Render::CFramePlanCapture captured;
        require(Render::CFramePlanCodec::capture(plan, captured) == PR_FRAME_OK && bool(captured), "real Rust frame capture failed");
        const PrFrame* first = captured.get();
        plan.draws[0].projection[0] = std::numeric_limits<float>::quiet_NaN();
        require(Render::CFramePlanCodec::capture(plan, captured) == PR_FRAME_INVALID && captured.get() == first, "finite refusal replaced output");
        plan.draws[0].projection[0] = expected.draws[0].projection[0];
        const auto savedDamage = plan.damage;
        plan.damage.resize(PR_FRAME_MAX_RECTS);
        require(Render::CFramePlanCodec::capture(plan, captured) == PR_FRAME_CAPACITY && captured.get() == first, "GLOBAL rect limit/refusal broken");
        plan.damage = savedDamage;
        const auto savedDescription = plan.resources[0].description;
        ++plan.resources[0].description.stride;
        require(Render::CFramePlanCodec::capture(plan, captured) == PR_FRAME_INVALID && captured.get() == first, "descriptor refusal replaced output");
        plan.resources[0].description = savedDescription;
        std::cout << "PASS global_bound_finite_descriptor_refusal_preserves_output\n";
        plan = Render::SFramePlan{}; // All original plan vectors and snapshot owners are gone.
        require(pr_frame_with_view_v1(captured.get(), visitCaptured, &expected) == PR_FRAME_OK && expected.visited && !expected.error,
                expected.error ? expected.error : "real frame visitor failed");
        bool sentinelVisited = false;
        require(pr_frame_with_view_v1(captured.get(), visitorSentinel, &sentinelVisited) == 73 && sentinelVisited, "visitor status not propagated");
        std::cout << "PASS full_values_matrices_global_ranges_after_original_plan_drop\n";
        Render::SFramePlan replacement;
        replacement.sceneSequence = expected.scene + 1;
        replacement.securityGeneration = expected.security;
        replacement.target = expected.target;
        replacement.damage = {{0, 0, 1080, 2424}};
        replacement.finalDamage = replacement.damage;
        replacement.draws.push_back(makeDraw(Render::eFrameDrawKind::CLEAR, 0));
        require(Render::CFramePlanCodec::capture(replacement, captured) == PR_FRAME_OK && bool(captured), "real frame replacement failed");
        replacement = Render::SFramePlan{};
        captured.reset(); // No original captured frame or original snapshot handles remain.
        const char* workerError = nullptr;
        std::thread worker([a = std::move(expected.transfers[0]), b = std::move(expected.transfers[1]), &expected, &workerError]() mutable {
            try {
                Render::CSHMSnapshot ownedA, ownedB;
                require(Render::CSHMSnapshot::takeTransferred(a.release(), expected.descriptions[0], ownedA) == PR_OK, "worker transfer A rejected");
                require(Render::CSHMSnapshot::takeTransferred(b.release(), expected.descriptions[1], ownedB) == PR_OK, "worker transfer B rejected");
                const auto viewA = ownedA.view();
                const auto viewB = ownedB.view();
                require(viewA && viewB, "worker owned snapshot view unavailable");
                checkPixels(*viewA, expected.descriptions[0], expected.bytes[0]);
                checkPixels(*viewB, expected.descriptions[1], expected.bytes[1]);
                ownedA.reset();
                ownedB.reset(); // Final imported handles released on this actual joined worker.
            } catch (const SFailure& failure) {
                workerError = failure.reason;
            } catch (...) {
                workerError = "unexpected worker exception";
            }
        });
        worker.join();
        require(!workerError, workerError ? workerError : "worker failure");
        std::cout << "PASS padded_stride_immutable_custody_after_frame_replacement_and_actual_worker_join\n";
        checkTransferredRefusal();
        std::cout << "PASS refused_transfer_consumes_incoming_handle_preserves_prior_result\n";
        std::cout << "CPU_CUSTODY_ONLY no_upload_fence_commit_presentation_or_authority_claim\n";
        return 0;
    } catch (const SFailure& failure) {
        std::cerr << "FAIL " << failure.reason << '\n';
        return 1;
    } catch (...) {
        std::cerr << "FAIL unexpected exception\n";
        return 2;
    }
}
