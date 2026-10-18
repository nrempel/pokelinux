// Actual normal QML + real Rust adapter + root normal-service lifecycle.
// Construct the view offscreen; never show a normal application window.
#include "SettingsClient.h"
#include "SettingsConnectivityClient.h"
#include "connectivity-close.h"
#include "PhoneAppearanceClient.h"
#include "PhoneNetworkStatusClient.h"
#include <QCoreApplication>
#include <QEventLoop>
#include <QGuiApplication>
#include <QQmlComponent>
#include <QQuickItem>
#include <QQuickStyle>
#include <QQuickView>
#include <QScopedPointer>
#include <cassert>
#include <cerrno>
#include <cstdio>
#include <cstdlib>
#include <fcntl.h>
#include <poll.h>
#include <time.h>
#include <unistd.h>

static uint64_t boot() {
    timespec value{};
    assert(::clock_gettime(CLOCK_BOOTTIME, &value) == 0);
    return uint64_t(value.tv_sec) * 1000000000ULL + uint64_t(value.tv_nsec);
}
static void orderedCommand(char expected, uint64_t until) {
    char bytes[2]{};
    size_t used = 0;
    while (used != sizeof(bytes)) {
        assert(boot() < until);
        pollfd input{STDIN_FILENO, POLLIN, 0};
        const int result = ::poll(&input, 1, 1);
        if (result < 0 && errno == EINTR) continue;
        assert(result >= 0);
        if (result == 0) continue;
        const auto count = ::read(STDIN_FILENO, bytes + used, sizeof(bytes) - used);
        if (count < 0 && (errno == EAGAIN || errno == EINTR)) continue;
        assert(count > 0);
        used += size_t(count);
    }
    assert(bytes[0] == expected && bytes[1] == '\n' && boot() < until);
}
template<class Predicate> static void deliveredUntil(uint64_t until, Predicate predicate) {
    while (boot() < until) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 5);
        if (predicate()) { assert(boot() < until); return; }
        ::usleep(1000);
    }
    assert(false && "actual network adapter/QML state must arrive before original case deadline");
}
int main(int argc, char **argv) {
    assert(argc == 3 && getuid() == 11050 && geteuid() == 11050);
    char *end = nullptr;
    const uint64_t until = std::strtoull(argv[2], &end, 10);
    assert(end && !*end && boot() < until);
    const int flags = ::fcntl(STDIN_FILENO, F_GETFL);
    assert(flags >= 0 && ::fcntl(STDIN_FILENO, F_SETFL, flags | O_NONBLOCK) == 0);
    QGuiApplication app(argc, argv);
    QQuickStyle::setStyle("Basic");
    PhoneAppearanceClient appearance(QStringLiteral("/run/pokelinux-settings-component-no-appearance/snapshot.sock"));
    QQuickView view;
    QQmlComponent component(view.engine(), QUrl::fromLocalFile(QString::fromLocal8Bit(argv[1]) + "/PhoneAppearance.qml"));
    QScopedPointer<QObject> theme(component.createWithInitialProperties({{"source", QVariant::fromValue(&appearance)}}));
    assert(theme);
    SettingsClient settings(QStringLiteral("/run/pokelinux-settings-component-no-actor/app.sock"));
    PhoneNetworkStatusClient network;
    SettingsConnectivityClient connectivity(QStringLiteral("/run/pokelinux-settings-component-no-actor/app.sock"));
    view.setInitialProperties({{"theme", QVariant::fromValue(theme.data())}, {"settingsClient", QVariant::fromValue(&settings)}, {"networkStatusClient", QVariant::fromValue(&network)}, {"connectivityClient", QVariant::fromValue(&connectivity)}});
    view.setSource(QUrl(QStringLiteral("qrc:/settings/SettingsView.qml")));
    if (view.status() != QQuickView::Ready) {
        for (const auto &error : view.errors()) qWarning("%s", qPrintable(error.toString()));
    }
    assert(view.status() == QQuickView::Ready && view.rootObject());
    const auto text = [&view](const char *name) {
        auto *item = view.rootObject()->findChild<QObject *>(QString::fromLatin1(name));
        assert(item && qobject_cast<QQuickItem *>(item));
        assert(item->property("textFormat").isValid() && item->property("textFormat").toInt() == 0);
        return item->property("text").toString();
    };
    assert(text("localNetworkTitle") == QStringLiteral("Local network status"));
    assert(text("localNetworkCaption") == QStringLiteral("Counts do not indicate internet access."));
    const auto zero = [&] {
        return network.available() && network.localInterfaces().isValid() && network.addressRecords().isValid()
            && network.localInterfaces().toInt() == 0 && network.addressRecords().toInt() == 0
            && text("localNetworkInterfaces") == QStringLiteral("Network interfaces: 0")
            && text("localNetworkAddresses") == QStringLiteral("Address records: 0");
    };
    deliveredUntil(until, zero);
    std::printf("NETWORK_QML_INITIAL_ZERO uid=%u observed_ns=%llu\n", unsigned(getuid()), static_cast<unsigned long long>(boot()));
    std::fflush(stdout);
    // Root driver must Stop + actually wait/drain daemon1 before sending L.
    // No Qt events are delivered during the ordered loss/expiry segment.
    orderedCommand('L', until);
    const uint64_t loss = boot();
    const uint64_t expiredAfter = loss + 2000000001ULL; // Strictly beyond max accepted wire freshness.
    assert(expiredAfter < until);
    while (boot() < expiredAfter) { assert(boot() < until); ::usleep(1000); }
    assert(!network.available() && !network.localInterfaces().isValid() && !network.addressRecords().isValid());
    const uint64_t expired = boot();
    deliveredUntil(until, [&] {
        return !network.available() && text("localNetworkInterfaces") == QStringLiteral("Network interfaces: —")
            && text("localNetworkAddresses") == QStringLiteral("Address records: —");
    });
    std::printf("NETWORK_QML_EXPIRED_DASH loss_ns=%llu expired_getters_ns=%llu rendered_ns=%llu\n",
        static_cast<unsigned long long>(loss), static_cast<unsigned long long>(expired), static_cast<unsigned long long>(boot()));
    std::fflush(stdout);
    // Root driver starts one fresh same normal ELF only after the dash marker.
    orderedCommand('R', until);
    deliveredUntil(until, zero);
    assert(text("localNetworkCaption") == QStringLiteral("Counts do not indicate internet access."));
    std::printf("NETWORK_QML_RECONNECTED_ZERO uid=%u observed_ns=%llu\n", unsigned(getuid()), static_cast<unsigned long long>(boot()));
    closeSettingsConnectivity(connectivity);
    std::puts("PASS real normal service QML 0/0 then expired dash then fresh reconnection 0/0");
    std::fflush(stdout);
}
