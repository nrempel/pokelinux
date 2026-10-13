// Copies one full native output SHM buffer and encodes it without resizing.
#include <QCoreApplication>
#include <QCryptographicHash>
#include <QFile>
#include <QImage>
#include <QImageWriter>
#include <QJsonDocument>
#include <QJsonObject>
#include <wayland-client.h>
#include "wlr-screencopy-client-protocol.h"
#include <sys/mman.h>
#include <unistd.h>
#include <algorithm>
#include <cstdio>
#include <cstring>
#include <memory>
#include <vector>

struct Output {
    wl_output* handle = nullptr;
    QString name;
    int width = 0, height = 0, mmWidth = 0, mmHeight = 0;
};
struct State {
    wl_display* display = nullptr;
    wl_shm* shm = nullptr;
    zwlr_screencopy_manager_v1* manager = nullptr;
    zwlr_screencopy_frame_v1* frame = nullptr;
    wl_buffer* buffer = nullptr;
    void* pixels = MAP_FAILED;
    size_t length = 0;
    int fd = -1;
    uint32_t format = 0, width = 0, height = 0, stride = 0, flags = 0;
    uint32_t timeHi = 0, timeLo = 0, timeNs = 0;
    bool ready = false, failed = false, copied = false;
    std::vector<std::unique_ptr<Output>> outputs;
};
static void geometry(void* ptr, wl_output*, int32_t, int32_t, int32_t w, int32_t h,
                     int32_t, const char*, const char*, int32_t transform) {
    auto& output = *static_cast<Output*>(ptr);
    output.mmWidth = w; output.mmHeight = h;
    if (transform != WL_OUTPUT_TRANSFORM_NORMAL) output.width = -1;
}
static void mode(void* ptr, wl_output*, uint32_t flags, int32_t w, int32_t h, int32_t) {
    if (flags & WL_OUTPUT_MODE_CURRENT) {
        auto& output = *static_cast<Output*>(ptr);
        if (output.width != -1) { output.width = w; output.height = h; }
    }
}
static void outputDone(void*, wl_output*) {}
static void outputScale(void*, wl_output*, int32_t) {}
static void outputName(void* ptr, wl_output*, const char* value) { static_cast<Output*>(ptr)->name = QString::fromUtf8(value); }
static void outputDescription(void*, wl_output*, const char*) {}
static const wl_output_listener outputListener = {geometry, mode, outputDone, outputScale, outputName, outputDescription};

static void global(void* ptr, wl_registry* registry, uint32_t id, const char* interface, uint32_t version) {
    auto& state = *static_cast<State*>(ptr);
    if (std::strcmp(interface, wl_output_interface.name) == 0) {
        if (version < 4) { state.failed = true; return; }
        auto output = std::make_unique<Output>();
        output->handle = static_cast<wl_output*>(wl_registry_bind(registry, id, &wl_output_interface, 4));
        wl_output_add_listener(output->handle, &outputListener, output.get());
        state.outputs.push_back(std::move(output));
    } else if (std::strcmp(interface, wl_shm_interface.name) == 0) {
        state.shm = static_cast<wl_shm*>(wl_registry_bind(registry, id, &wl_shm_interface, 1));
    } else if (std::strcmp(interface, zwlr_screencopy_manager_v1_interface.name) == 0) {
        if (version < 3) { state.failed = true; return; }
        state.manager = static_cast<zwlr_screencopy_manager_v1*>(wl_registry_bind(registry, id, &zwlr_screencopy_manager_v1_interface, 3));
    }
}
static void removed(void*, wl_registry*, uint32_t) {}
static const wl_registry_listener registryListener = {global, removed};

