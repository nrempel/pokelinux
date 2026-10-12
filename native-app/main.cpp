#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQuickStyle>
#include <QWindow>
#include <QPointer>
#include <QFileInfo>
#include <QDir>
#include <QCommandLineParser>
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusReply>
#include <unistd.h>
#include "PhoneRpc.h"
#include "PhonePower.h"
#include "PhoneThemeFiles.h"

class PhoneActivation final : public QObject {
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.omarchy.Phone")
public:
    QPointer<QWindow> window;
public slots:
    bool Activate() {
        if (!window) return false;
        window->show();
        window->requestActivate();
        return true;
    }
};

int main(int argc, char **argv) {
    const auto runtime=qEnvironmentVariable("XDG_RUNTIME_DIR");
    QGuiApplication app(argc,argv);
    QCoreApplication::setApplicationName("omarchy-phone-app");
    QGuiApplication::setApplicationDisplayName("Phone");
    QGuiApplication::setDesktopFileName("org.omarchy.Phone");
    QCommandLineParser parser;
    parser.setApplicationDescription("Omarchy phone controls");
    parser.addHelpOption();
    parser.process(app);
    if (!parser.positionalArguments().isEmpty()) {
        qCritical("Phone accepts no command or phone-number arguments");
        return 2;
    }
    const QFileInfo runtimeInfo(runtime);
    if (runtime.isEmpty() || !QDir::isAbsolutePath(runtime) || !runtimeInfo.isDir()
        || runtimeInfo.isSymLink() || runtimeInfo.ownerId()!=getuid()
        || (runtimeInfo.permissions() & (QFile::ReadGroup|QFile::WriteGroup|QFile::ExeGroup
            |QFile::ReadOther|QFile::WriteOther|QFile::ExeOther))) {
        qCritical("A private, owned XDG_RUNTIME_DIR is required");
        return 2;
    }
    auto bus=QDBusConnection::sessionBus();
    if (!bus.isConnected()) {
        qCritical("A user D-Bus session is required");
        return 2;
    }
    if (!bus.registerService("org.omarchy.Phone")) {
        QDBusInterface existing("org.omarchy.Phone","/org/omarchy/Phone","org.omarchy.Phone",bus);
        existing.setTimeout(2000);
        const QDBusReply<bool> reply=existing.call("Activate");
        return reply.isValid() && reply.value() ? 0 : 1;
    }
    PhoneActivation activation;
    if (!bus.registerObject("/org/omarchy/Phone",&activation,QDBusConnection::ExportAllSlots)) return 2;
    // The CLI is a matching component in the same installation prefix. No
    // PATH lookup, shell command, enrollment material or dial arguments.
    PhoneRpc rpc(QCoreApplication::applicationDirPath()+"/omarchy-phone");
    PhonePower power;
    PhoneThemeFiles files(QDir::homePath());
    QQuickStyle::setStyle("Basic");
    QQmlApplicationEngine engine;
    engine.setInitialProperties({{"rpc",QVariant::fromValue(&rpc)},
        {"power",QVariant::fromValue(&power)}, {"themeFiles",QVariant::fromValue(&files)},
        {"socketPath",runtime+"/omarchy-phone/bridge.sock"}});
    engine.load(QUrl("qrc:/phone/Main.qml"));
    if (engine.rootObjects().size()!=1) return 2;
    activation.window=qobject_cast<QWindow*>(engine.rootObjects().first());
    if (!activation.window) return 2;
    return app.exec();
}

#include "main.moc"
