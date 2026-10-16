// Nonpresenting discovery via actual DRM, wl_output and Qt APIs.
// Creates no QWindow, renderer or measurement surface; changes no output metadata.
#include <QGuiApplication>
#include <QScreen>
#include <QTimer>
#include <QFile>
#include <QDir>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QDateTime>
#include <wayland-client.h>
#include <xf86drm.h>
#include <xf86drmMode.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h>
#include <cstring>
#include <cstdlib>
#include <algorithm>
#include <memory>
#include <vector>

static qint64 bootNs() {
    timespec ts{};
    if (clock_gettime(CLOCK_BOOTTIME, &ts) != 0) std::abort();
    return qint64(ts.tv_sec) * 1000000000LL + ts.tv_nsec;
}

struct Output {
    wl_output* handle = nullptr;
    QJsonObject data;
};

static void geometry(void* ptr, wl_output*, int32_t x, int32_t y, int32_t w, int32_t h,
                     int32_t subpixel, const char* make, const char* model, int32_t transform) {
    auto& d = static_cast<Output*>(ptr)->data;
    d["physical_width_mm"] = w; d["physical_height_mm"] = h;
    d["position_x"] = x; d["position_y"] = y; d["subpixel"] = subpixel;
    d["make"] = QString::fromUtf8(make); d["model"] = QString::fromUtf8(model);
    d["transform"] = transform;
}
static void mode(void* ptr, wl_output*, uint32_t flags, int32_t w, int32_t h, int32_t refresh) {
    if (flags & WL_OUTPUT_MODE_CURRENT)
        static_cast<Output*>(ptr)->data["current_mode"] = QJsonObject{{"width", w}, {"height", h}, {"refresh_millihz", refresh}};
}
static void done(void*, wl_output*) {}
static void scale(void* ptr, wl_output*, int32_t value) { static_cast<Output*>(ptr)->data["integer_scale"] = value; }
static void name(void* ptr, wl_output*, const char* value) { static_cast<Output*>(ptr)->data["name"] = QString::fromUtf8(value); }
static void description(void* ptr, wl_output*, const char* value) { static_cast<Output*>(ptr)->data["description"] = QString::fromUtf8(value); }
static const wl_output_listener outputListener = {geometry, mode, done, scale, name, description};

static void global(void* ptr, wl_registry* registry, uint32_t id, const char* interface, uint32_t version) {
    if (std::strcmp(interface, wl_output_interface.name) != 0) return;
    auto& outputs = *static_cast<std::vector<std::unique_ptr<Output>>*>(ptr);
    auto output = std::make_unique<Output>();
    output->handle = static_cast<wl_output*>(wl_registry_bind(registry, id, &wl_output_interface, std::min(version, 4u)));
    output->data["protocol_version"] = int(std::min(version, 4u));
    wl_output_add_listener(output->handle, &outputListener, output.get());
    outputs.push_back(std::move(output));
}
static void removed(void*, wl_registry*, uint32_t) {}
static const wl_registry_listener registryListener = {global, removed};