static void buffer(void* ptr, zwlr_screencopy_frame_v1*, uint32_t format, uint32_t w, uint32_t h, uint32_t stride) {
    auto& state = *static_cast<State*>(ptr);
    if (state.width || w != 1080 || h != 2424 || stride < w * 4 || stride > 16384
        || (format != WL_SHM_FORMAT_XRGB8888 && format != WL_SHM_FORMAT_ARGB8888)) {
        state.failed = true; return;
    }
    state.format = format; state.width = w; state.height = h; state.stride = stride;
}
static void flags(void* ptr, zwlr_screencopy_frame_v1*, uint32_t value) {
    auto& state = *static_cast<State*>(ptr);
    if (value & ~uint32_t(ZWLR_SCREENCOPY_FRAME_V1_FLAGS_Y_INVERT)) state.failed = true;
    state.flags = value;
}
static void ready(void* ptr, zwlr_screencopy_frame_v1*, uint32_t hi, uint32_t lo, uint32_t ns) {
    auto& state = *static_cast<State*>(ptr);
    state.timeHi = hi; state.timeLo = lo; state.timeNs = ns; state.ready = true;
}
static void failed(void* ptr, zwlr_screencopy_frame_v1*) { static_cast<State*>(ptr)->failed = true; }
static void damage(void*, zwlr_screencopy_frame_v1*, uint32_t, uint32_t, uint32_t, uint32_t) {}
static void dmabuf(void*, zwlr_screencopy_frame_v1*, uint32_t, uint32_t, uint32_t) {}
static void bufferDone(void* ptr, zwlr_screencopy_frame_v1* frame) {
    auto& state = *static_cast<State*>(ptr);
    if (state.failed || !state.width || state.copied) { state.failed = true; return; }
    state.length = size_t(state.stride) * state.height;
    state.fd = memfd_create("pokelinux-native-output", MFD_CLOEXEC);
    if (state.fd < 0 || ftruncate(state.fd, off_t(state.length)) != 0) { state.failed = true; return; }
    state.pixels = mmap(nullptr, state.length, PROT_READ | PROT_WRITE, MAP_SHARED, state.fd, 0);
    if (state.pixels == MAP_FAILED) { state.failed = true; return; }
    wl_shm_pool* pool = wl_shm_create_pool(state.shm, state.fd, int32_t(state.length));
    state.buffer = wl_shm_pool_create_buffer(pool, 0, int32_t(state.width), int32_t(state.height), int32_t(state.stride), state.format);
    wl_shm_pool_destroy(pool);
    zwlr_screencopy_frame_v1_copy(frame, state.buffer);
    state.copied = true;
}
static const zwlr_screencopy_frame_v1_listener frameListener = {buffer, flags, ready, failed, damage, dmabuf, bufferDone};

static QString pixelHash(const QImage& image) {
    QCryptographicHash hash(QCryptographicHash::Sha256);
    for (int y = 0; y < image.height(); ++y)
        hash.addData(QByteArrayView(reinterpret_cast<const char*>(image.constScanLine(y)), image.width() * 4));
    return QString::fromLatin1(hash.result().toHex());
}

