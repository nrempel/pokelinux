#include "PhoneThemeFiles.h"
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QSet>
#include <QStringDecoder>

namespace {
constexpr qint64 maxFileBytes = 256 * 1024;
bool readText(const QString &path, bool optional, QString &text) {
    const QFileInfo info(path);
    if (!info.exists() && !info.isSymLink()) {
        text.clear();
        return optional;
    }
    if (!info.isFile() || info.size() > maxFileBytes) return false;
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) return false;
    const auto bytes = file.read(maxFileBytes + 1);
    if (file.error() != QFileDevice::NoError || bytes.size() > maxFileBytes
        || (!optional && bytes.isEmpty()) || bytes.contains('\0')) return false;
    QStringDecoder decoder(QStringDecoder::Utf8, QStringConverter::Flag::Stateless);
    text = decoder.decode(bytes);
    return !decoder.hasError();
}
}

PhoneThemeFiles::PhoneThemeFiles(const QString &home, QObject *parent)
    : QObject(parent), homePath(QDir(home).absolutePath()),
      themePath(homePath + "/.local/state/omarchy/current/theme"),
      overridePath(homePath + "/.config/omarchy/shell.toml") {
    settle.setSingleShot(true);
    settle.setInterval(50);
    connect(&watcher, &QFileSystemWatcher::directoryChanged, this, [this] { settle.start(); });
    connect(&watcher, &QFileSystemWatcher::fileChanged, this, [this] { settle.start(); });
    connect(&settle, &QTimer::timeout, this, &PhoneThemeFiles::reload);
    reload();
}

void PhoneThemeFiles::watchPaths() {
    QSet<QString> desired;
    // Watch every existing lexical parent, including parents of missing paths.
    // A symlink replacement or creation of an initially absent tree then wakes
    // the reader without periodic polling.
    const QStringList inputs{themePath + "/colors.toml", themePath + "/shell.toml", overridePath};
    for (auto path : inputs) {
        while (path == homePath || path.startsWith(homePath + '/')) {
            if (QFileInfo::exists(path)) desired.insert(path);
            if (path == homePath) break;
            path = QFileInfo(path).absolutePath();
        }
    }
    const auto resolved = QFileInfo(themePath).canonicalFilePath();
    if (!resolved.isEmpty()) {
        desired.insert(resolved);
        desired.insert(QFileInfo(resolved).absolutePath());
        for (const auto &name : {"colors.toml", "shell.toml"}) {
            const auto path = resolved + '/' + name;
            if (QFileInfo::exists(path)) desired.insert(path);
        }
    }
    const auto watched = watcher.files() + watcher.directories();
    for (const auto &path : watched) {
        if (!desired.contains(path)) watcher.removePath(path);
        else desired.remove(path);
    }
    if (!desired.isEmpty()) watcher.addPaths(desired.values());
}

void PhoneThemeFiles::reload() {
    watchPaths();
    // Resolve once, so a selection change cannot read colors from one target
    // and shell settings from another. Check selection again before publishing.
    const auto target = QFileInfo(themePath).canonicalFilePath();
    QString colors, shell, user;
    const bool loaded = !target.isEmpty() && QFileInfo(target).isDir()
        && readText(target + "/colors.toml", false, colors)
        && readText(target + "/shell.toml", true, shell)
        && readText(overridePath, true, user)
        && target == QFileInfo(themePath).canonicalFilePath();
    const QVariantMap next{{"colors", colors}, {"shell", shell},
                           {"userShell", user}, {"themeDirectory", target}};
    if (valid != loaded || (loaded && current != next)) {
        valid = loaded;
        if (loaded) current = next;
        emit changed();
    }
}
