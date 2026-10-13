#include "PhoneThemeFiles.h"
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QSet>
#include <QStringDecoder>
#include <QProcessEnvironment>
#include <cstdlib>
#include <limits>
#include <time.h>

namespace {
constexpr qint64 maxFileBytes = 256 * 1024;
constexpr int fontBudgetMs = 1500;
qint64 bootTimeMs() {
    timespec value{};
    if (clock_gettime(CLOCK_BOOTTIME,&value) != 0 || value.tv_sec < 0
        || value.tv_sec > std::numeric_limits<qint64>::max()/1000
        || value.tv_nsec < 0 || value.tv_nsec >= 1000000000) return -1;
    const auto seconds = static_cast<qint64>(value.tv_sec)*1000;
    const auto millis = value.tv_nsec/1000000;
    if (seconds > std::numeric_limits<qint64>::max()-millis) return -1;
    return seconds+millis;
}
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
      overridePath(homePath + "/.config/omarchy/shell.toml"),
      fontPath(homePath + "/.config/fontconfig/fonts.conf") {
    fontDeadline.setSingleShot(true);
    fontDeadline.setInterval(fontBudgetMs);
    connect(&fontDeadline, &QTimer::timeout, this, [this] { fontOutputValid = false; fontMatch.kill(); });
    connect(&fontMatch, &QProcess::readyReadStandardOutput, this, [this] {
        fontOutput += fontMatch.readAllStandardOutput();
        if (fontOutput.size() > 4096) { fontOutputValid = false; fontOutput.clear(); fontMatch.kill(); }
    });
    connect(&fontMatch, &QProcess::readyReadStandardError, this, [this] { fontMatch.readAllStandardError(); });
    connect(&fontMatch, qOverload<int,QProcess::ExitStatus>(&QProcess::finished), this, &PhoneThemeFiles::finishFontMatch);
    connect(&fontMatch, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart) finishFontMatch(-1,QProcess::CrashExit);
    });
    settle.setSingleShot(true);
    settle.setInterval(50);
    connect(&watcher, &QFileSystemWatcher::directoryChanged, this, [this](const QString &path) {
        if (path == QFileInfo(fontPath).absolutePath()) fontRequested = false;
        settle.start();
    });
    connect(&watcher, &QFileSystemWatcher::fileChanged, this, [this](const QString &path) {
        if (path == fontPath) fontRequested = false;
        settle.start();
    });
    connect(&settle, &QTimer::timeout, this, &PhoneThemeFiles::reload);
    reload();
}

void PhoneThemeFiles::watchPaths() {
    QSet<QString> desired;
    // Watch every existing lexical parent, including parents of missing paths.
    // A symlink replacement or creation of an initially absent tree then wakes
    // the reader without periodic polling.
    const QStringList inputs{themePath + "/colors.toml", themePath + "/shell.toml", overridePath, fontPath};
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
    QString colors, shell, user, fontConfiguration;
    const bool fontReadable = readText(fontPath, true, fontConfiguration);
    if (fontReadable) requestFont(fontConfiguration);
    else {
        fontResolutionAvailable = false;
        fontRequested = false;
        fontPending = false;
        fontOutputValid = false;
        if (fontMatch.state() != QProcess::NotRunning) fontMatch.kill();
    }
    const bool loaded = !target.isEmpty() && QFileInfo(target).isDir()
        && readText(target + "/colors.toml", false, colors)
        && readText(target + "/shell.toml", true, shell)
        && readText(overridePath, true, user)
        && target == QFileInfo(themePath).canonicalFilePath();
    const QVariantMap next{{"colors", colors}, {"shell", shell},
                           {"userShell", user}, {"themeDirectory", target},
                           {"resolvedFontFamily", resolvedFontFamily},
                           {"fontResolutionAvailable", fontResolutionAvailable}};
    if (valid != loaded || (loaded && current != next)) {
        valid = loaded;
        if (loaded) current = next;
        emit changed();
    }
}


PhoneThemeFiles::~PhoneThemeFiles() {
    // The only helper is a fixed, direct fc-match child; no shell or detached worker.
    disconnect(&fontMatch,nullptr,this,nullptr);
    disconnect(&watcher,nullptr,this,nullptr);
    fontDeadline.stop();
    settle.stop();
    if (fontMatch.state() != QProcess::NotRunning) {
        fontMatch.kill();
        // Never enter QProcess's implicit destructor wait with occupied or
        // uncertain ownership. Aborting is a local failure disposition, not
        // evidence that an unobserved kernel child was cleaned up.
        if (!fontMatch.waitForFinished(1000) || fontMatch.state() != QProcess::NotRunning)
            std::abort();
    }
}

void PhoneThemeFiles::requestFont(const QString &configuration) {
    if (fontRequested && wantedFontConfiguration == configuration) return;
    fontRequested = true;
    wantedFontConfiguration = configuration;
    fontPending = true;
    fontResolutionAvailable = false;
    if (fontMatch.state() != QProcess::NotRunning) {
        // One worker maximum. Its finished callback starts only the latest request.
        fontOutputValid = false;
        fontMatch.kill();
    } else startFontMatch();
}

void PhoneThemeFiles::startFontMatch() {
    fontPending = false;
    activeFontConfiguration = wantedFontConfiguration;
    fontOutput.clear();
    fontOutputValid = true;
    fontStartedBootMs = bootTimeMs();
    if (fontStartedBootMs < 0) {
        fontOutputValid = false;
        fontResolutionAvailable = false;
        return;
    }
    auto environment = QProcessEnvironment::systemEnvironment();
    environment.insert("HOME",homePath);
    environment.insert("XDG_CONFIG_HOME",homePath+"/.config");
    fontMatch.setProcessEnvironment(environment);
    fontMatch.start("/usr/bin/fc-match",{"-f","%{family[0]}","monospace"},QIODevice::ReadOnly);
    fontDeadline.start();
}

void PhoneThemeFiles::finishFontMatch(int exitCode, QProcess::ExitStatus status) {
    fontDeadline.stop();
    // Qt timers assist cancellation. Completion itself must qualify against
    // boot time, including time in suspend or with the event loop undelivered.
    const auto completedBootMs = bootTimeMs();
    const bool timely = fontStartedBootMs >= 0 && completedBootMs >= fontStartedBootMs
        && completedBootMs-fontStartedBootMs < fontBudgetMs;
    fontOutput += fontMatch.readAllStandardOutput();
    QStringDecoder decoder(QStringDecoder::Utf8,QStringConverter::Flag::Stateless);
    const QString decoded = decoder.decode(fontOutput);
    const auto family = decoded.trimmed();
    QString configuration;
    const bool currentRequest = readText(fontPath,true,configuration)
        && configuration == activeFontConfiguration
        && activeFontConfiguration == wantedFontConfiguration;
    if (currentRequest && timely && fontOutputValid && exitCode == 0 && status == QProcess::NormalExit
        && fontOutput.size() <= 4096 && !decoder.hasError() && !family.isEmpty()
        && family.size() <= 512 && !family.contains('\n') && !family.contains('\0')) {
        resolvedFontFamily = family;
        fontResolutionAvailable = true;
    }
    if (fontPending) startFontMatch();
    // Palette and draft objects stay in place. A concrete family avoids Qt's
    // cached monospace alias surviving original omarchy-font-set's file update.
    reload();
}
