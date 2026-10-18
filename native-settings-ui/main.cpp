#include "SettingsClient.h"
#include "PhoneAppearanceClient.h"
#include "PhoneNetworkStatusClient.h"
#include <QGuiApplication>
#include <QQuickView>
#include <QQmlComponent>
#include <QQuickStyle>
#include <QScopedPointer>
#include <QSocketNotifier>
#include <cerrno>
#include <csignal>
#include <sys/signalfd.h>
#include <unistd.h>

int main(int argc, char **argv) {
    if (argc != 7) return 64;
    if (getuid() != 11050 || geteuid() != 11050) return 65;
    for (int i = 1; i < 7; ++i) if (argv[i][0] != '/') return 64;
    // Block before Qt or the Rust client starts any worker thread.
    sigset_t termination;
    if (::sigemptyset(&termination) || ::sigaddset(&termination, SIGTERM)
        || ::sigprocmask(SIG_BLOCK, &termination, nullptr)) return 70;
    struct TerminationFd {
        int value;
        ~TerminationFd() { if (value >= 0) ::close(value); }
    } terminationFd{::signalfd(-1, &termination, SFD_NONBLOCK | SFD_CLOEXEC)};
    if (terminationFd.value < 0) return 70;
    qputenv("HOME", argv[1]);
    qputenv("XDG_RUNTIME_DIR", argv[2]);
    qputenv("WAYLAND_DISPLAY", argv[3]);
    qputenv("QT_QPA_PLATFORM", "wayland");
    qputenv("PATH", "/usr/bin");
    QGuiApplication::setApplicationName(QStringLiteral("pokelinux-settings"));
    QGuiApplication app(argc, argv);
    QQuickStyle::setStyle("Basic");
    QSocketNotifier terminate(terminationFd.value, QSocketNotifier::Read);
    QObject::connect(&terminate, &QSocketNotifier::activated, &app, [&] {
        signalfd_siginfo notification{};
        const auto count = ::read(terminationFd.value, &notification, sizeof(notification));
        if (count < 0 && (errno == EAGAIN || errno == EINTR)) return;
        if (count == ssize_t(sizeof(notification)) && notification.ssi_signo == SIGTERM) app.quit();
        else app.exit(70);
    });
    PhoneAppearanceClient appearance(QString::fromLocal8Bit(argv[4]));
    QQuickView view;
    QQmlComponent component(view.engine(), QUrl::fromLocalFile(QString::fromLocal8Bit(argv[6]) + "/PhoneAppearance.qml"));
    QScopedPointer<QObject> theme(component.createWithInitialProperties({{"source", QVariant::fromValue(&appearance)}}));
    if (!theme) return 66;
    SettingsClient client(QString::fromLocal8Bit(argv[5]));
    view.setTitle(QStringLiteral("Settings"));
    PhoneNetworkStatusClient networkStatus;
    view.setInitialProperties({{"theme", QVariant::fromValue(theme.data())}, {"settingsClient", QVariant::fromValue(&client)}, {"networkStatusClient", QVariant::fromValue(&networkStatus)}});
    view.setResizeMode(QQuickView::SizeRootObjectToView);
    view.setSource(QUrl(QStringLiteral("qrc:/settings/SettingsView.qml")));
    if (view.status() != QQuickView::Ready) return 67;
    view.showFullScreen();
    return app.exec();
}