int main(int argc, char** argv) {
    if (argc != 4) return 2; // exact output name, new PNG, new receipt
    QCoreApplication app(argc, argv);
    State state;
    state.display = wl_display_connect(nullptr);
    if (!state.display) return 3;
    wl_registry* registry = wl_display_get_registry(state.display);
    wl_registry_add_listener(registry, &registryListener, &state);
    int status = 0;
    if (wl_display_roundtrip(state.display) < 0 || wl_display_roundtrip(state.display) < 0
        || state.failed || !state.shm || !state.manager || state.outputs.size() != 1) status = 4;
    if (!status) {
        const auto& output = *state.outputs.front();
        if (output.name != QString::fromLocal8Bit(argv[1]) || output.width != 1080 || output.height != 2424
            || output.mmWidth != 65 || output.mmHeight != 146) status = 5;
        else {
            state.frame = zwlr_screencopy_manager_v1_capture_output(state.manager, 0, output.handle);
            zwlr_screencopy_frame_v1_add_listener(state.frame, &frameListener, &state);
            while (!state.ready && !state.failed)
                if (wl_display_dispatch(state.display) < 0) { state.failed = true; break; }
            if (!state.ready || state.failed || !state.copied || state.pixels == MAP_FAILED) status = 6;
        }
    }
    if (!status) {
        const QImage native(static_cast<const uchar*>(state.pixels), int(state.width), int(state.height), int(state.stride),
                            state.format == WL_SHM_FORMAT_XRGB8888 ? QImage::Format_RGB32 : QImage::Format_ARGB32);
        const bool inverted = state.flags & ZWLR_SCREENCOPY_FRAME_V1_FLAGS_Y_INVERT;
        const QImage oriented = inverted ? native.flipped(Qt::Vertical) : native;
        const QImage rgba = oriented.convertToFormat(QImage::Format_RGBA8888);
        const QString pixelsSha = pixelHash(rgba);
        QFile file(QString::fromLocal8Bit(argv[2]));
        if (!file.open(QIODevice::WriteOnly | QIODevice::NewOnly)) status = 7;
        else {
            QImageWriter writer(&file, "PNG");
            if (!writer.write(oriented)) status = 8;
            file.close();
        }
        if (!status) {
            const QImage decoded(QString::fromLocal8Bit(argv[2]));
            if (decoded.size() != QSize(1080, 2424) || pixelHash(decoded.convertToFormat(QImage::Format_RGBA8888)) != pixelsSha) status = 9;
        }
        if (!status) {
            QFile encoded(QString::fromLocal8Bit(argv[2]));
            if (!encoded.open(QIODevice::ReadOnly)) status = 10;
            else {
                const auto pngSha = QString::fromLatin1(QCryptographicHash::hash(encoded.readAll(), QCryptographicHash::Sha256).toHex());
                const auto bufferSha = QString::fromLatin1(QCryptographicHash::hash(QByteArrayView(static_cast<const char*>(state.pixels), qsizetype(state.length)), QCryptographicHash::Sha256).toHex());
                const QJsonObject receipt{{"protocol", "zwlr_screencopy_manager_v1.capture_output"}, {"output", state.outputs.front()->name},
                    {"native_buffer_width", int(state.width)}, {"native_buffer_height", int(state.height)}, {"native_buffer_stride", int(state.stride)},
                    {"native_buffer_shm_format", int(state.format)}, {"protocol_flags", int(state.flags)}, {"protocol_y_invert_corrected", inverted},
                    {"ready_seconds", QString::number((uint64_t(state.timeHi) << 32) | state.timeLo)}, {"ready_nanoseconds", int(state.timeNs)},
                    {"raw_native_buffer_sha256", bufferSha}, {"png_sha256", pngSha}, {"oriented_rgba_pixels_sha256", pixelsSha},
                    {"png_decode_pixel_hash_matches_native_buffer", true}, {"raw_png_width", 1080}, {"raw_png_height", 2424},
                    {"resize_or_export_scale_used", false}, {"hardware_verified", false}};
                QFile record(QString::fromLocal8Bit(argv[3]));
                if (!record.open(QIODevice::WriteOnly | QIODevice::NewOnly)) status = 11;
                else {
                    const auto payload = QJsonDocument(receipt).toJson(QJsonDocument::Indented);
                    if (record.write(payload) != payload.size()) status = 12;
                    record.close();
                }
            }
        }
    }
    if (state.frame) zwlr_screencopy_frame_v1_destroy(state.frame);
    if (state.buffer) wl_buffer_destroy(state.buffer);
    if (state.pixels != MAP_FAILED) munmap(state.pixels, state.length);
    if (state.fd >= 0) close(state.fd);
    for (const auto& output : state.outputs) wl_output_destroy(output->handle);
    if (state.manager) zwlr_screencopy_manager_v1_destroy(state.manager);
    if (state.shm) wl_shm_destroy(state.shm);
    wl_registry_destroy(registry); wl_display_disconnect(state.display);
    if (status) std::fprintf(stderr, "native output capture boundary failed:%d\n", status);
    return status;
}
