#pragma once
#include <QObject>
#include <QVariantMap>
#include <QFileSystemWatcher>
#include <QTimer>

// Trusted local appearance files only. Never used for enrollment or commands.
class PhoneThemeFiles final : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantMap snapshot READ snapshot NOTIFY changed)
    Q_PROPERTY(bool available READ available NOTIFY changed)
public:
    explicit PhoneThemeFiles(const QString &home, QObject *parent = nullptr);
    QVariantMap snapshot() const { return current; }
    bool available() const { return valid; }
signals:
    void changed();
private:
    void reload();
    void watchPaths();
    QString homePath;
    QString themePath;
    QString overridePath;
    QVariantMap current;
    bool valid = false;
    QFileSystemWatcher watcher;
    QTimer settle;
};
