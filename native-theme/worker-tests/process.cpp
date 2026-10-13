// Actual Qt + Linux process boundary; no helper executable substitution.
// Only the reap observation/clock error are test-linker injections.
#include <QCoreApplication>
#include <QTemporaryDir>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QThread>
#include <memory>
#include <cstdio>
#include <cerrno>
#include <csignal>
#include <fcntl.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>
#include "PhoneThemeFiles.h"

bool rejectReapReport=false, rejectBootClock=false;
extern "C" bool __real__ZN8QProcess15waitForFinishedEi(QProcess *,int);
extern "C" bool __wrap__ZN8QProcess15waitForFinishedEi(QProcess *p,int millis) {
    const bool observed=__real__ZN8QProcess15waitForFinishedEi(p,millis);
    if (rejectReapReport && millis==1000) {
        std::fprintf(stderr,"INJECTED_REAP_REPORT: actual_wait=%d actual_state=%d; return false\n",observed,int(p->state()));
        std::fflush(stderr);
        return false;
    }
    return observed;
}
extern "C" int __real_clock_gettime(clockid_t,timespec *);
extern "C" int __wrap_clock_gettime(clockid_t id,timespec *value) {
    if (rejectBootClock && id==CLOCK_BOOTTIME) { errno=EIO; return -1; }
    return __real_clock_gettime(id,value);
}
class NativeThemeWorkerProbe {
public:
    static QProcess &process(PhoneThemeFiles &f) { return f.fontMatch; }
    static QTimer &timer(PhoneThemeFiles &f) { return f.fontDeadline; }
};
static qint64 bootMillis() {
    timespec value{};
    if (__real_clock_gettime(CLOCK_BOOTTIME,&value)!=0) return -1;
    return qint64(value.tv_sec)*1000+value.tv_nsec/1000000;
}
static bool writeFile(const QString &path,const QByteArray &data) {
    QDir().mkpath(QFileInfo(path).absolutePath());
    QFile f(path); return f.open(QIODevice::WriteOnly) && f.write(data)==data.size();
}
static int report(QJsonObject state,int status) {
    const auto bytes=QJsonDocument(state).toJson(QJsonDocument::Compact);
    std::printf("%s\n",bytes.constData());std::fflush(stdout);return status;
}
int main(int argc,char **argv) {
    QCoreApplication app(argc,argv);
    if (argc!=2) return 64;
    const QString mode=QString::fromLocal8Bit(argv[1]);
    QTemporaryDir home;
    if (!home.isValid()) return 70;
    const QString fifo=home.path()+"/font-input.fifo";
    const bool startFault=mode=="clock-start-failure";
    if (!startFault && ::mkfifo(QFile::encodeName(fifo).constData(),0600)!=0) return 70;
    const QByteArray configuration=startFault ? QByteArray("<fontconfig/>\n")
        : QByteArray("<fontconfig><include>")+fifo.toUtf8()+"</include></fontconfig>\n";
    if (!writeFile(home.path()+"/.config/fontconfig/fonts.conf",configuration)
        || !writeFile(home.path()+"/.local/state/omarchy/current/theme/colors.toml","background='#101010'\nforeground='#eeeeee'\n")) return 70;
    rejectBootClock=startFault;
    const qint64 began=bootMillis();
    auto files=std::make_unique<PhoneThemeFiles>(home.path());
    auto &child=NativeThemeWorkerProbe::process(*files);
    const bool didStart=child.state()!=QProcess::NotRunning;
    if (startFault) {
        if (didStart) child.waitForFinished(3000);
        const bool available=files->snapshot().value("fontResolutionAvailable").toBool();
        return report({{"mode",mode},{"injected_boot_clock_error",true},{"child_started",didStart},
            {"available",available}},(!didStart&&!available)?0:101);
    }
    if (!child.waitForStarted(2000)) return 70;
    const auto pid=child.processId();
    if (child.program()!="/usr/bin/fc-match"||pid<=0) return 70;
    int writer=-1;
    const qint64 openEnd=bootMillis()+1500;
    while (bootMillis()<openEnd && writer<0) {
        writer=::open(QFile::encodeName(fifo).constData(),O_WRONLY|O_NONBLOCK|O_CLOEXEC);
        if (writer<0) QThread::msleep(2);
    }
    if (writer<0||child.state()!=QProcess::Running) return 70;
    if (mode=="reap-failure"||mode=="normal-destruction") {
        rejectReapReport=mode=="reap-failure";
        std::fprintf(stderr,"ACTUAL_CHILD: pid=%lld program=/usr/bin/fc-match private_fontconfig_fifo=true\n",static_cast<long long>(pid));std::fflush(stderr);
        files.reset();
        ::close(writer);
        const bool absent=(::kill(pid,0)!=0 && errno==ESRCH);
        return report({{"mode",mode},{"owner_destroyed",true},{"actual_child_absent",absent}},absent?0:101);
    }
    int timerCallbacks=0;
    QObject::connect(&NativeThemeWorkerProbe::timer(*files),&QTimer::timeout,&app,[&]{++timerCallbacks;});
    if (mode=="late-completion") {
        // Real elapsed boot time; no clock jump, suspend or timer-expiry injection.
        timespec duration{1,800000000};
        while (::clock_nanosleep(CLOCK_MONOTONIC,0,&duration,&duration)==EINTR) {}
    } else if (mode=="clock-completion-failure") rejectBootClock=true;
    else if (mode!="within-budget") return 64;
    constexpr char input[]="<fontconfig/>\n";
    if (::write(writer,input,sizeof(input)-1)!=ssize_t(sizeof(input)-1)) return 70;
    ::close(writer);
    // Qt wait drives original process IO/completion, without delivering app timers.
    const bool completed=child.waitForFinished(3000);
    const bool available=files->snapshot().value("fontResolutionAvailable").toBool();
    const qint64 elapsed=bootMillis()-began;
    QJsonObject state{{"mode",mode},{"actual_completion_observed",completed},{"actual_child_absent",!QFile::exists("/proc/"+QString::number(pid))},
        {"elapsed_boottime_ms",elapsed},{"qt_timer_callbacks",timerCallbacks},{"available",available},
        {"family",files->snapshot().value("resolvedFontFamily").toString()},
        {"injected_boot_clock_error",rejectBootClock}};
    if (!completed||timerCallbacks!=0) return report(state,70);
    if (mode=="within-budget" && elapsed>=1500) return report(state,70);
    const bool expected=mode=="within-budget";
    return report(state,available==expected?0:101);
}
