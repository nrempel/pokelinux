// Normal unshown Settings QML + real adapter/C ABI against an authored root peer.
// Actual actor, OS grant, backend packets, normal Main/window and output are separate.
#include "SettingsClient.h"
#include "SettingsConnectivityClient.h"
#include "PhoneAppearanceClient.h"
#include "PhoneNetworkStatusClient.h"
#include <QCoreApplication>
#include <QEventLoop>
#include <QFile>
#include <QGuiApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMouseEvent>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQuickItem>
#include <QQuickStyle>
#include <QQuickView>
#include <QSignalSpy>
#include <QTimer>
#include <algorithm>
#include <cassert>
#include <cmath>
#include <cstdio>
#include <memory>
#include <stdexcept>
#include <time.h>
#include <unistd.h>

namespace {
uint64_t boot() {
    timespec value{};
    if (::clock_gettime(CLOCK_BOOTTIME, &value) != 0) std::abort();
    return uint64_t(value.tv_sec) * 1000000000ULL + uint64_t(value.tv_nsec);
}
QString decimal(uint64_t value) { return QString::number(value); }
void require(bool condition, const char *message) {
    if (!condition) throw std::runtime_error(message);
}
QJsonArray calls;
unsigned deadlineCalls = 0, stopCalls = 0, joinedOK = 0, freedOK = 0;
uint64_t originalUntil = 0, captureBefore = 0, captureAfter = 0;
void called(const char *name, int32_t result, uint64_t before) {
    calls.append(QJsonObject{{"operation", QString::fromLatin1(name)}, {"result", result},
        {"before_ns", decimal(before)}, {"after_ns", decimal(boot())}});
}
bool hidden(const QQuickView &view) {
    if (view.isVisible() || view.isExposed()) return false;
    for (auto *window : QGuiApplication::allWindows()) {
        if (qobject_cast<QQuickWindow *>(window) && (window->isVisible() || window->isExposed())) return false;
    }
    return true;
}
template<class Predicate> bool delivered(uint64_t until, Predicate predicate) {
    while (boot() < until) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 1);
        if (predicate()) return boot() < until;
        ::usleep(1000);
    }
    return false;
}
QQuickItem *named(QQuickView &view, const QString &name) {
    QList<QQuickItem *> items{view.rootObject()};
    QQuickItem *found = nullptr;
    for (qsizetype i = 0; i < items.size(); ++i) {
        auto *item = items.at(i);
        if (item->objectName() == name) { require(!found, "ambiguous real QML objectName"); found = item; }
        items.append(item->childItems());
    }
    require(found, "required normal QML item absent");
    return found;
}
QString text(QQuickView &view, const char *name) {
    auto *item = named(view, QString::fromLatin1(name));
    require(item->property("textFormat").isValid() && item->property("textFormat").toInt() == 0,
            "normal QML text must be plain");
    return item->property("text").toString();
}
QJsonObject click(QQuickView &view, QQuickItem *button) {
    QList<QQuickItem *> visual{view.rootObject()};
    for (qsizetype i = 0; i < visual.size(); ++i) visual.append(visual.at(i)->childItems());
    for (qsizetype i = visual.size(); i > 0; --i) visual.at(i - 1)->ensurePolished();
    view.contentItem()->ensurePolished();
    require(hidden(view), "unshown component became visible during layout");
    require(button->inherits("QQuickButton") && button->isEnabled() && button->isVisible(), "real enabled Button required");
    require(button->width() > 0 && button->height() > 0, "real Button must have positive geometry");
    const QPointF center = button->mapToScene(QPointF(button->width() / 2, button->height() / 2));
    require(std::isfinite(center.x()) && std::isfinite(center.y()) && center.x() >= 0 && center.y() >= 0
        && center.x() < view.width() && center.y() < view.height(), "real Button center outside component");
    for (auto *parent = button->parentItem(); parent; parent = parent->parentItem()) {
        if (parent->clip()) require(QRectF(0, 0, parent->width(), parent->height()).contains(parent->mapFromScene(center)),
                                   "real Button center clipped");
    }
    const int index = button->metaObject()->indexOfSignal("clicked()");
    require(index >= 0, "real Button has no clicked signal");
    QSignalSpy clicked(button, button->metaObject()->method(index));
    require(clicked.isValid(), "real clicked observer invalid");
    const uint64_t before = boot();
    QMouseEvent press(QEvent::MouseButtonPress, center, center, center, Qt::LeftButton,
                      Qt::LeftButton, Qt::NoModifier, QPointingDevice::primaryPointingDevice());
    press.setTimestamp(before / 1000000ULL);
    const bool pressDelivered = QCoreApplication::sendEvent(&view, &press);
    const bool afterPressHidden = hidden(view);
    QMouseEvent release(QEvent::MouseButtonRelease, center, center, center, Qt::LeftButton,
                        Qt::NoButton, Qt::NoModifier, QPointingDevice::primaryPointingDevice());
    release.setTimestamp(boot() / 1000000ULL);
    const bool releaseDelivered = QCoreApplication::sendEvent(&view, &release);
    const uint64_t after = boot();
    require(pressDelivered && releaseDelivered && clicked.count() == 1, "real press/release must activate Button once");
    require(afterPressHidden && hidden(view), "mouse delivery showed component");
    return {{"button", button->objectName()}, {"press_delivered", pressDelivered}, {"release_delivered", releaseDelivered},
        {"press_accepted", press.isAccepted()}, {"release_accepted", release.isAccepted()}, {"clicked", clicked.count()},
        {"before_ns", decimal(before)}, {"after_ns", decimal(after)}, {"scene_x", center.x()}, {"scene_y", center.y()},
        {"button_width", button->width()}, {"button_height", button->height()}, {"all_quick_windows_hidden", true}};
}
}
extern "C" uint64_t __real_pr_settings_connectivity_deadline_v1();
extern "C" int32_t __real_pr_settings_connectivity_stop_v1(PrSettingsConnectivityClient *);
extern "C" int32_t __real_pr_settings_connectivity_join_v1(PrSettingsConnectivityClient *);
extern "C" int32_t __real_pr_settings_connectivity_free_v1(PrSettingsConnectivityClient *);
extern "C" uint64_t __wrap_pr_settings_connectivity_deadline_v1() {
    ++deadlineCalls; captureBefore = boot();
    originalUntil = __real_pr_settings_connectivity_deadline_v1(); captureAfter = boot();
    calls.append(QJsonObject{{"operation", "deadline"}, {"original_until_ns", decimal(originalUntil)},
        {"before_ns", decimal(captureBefore)}, {"after_ns", decimal(captureAfter)}});
    return originalUntil;
}
extern "C" int32_t __wrap_pr_settings_connectivity_stop_v1(PrSettingsConnectivityClient *core) {
    ++stopCalls; const auto before = boot(); const auto result = __real_pr_settings_connectivity_stop_v1(core);
    called("stop", result, before); return result;
}
extern "C" int32_t __wrap_pr_settings_connectivity_join_v1(PrSettingsConnectivityClient *core) {
    const auto before = boot(); const auto result = __real_pr_settings_connectivity_join_v1(core);
    if (result == PR_SETTINGS_OK) ++joinedOK;
    called("join", result, before); return result;
}
extern "C" int32_t __wrap_pr_settings_connectivity_free_v1(PrSettingsConnectivityClient *core) {
    const auto before = boot(); const auto result = __real_pr_settings_connectivity_free_v1(core);
    if (result == PR_SETTINGS_OK) ++freedOK;
    called("free", result, before); return result;
}
int main(int argc, char **argv) {
    assert(getuid() == 11050 && geteuid() == 11050 && getgid() == 11050);
    if (argc == 3 && QByteArray(argv[1]) == "--assert-result") {
        QFile file(QString::fromLocal8Bit(argv[2])); assert(file.open(QIODevice::ReadOnly));
        const auto result = QJsonDocument::fromJson(file.readAll()).object();
        assert(result.value("setup_qualified").toBool());
        assert(result.value("actual_click_qualified").toBool());
        assert(result.value("original_deadline_capture_qualified").toBool());
        assert(result.value("async_close_qualified").toBool());
        assert(result.value("actual_child_wait_EOF").toBool());
        assert(result.value("prior_parent_handles_closed").toBool());
        assert(result.value("exact_Set_and_configured_readback").toBool()
               && "SETTINGS_CONNECTIVITY_QML_ENABLE_CLICK_MUST_SEND_EXACT_SET_AND_READBACK");
        std::puts("PASS normal Settings real QML click/exact wire/readback/async close");
        return 0;
    }
    assert(argc == 4);
    bool valid = false;
    const uint64_t caseUntil = QString::fromLocal8Bit(argv[3]).toULongLong(&valid);
    assert(valid && boot() < caseUntil);
    QGuiApplication::setQuitOnLastWindowClosed(false);
    QGuiApplication app(argc, argv);
    QQuickStyle::setStyle("Basic");
    QJsonObject result{{"setup_qualified", false}, {"actual_click_qualified", false}, {"async_close_qualified", false},
        {"configured_true_readback", false}, {"Qt_version", QString::fromLatin1(qVersion())}};
    std::unique_ptr<SettingsConnectivityClient> connectivity;
    std::unique_ptr<PhoneAppearanceClient> appearance;
    std::unique_ptr<SettingsClient> brightness;
    std::unique_ptr<PhoneNetworkStatusClient> network;
    std::unique_ptr<QQuickView> view;
    std::unique_ptr<QObject> theme;
    QStringList warnings;
    try {
        const uint64_t setupUntil = std::min<uint64_t>(caseUntil, boot() + 1000000000ULL);
        appearance = std::make_unique<PhoneAppearanceClient>(QStringLiteral("/run/settings-connectivity-no-appearance/snapshot.sock"));
        brightness = std::make_unique<SettingsClient>(QString::fromLocal8Bit(argv[1]));
        network = std::make_unique<PhoneNetworkStatusClient>();
        connectivity = std::make_unique<SettingsConnectivityClient>(QString::fromLocal8Bit(argv[1]));
        view = std::make_unique<QQuickView>();
        view->setResizeMode(QQuickView::SizeViewToRootObject);
        QObject::connect(view->engine(), &QQmlEngine::warnings, &app, [&](const QList<QQmlError> &items) {
            for (const auto &item : items) warnings.append(item.toString());
        });
        QQmlComponent component(view->engine(), QUrl::fromLocalFile(QString::fromLocal8Bit(argv[2]) + "/PhoneAppearance.qml"));
        theme.reset(component.createWithInitialProperties({{"source", QVariant::fromValue(appearance.get())}}));
        require(bool(theme), "real shared theme construction failed");
        view->setInitialProperties({{"theme", QVariant::fromValue(theme.get())}, {"settingsClient", QVariant::fromValue(brightness.get())},
            {"networkStatusClient", QVariant::fromValue(network.get())}, {"connectivityClient", QVariant::fromValue(connectivity.get())}});
        view->setSource(QUrl(QStringLiteral("qrc:/settings/SettingsView.qml")));
        require(view->status() == QQuickView::Ready && view->rootObject(), "normal Settings QML must construct before behavior test");
        view->rootObject()->setWidth(1080.0 / 2.625); view->rootObject()->setHeight(2424.0 / 2.625);
        view->resize(int(std::ceil(1080.0 / 2.625)), int(std::ceil(2424.0 / 2.625)));
        require(hidden(*view), "component must remain unshown");
        require(delivered(setupUntil, [&] { return connectivity->available() && connectivity->canControl()
            && connectivity->configured().isValid() && !connectivity->configured().toBool()
            && text(*view, "connectivityState") == QStringLiteral("Local connection: Disabled"); }), "fresh readonly normal SDK/QML state required");
        require(warnings.isEmpty(), "QML warnings are setup failure");
        require(text(*view, "connectivityTitle") == QStringLiteral("Local connection"), "normal title mismatch");
        require(text(*view, "connectivityCaption") == QStringLiteral("Connection state does not indicate internet access."), "normal caption mismatch");
        auto *enable = named(*view, QStringLiteral("connectivityEnable"));
        auto *disable = named(*view, QStringLiteral("connectivityDisable"));
        require(enable->isEnabled() && !disable->isEnabled(), "fresh false state must enable only Enable");
        require(deadlineCalls == 0, "readonly construction/refresh must not capture an action");
        result.insert("setup_qualified", true);
        result.insert("root_logical_width", view->rootObject()->width()); result.insert("root_logical_height", view->rootObject()->height());
        result.insert("view_logical_width", view->width()); result.insert("view_logical_height", view->height());
        result.insert("scale_intent", 2.625); result.insert("native_output_or_window_profile_qualified", false);
        result.insert("click", click(*view, enable)); result.insert("actual_click_qualified", true);
        require(deadlineCalls == 1 && originalUntil >= captureBefore + 1000000000ULL
            && originalUntil <= captureAfter + 1000000000ULL, "one actual Rust original deadline capture required");
        const uint64_t observationUntil = std::min<uint64_t>(caseUntil, boot() + 350000000ULL);
        require(observationUntil < originalUntil, "selected wire observation must precede original action deadline");
        const bool configured = delivered(observationUntil, [&] { return connectivity->available() && !connectivity->pending()
            && connectivity->configured().isValid() && connectivity->configured().toBool()
            && connectivity->status() == QStringLiteral("Connection ready")
            && text(*view, "connectivityStatus") == QStringLiteral("Connection ready")
            && text(*view, "connectivityState") == QStringLiteral("Local connection: Enabled")
            && !enable->isEnabled() && disable->isEnabled(); });
        result.insert("configured_true_readback", configured);
        result.insert("end_available", connectivity->available()); result.insert("end_canControl", connectivity->canControl());
        result.insert("end_configured", QJsonValue::fromVariant(connectivity->configured()));
        result.insert("end_status", connectivity->status());
        result.insert("end_status_text", text(*view, "connectivityStatus"));
        result.insert("observation_until_ns", decimal(observationUntil));
        require(warnings.isEmpty() && hidden(*view) && deadlineCalls == 1, "refresh must preserve single click intent and hidden component");
    } catch (const std::exception &error) { result.insert("error", QString::fromUtf8(error.what())); }
    if (connectivity) {
        int closed = 0, heartbeat = 0; bool insideBegin = false, inlineClosed = false;
        const auto signal = QObject::connect(connectivity.get(), &SettingsConnectivityClient::closed, &app, [&] {
            ++closed; inlineClosed = inlineClosed || insideBegin;
        });
        QTimer::singleShot(0, &app, [&] { ++heartbeat; });
        const uint64_t closeStarted = boot(), closeUntil = closeStarted + 200000000ULL;
        insideBegin = true; connectivity->beginClose(); connectivity->beginClose(); insideBegin = false;
        const uint64_t closeReturned = boot();
        const bool deliveredClosed = delivered(closeUntil, [&] { return closed != 0; });
        const uint64_t completed = boot();
        // The object is retained through closed; no property/getter access follows it.
        bool joinsAfterReturn = true;
        for (const auto &entry : calls) {
            const auto call = entry.toObject();
            if (call.value("operation") == QStringLiteral("join") || call.value("operation") == QStringLiteral("free"))
                joinsAfterReturn = joinsAfterReturn && call.value("before_ns").toString().toULongLong() >= closeReturned;
        }
        const bool qualified = deliveredClosed && closed == 1 && !inlineClosed && heartbeat > 0
            && joinsAfterReturn && stopCalls == 1 && joinedOK == 1 && freedOK == 1 && completed < closeUntil;
        result.insert("async_close_qualified", qualified); result.insert("closed_signals", closed);
        result.insert("event_heartbeat", heartbeat); result.insert("closed_inline", inlineClosed);
        result.insert("close_started_ns", decimal(closeStarted)); result.insert("close_until_ns", decimal(closeUntil));
        result.insert("close_completed_ns", decimal(completed));
        result.insert("close_call_returned_ns", decimal(closeReturned));
        result.insert("all_joins_and_free_after_beginClose_return", joinsAfterReturn);
        QObject::disconnect(signal);
        if (!qualified) std::abort();
    }
    result.insert("deadline_calls", int(deadlineCalls)); result.insert("original_until_ns", decimal(originalUntil));
    result.insert("capture_before_ns", decimal(captureBefore)); result.insert("capture_after_ns", decimal(captureAfter));
    result.insert("actual_FFI_calls", calls); result.insert("all_quick_windows_hidden", !view || hidden(*view));
    view.reset(); theme.reset(); connectivity.reset(); network.reset(); brightness.reset(); appearance.reset();
    std::printf("QT_CLICK_CASE %s\n", QJsonDocument(result).toJson(QJsonDocument::Compact).constData());
    std::fflush(stdout);
    return 0; // Desired product assertion runs only after actual parent wait/EOF/handle closure.
}
