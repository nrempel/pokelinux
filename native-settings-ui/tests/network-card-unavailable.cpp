// C++/Rust ABI and real adapter/QML construction only, without an app window.
#include "SettingsClient.h"
#include "PhoneAppearanceClient.h"
#include "PhoneNetworkStatusClient.h"
#include <QGuiApplication>
#include <QQuickItem>
#include <QQuickView>
#include <QQuickStyle>
#include <QQmlComponent>
#include <QScopedPointer>
#include <cassert>
#include <chrono>
#include <cstddef>
#include <cstdio>
#include <thread>
#include <unistd.h>

int main(int argc, char **argv) {
    assert(argc == 2 && getuid() == 11050 && geteuid() == 11050);
    static_assert(sizeof(PrSettingsView) == 48 && alignof(PrSettingsView) == 8);
    static_assert(offsetof(PrSettingsView, revision) == 0);
    static_assert(offsetof(PrSettingsView, status) == 32);
    static_assert(offsetof(PrSettingsView, observed_percent) == 40);
    PrSettingsView sentinel{};
    sentinel.revision = 91;
    assert(pr_settings_client_view_v1(nullptr, &sentinel) == PR_SETTINGS_INVALID && sentinel.revision == 91);
    assert(pr_settings_client_new_v1(nullptr, 0, nullptr) == PR_SETTINGS_INVALID);
    PrSettingsClient *core = nullptr;
    const QByteArray missing = "/run/pokelinux-settings-component-no-actor/app.sock";
    assert(pr_settings_client_new_v1(reinterpret_cast<const uint8_t *>(missing.constData()), size_t(missing.size()), &core) == PR_SETTINGS_OK);
    const auto original = core;
    assert(pr_settings_client_new_v1(reinterpret_cast<const uint8_t *>(missing.constData()), size_t(missing.size()), &core) == PR_SETTINGS_INVALID && core == original);
    assert(pr_settings_client_view_v1(core, nullptr) == PR_SETTINGS_INVALID);
    assert(pr_settings_client_view_v1(core, &sentinel) == PR_SETTINGS_OK);
    assert(!sentinel.available && !sentinel.can_set && sentinel.observed_percent == -1);
    assert(pr_settings_client_set_v1(core, 75) == PR_SETTINGS_REFUSED);
    assert(pr_settings_client_free_v1(core) == PR_SETTINGS_REFUSED);
    const auto started = std::chrono::steady_clock::now();
    assert(pr_settings_client_stop_v1(core) == PR_SETTINGS_OK);
    for (;;) {
        const auto code = pr_settings_client_join_v1(core);
        if (code == PR_SETTINGS_OK) break;
        assert(code == PR_SETTINGS_WAIT);
        assert(std::chrono::steady_clock::now() - started < std::chrono::milliseconds(200));
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    assert(pr_settings_client_view_v1(core, &sentinel) == PR_SETTINGS_OK && sentinel.status == PR_SETTINGS_STOPPED);
    assert(pr_settings_client_free_v1(core) == PR_SETTINGS_OK);
    std::puts("PASS C++ C ABI layout/refusal/actual_join");

    QGuiApplication app(argc, argv);
    QQuickStyle::setStyle("Basic");
    PhoneAppearanceClient appearance(QStringLiteral("/run/pokelinux-settings-component-no-appearance/snapshot.sock"));
    QQuickView view;
    QQmlComponent component(view.engine(), QUrl::fromLocalFile(QString::fromLocal8Bit(argv[1]) + "/PhoneAppearance.qml"));
    QScopedPointer<QObject> theme(component.createWithInitialProperties({{"source", QVariant::fromValue(&appearance)}}));
    assert(theme);
    SettingsClient client(QString::fromUtf8(missing));
    assert(!client.available() && !client.canSet() && !client.confirmedPercent().isValid());
    PhoneNetworkStatusClient networkStatus;
    view.setInitialProperties({{"theme", QVariant::fromValue(theme.data())}, {"settingsClient", QVariant::fromValue(&client)}, {"networkStatusClient", QVariant::fromValue(&networkStatus)}});
    view.setSource(QUrl(QStringLiteral("qrc:/settings/SettingsView.qml")));
    if (view.status() != QQuickView::Ready) {
        for (const auto &error : view.errors()) qWarning("%s", qPrintable(error.toString()));
    }
    assert(view.status() == QQuickView::Ready && view.rootObject());
    for (const char *name : {"confirmedBrightness", "brightnessStatus"}) {
        auto *item = view.rootObject()->findChild<QObject *>(QString::fromLatin1(name));
        assert(item && item->property("textFormat").toInt() == 0);
    }
    for (const char *name : {"brightness25", "brightness75"}) {
        auto *item = view.rootObject()->findChild<QObject *>(QString::fromLatin1(name));
        assert(item && !item->property("enabled").toBool());
    }
    assert(client.status() == QStringLiteral("Brightness unavailable"));
    std::puts("PASS real adapters and normal QML construction without window");

    // Normal embedded QML, actual existing adapters, no future client or fake
    // properties. Keep every brightness control above as the original baseline.
    std::fflush(stdout);
    const auto requireCardText = [&view](const char *name, const QString &expected) {
        auto *item = view.rootObject()->findChild<QObject *>(QString::fromLatin1(name));
        assert(item && "normal Settings QML must contain the local network card");
        assert(qobject_cast<QQuickItem *>(item));
        assert(item->property("textFormat").isValid() && item->property("textFormat").toInt() == 0);
        assert(item->property("text").toString() == expected);
    };
    requireCardText("localNetworkTitle", QStringLiteral("Local network status"));
    requireCardText("localNetworkInterfaces", QStringLiteral("Network interfaces: —"));
    requireCardText("localNetworkAddresses", QStringLiteral("Address records: —"));
    requireCardText("localNetworkCaption", QStringLiteral("Counts do not indicate internet access."));
    std::puts("PASS normal Settings local network unavailable card");
}