static QJsonArray drm() {
    QJsonArray rows;
    for (const auto& entry : QDir("/dev/dri").entryList({"card*"}, QDir::System | QDir::Files)) {
        const QString path = "/dev/dri/" + entry;
        const int fd = open(path.toLocal8Bit().constData(), O_RDONLY | O_CLOEXEC);
        if (fd < 0) continue;
        drmModeRes* resources = drmModeGetResources(fd);
        if (!resources) { close(fd); continue; }
        for (int i = 0; i < resources->count_connectors; ++i) {
            drmModeConnector* connector = drmModeGetConnector(fd, resources->connectors[i]);
            if (!connector) continue;
            const QString connectorName = QString::fromLatin1(drmModeGetConnectorTypeName(connector->connector_type)) + "-" + QString::number(connector->connector_type_id);
            QJsonObject row{{"card", path}, {"name", connectorName}, {"connected", connector->connection == DRM_MODE_CONNECTED},
                            {"physical_width_mm", int(connector->mmWidth)}, {"physical_height_mm", int(connector->mmHeight)}};
            if (connector->encoder_id) {
                drmModeEncoder* encoder = drmModeGetEncoder(fd, connector->encoder_id);
                if (encoder) {
                    drmModeCrtc* crtc = encoder->crtc_id ? drmModeGetCrtc(fd, encoder->crtc_id) : nullptr;
                    if (crtc) {
                        row["active_mode"] = QJsonObject{{"valid", crtc->mode_valid != 0}, {"width", int(crtc->mode.hdisplay)}, {"height", int(crtc->mode.vdisplay)}};
                        drmModeFreeCrtc(crtc);
                    }
                    drmModeFreeEncoder(encoder);
                }
            }
            QFile edidFile("/sys/class/drm/" + entry + "-" + connectorName + "/edid");
            if (edidFile.open(QIODevice::ReadOnly)) row["edid_base64"] = QString::fromLatin1(edidFile.readAll().toBase64());
            rows.append(row);
            drmModeFreeConnector(connector);
        }
        drmModeFreeResources(resources); close(fd);
    }
    return rows;
}

static QJsonObject rectangle(const QRect& r) {
    return {{"x", r.x()}, {"y", r.y()}, {"width", r.width()}, {"height", r.height()}};
}

int main(int argc, char** argv) {
    if (argc != 2) return 2;
    for (const char* variable : {"QT_FONT_DPI", "QT_SCALE_FACTOR", "QT_SCREEN_SCALE_FACTORS", "QT_DEVICE_PIXEL_RATIO", "QT_USE_PHYSICAL_DPI"})
        if (qEnvironmentVariableIsSet(variable)) return 3;
    QGuiApplication app(argc, argv);
    if (QGuiApplication::platformName() != "wayland") return 4;
    app.setApplicationName("pixel10-display-discovery-probe");
    app.setDesktopFileName("pixel10-display-discovery-probe");
    QTimer::singleShot(1500, [&] {
        QJsonObject result{{"schema_version", 1}, {"qt_version", QString(qVersion())},
                           {"platform", QGuiApplication::platformName()}, {"sampled_boottime_ns", bootNs()}};
        QJsonArray screens;
        for (QScreen* screen : app.screens()) {
            screens.append(QJsonObject{{"name", screen->name()}, {"geometry", rectangle(screen->geometry())},
                {"available_geometry", rectangle(screen->availableGeometry())}, {"device_pixel_ratio", screen->devicePixelRatio()},
                {"physical_size_mm", QJsonObject{{"width", screen->physicalSize().width()}, {"height", screen->physicalSize().height()}}},
                {"physical_dpi_x", screen->physicalDotsPerInchX()}, {"physical_dpi_y", screen->physicalDotsPerInchY()},
                {"logical_dpi_x", screen->logicalDotsPerInchX()}, {"logical_dpi_y", screen->logicalDotsPerInchY()}});
        }
        result["qt_screens"] = screens;
        std::vector<std::unique_ptr<Output>> outputs;
        wl_display* display = wl_display_connect(nullptr);
        if (!display) { app.exit(5); return; }
        wl_registry* registry = wl_display_get_registry(display);
        wl_registry_add_listener(registry, &registryListener, &outputs);
        if (wl_display_roundtrip(display) < 0 || wl_display_roundtrip(display) < 0) { app.exit(6); return; }
        QJsonArray wayland;
        for (const auto& output : outputs) wayland.append(output->data);
        result["wayland_outputs"] = wayland;
        for (const auto& output : outputs) wl_output_destroy(output->handle);
        wl_registry_destroy(registry); wl_display_disconnect(display);
        result["drm_connectors"] = drm();
        QFile file(QString::fromLocal8Bit(argv[1]));
        if (!file.open(QIODevice::WriteOnly | QIODevice::NewOnly)) { app.exit(7); return; }
        if (file.write(QJsonDocument(result).toJson(QJsonDocument::Indented)) < 0) { app.exit(8); return; }
        file.close(); app.quit();
    });
    return app.exec();
}
