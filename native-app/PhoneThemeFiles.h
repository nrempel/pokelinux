#pragma once
#include <QObject>
#include <QVariantMap>
#include <QFileSystemWatcher>
#include <QTimer>
#include <QProcess>

// Trusted local appearance files only. Never used for enrollment or commands.
class PhoneThemeFiles final : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantMap snapshot READ snapshot NOTIFY changed)
    Q_PROPERTY(bool available READ available NOTIFY changed)
public:
    explicit PhoneThemeFiles(const QString &home, QObject *parent = nullptr);
    ~PhoneThemeFiles() override;
    QVariantMap snapshot() const { return current; }
    bool available() const { return valid; }
signals:
    void changed();
private:
    void reload();
    void watchPaths();
    void requestFont(const QString &configuration);
    void startFontMatch();
    void finishFontMatch(int exitCode, QProcess::ExitStatus status);
    QString homePath;
    QString themePath;
    QString overridePath;
    QString fontPath;
    QString wantedFontConfiguration;
    QString activeFontConfiguration;
    QString resolvedFontFamily = "monospace";
    QByteArray fontOutput;
    bool fontRequested = false;
    bool fontPending = false;
    bool fontResolutionAvailable = false;
    bool fontOutputValid = true;
    qint64 fontStartedBootMs = -1;
    QProcess fontMatch;
    QTimer fontDeadline;
    QVariantMap current;
    bool valid = false;
    QFileSystemWatcher watcher;
    QTimer settle;
};
