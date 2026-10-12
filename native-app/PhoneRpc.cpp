#include "PhoneRpc.h"
#include <QDir>
#include <QJsonDocument>
#include <QUuid>
#include <utility>
#include <time.h>

PhoneRpc::PhoneRpc(QString path, QObject *parent)
    : QObject(parent), executable(std::move(path)) {
    process.setStandardErrorFile(QProcess::nullDevice());
    deadline.setSingleShot(true);
    deadline.setInterval(4000);
    connect(&deadline, &QTimer::timeout, this, [this] {
        failed = true;
        process.kill();
    });
    connect(&process, &QProcess::started, this, [this] {
        if (failed) { process.kill(); return; }
        if (process.write(input) != input.size()) {
            failed = true;
            process.kill();
        }
        input.clear();
        process.closeWriteChannel();
    });
    connect(&process, &QProcess::readyReadStandardOutput, this, &PhoneRpc::readOutput);
    connect(&process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart) finish(2);
        else { failed = true; process.kill(); }
    });
    connect(&process, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), this,
        [this](int code, QProcess::ExitStatus status) {
            readOutput();
            if (failed || status != QProcess::NormalExit || (code != 0 && code != 2 && code != 3 && code != 4)) code = 3;
            if (code == 0 || code == 4) {
                QJsonParseError error;
                const auto reply = QJsonDocument::fromJson(output, &error);
                if (error.error != QJsonParseError::NoError || !reply.isObject()) code = 3;
            }
            finish(code);
        });
}
PhoneRpc::~PhoneRpc() {
    // QProcess destruction can emit signals while its containing object's
    // members are already being destroyed. Disconnect before tearing down
    // buffers/timers; closing the UI must never deliver a late completion.
    disconnect(&process, nullptr, this, nullptr);
    deadline.stop();
    if (process.state() != QProcess::NotRunning) {
        process.kill();
        process.waitForFinished(1000);
    }
}
double PhoneRpc::boottimeMs() const {
    timespec value{};
    if (clock_gettime(CLOCK_BOOTTIME, &value) != 0) return -1;
    return static_cast<double>(value.tv_sec) * 1000.0 + value.tv_nsec / 1000000.0;
}
QString PhoneRpc::newId() const {
    return QUuid::createUuid().toString(QUuid::Id128);
}
bool PhoneRpc::request(const QString &socket, const QString &frame, const QString &token) {
    if (token.isEmpty() || token.size() > 128 || active || !QDir::isAbsolutePath(executable) || !QDir::isAbsolutePath(socket)) return false;
    const auto bytes = frame.toUtf8();
    if (bytes.isEmpty() || bytes.size() > 16384 || !bytes.endsWith('\n')) return false;
    callerToken = token;
    active = true;
    failed = false;
    input = bytes;
    output.clear();
    emit busyChanged();
    process.setProgram(executable);
    process.setArguments({QStringLiteral("--socket"), socket, QStringLiteral("rpc")});
    deadline.start();
    process.start();
    return true;
}
void PhoneRpc::readOutput() {
    if (!active) return;
    output += process.read(16385 - output.size());
    if (output.size() > 16384) {
        failed = true;
        process.kill();
    }
}
void PhoneRpc::finish(int status) {
    if (!active) return;
    deadline.stop();
    const auto token = callerToken;
    callerToken.clear();
    active = false;
    input.clear();
    const auto reply = (status == 0 || status == 4) ? output : QByteArray();
    output.clear();
    emit busyChanged();
    emit completed(status, reply, token);
}
