// Simulation-only normal Hyprland plugin API; no Source or policy interface.
#define QT_NO_KEYWORDS
// Qt declares Qt::HANDLE; parse it before the plugin API's HANDLE macro.
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QCryptographicHash>
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/output/Monitor.hpp>
#include <hyprland/src/state/MonitorState.hpp>
#include <aquamarine/output/Output.hpp>
#include <cmath>
#include <cstdlib>
#include <stdexcept>

namespace {
PHLMONITORREF selectedMonitor;
WP<Aquamarine::IOutput> selectedOutput;
Hyprutils::Math::Vector2D originalSize;
Hyprutils::Math::Vector2D selectedSize;
bool applied = false;

QString environmentPath(const char* key) {
    const char* value = std::getenv(key);
    if (!value || value[0] != '/') throw std::runtime_error("absolute selected profile/receipt path required");
    return QString::fromLocal8Bit(value);
}
}

APICALL EXPORT std::string PLUGIN_API_VERSION() { return HYPRLAND_API_VERSION; }

APICALL EXPORT PLUGIN_DESCRIPTION_INFO PLUGIN_INIT(HANDLE) {
    if (std::string(__hyprland_api_get_hash()) != __hyprland_api_get_client_hash())
        throw std::runtime_error("exact installed compositor API mismatch");
    if (applied) throw std::runtime_error("virtual physical extent already selected");
    QFile profileFile(environmentPath("POKELINUX_DISPLAY_PROFILE"));
    if (!profileFile.open(QIODevice::ReadOnly)) throw std::runtime_error("selected profile unavailable");
    const QByteArray bytes = profileFile.readAll();
    QJsonParseError error{};
    const auto document = QJsonDocument::fromJson(bytes, &error);
    if (error.error != QJsonParseError::NoError || !document.isObject())
        throw std::runtime_error("selected profile is not a JSON object");
    const auto profile = document.object();
    const auto pixels = profile["native_pixels"].toObject();
    const auto physical = profile["physical"].toObject();
    const auto mm = physical["nominal_mm"].toObject();
    const auto ui = profile["native_ui"].toObject();
    if (profile["schema_version"].toInt() != 1 || profile["id"].toString() != "pixel10"
        || pixels["width"].toInt() != 1080 || pixels["height"].toInt() != 2424
        || mm["width"].toInt() != 65 || mm["height"].toInt() != 146
        || physical["published_ppi"].toDouble() != 422 || physical["ppi_tolerance"].toDouble() != 1
        || ui["requested_device_pixel_ratio"].toDouble() != 2.625 || ui["dpr_tolerance"].toDouble() != 0.001)
        throw std::runtime_error("reviewed Pixel10 simulation profile guard changed");
    const auto& monitors = State::monitorState()->monitors();
    if (monitors.size() != 1) throw std::runtime_error("require exactly one actual virtual output");
    const auto monitor = monitors.front();
    const auto output = monitor->output();
    const auto size = monitor->pixelSize();
    if (!monitor->enabled() || !output || monitor->name() != "Virtual-1"
        || size.x != 1080 || size.y != 2424 || std::abs(monitor->scale() - 2.625) > 0.001
        || output->make != "Red Hat, Inc." || output->model != "QEMU Monitor")
        throw std::runtime_error("actual output is not the selected1080x2424 QEMU virtual display");
    QFile receipt(environmentPath("POKELINUX_DISPLAY_PROFILE_RECEIPT"));
    if (!receipt.open(QIODevice::WriteOnly | QIODevice::NewOnly))
        throw std::runtime_error("new profile application receipt unavailable");
    originalSize = output->physicalSize;
    selectedSize = {mm["width"].toDouble(), mm["height"].toDouble()};
    selectedMonitor = monitor;
    selectedOutput = output;
    output->physicalSize = selectedSize;
    applied = true;
    monitor->m_events.modeChanged.emit();
    monitor->scheduleDone();
    const QJsonObject record{{"profile_sha256", QString::fromLatin1(QCryptographicHash::hash(bytes, QCryptographicHash::Sha256).toHex())},
        {"plugin_api_hash", QString::fromUtf8(__hyprland_api_get_hash())}, {"output", QString::fromUtf8(monitor->name().data(), monitor->name().size())},
        {"native_width", size.x}, {"native_height", size.y}, {"scale", monitor->scale()},
        {"original_physical_mm", QJsonObject{{"width", originalSize.x}, {"height", originalSize.y}}},
        {"selected_physical_mm", QJsonObject{{"width", selectedSize.x}, {"height", selectedSize.y}}},
        {"api", "IOutput.physicalSize; Monitor.modeChanged.emit; Monitor.scheduleDone"},
        {"simulation_only", true}, {"hardware_verified", false}, {"source_or_policy_selected", false}};
    const auto payload = QJsonDocument(record).toJson(QJsonDocument::Indented);
    if (receipt.write(payload) != payload.size()) {
        output->physicalSize = originalSize;
        applied = false;
        monitor->m_events.modeChanged.emit();
        monitor->scheduleDone();
        throw std::runtime_error("profile application receipt write failed");
    }
    receipt.close();
    return {"pokelinux-virtual-display-profile", "Physical extent for the selected virtual Pixel10 display", "Pokelinux", "1"};
}

APICALL EXPORT void PLUGIN_EXIT() {
    const auto monitor = selectedMonitor.lock();
    const auto output = selectedOutput.lock();
    if (applied && monitor && output && monitor->output() == output
        && output->physicalSize == selectedSize) {
        output->physicalSize = originalSize;
        monitor->m_events.modeChanged.emit();
        monitor->scheduleDone();
    }
    applied = false;
    selectedMonitor.reset();
    selectedOutput.reset();
}
