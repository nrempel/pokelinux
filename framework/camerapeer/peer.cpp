// Synthetic peer, no vendor binary, allocator or camera hardware access.
#include <aidl/org/pokelinux/cameratest/BnCameraBoundaryProbe.h>
#include <android/binder_ibinder_jni.h>
#include <dirent.h>
#include <fcntl.h>
#include <jni.h>
#include <mutex>
#include <signal.h>
#include <unistd.h>
using aidl::android::hardware::common::NativeHandle;
using aidl::org::pokelinux::cameratest::BnCameraBoundaryProbe;
static int fd_count() {
    DIR* dir = opendir("/proc/self/fd");
    if (!dir) return -1;
    int count = 0;
    while (auto* entry = readdir(dir)) if (entry->d_name[0] != '.') ++count;
    closedir(dir);
    return count;
}
class Peer final : public BnCameraBoundaryProbe {
    std::mutex lock;
    ndk::ScopedFileDescriptor retained;
public:
    ndk::ScopedAStatus submit(const NativeHandle& buffer, int32_t mode, int32_t* count) override {
        std::lock_guard<std::mutex> guard(lock);
        if (buffer.fds.size() != 1 || buffer.ints != std::vector<int32_t>{37, 4} || retained.get() >= 0)
            return ndk::ScopedAStatus::fromServiceSpecificError(23);
        retained.set(fcntl(buffer.fds[0].get(), F_DUPFD_CLOEXEC, 0));
        if (retained.get() < 0) return ndk::ScopedAStatus::fromServiceSpecificError(24);
        *count = 1;
        if (mode == 2) {
            // Observable receipt occurs before deliberate process death/no reply.
            if (write(retained.get(), "D", 1) != 1) return ndk::ScopedAStatus::fromServiceSpecificError(25);
            kill(getpid(), SIGKILL);
            _exit(99);
        }
        if (mode == 1) return ndk::ScopedAStatus::fromServiceSpecificErrorWithMessage(17, "retained before error");
        return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus returnedHandle(NativeHandle* out) override {
        std::lock_guard<std::mutex> guard(lock);
        if (retained.get() < 0) return ndk::ScopedAStatus::fromServiceSpecificError(26);
        int copy = fcntl(retained.get(), F_DUPFD_CLOEXEC, 0);
        if (copy < 0) return ndk::ScopedAStatus::fromServiceSpecificError(24);
        out->fds.emplace_back(copy);
        out->ints = {37, 4};
        return ndk::ScopedAStatus::ok();
    }
    ndk::ScopedAStatus release() override {
        std::lock_guard<std::mutex> guard(lock);
        if (retained.get() < 0) return ndk::ScopedAStatus::fromServiceSpecificError(26);
        auto written = write(retained.get(), "R", 1);
        retained.set(-1);
        return written == 1 ? ndk::ScopedAStatus::ok() : ndk::ScopedAStatus::fromServiceSpecificError(25);
    }
    ndk::ScopedAStatus fdCount(int32_t* out) override { *out = fd_count(); return ndk::ScopedAStatus::ok(); }
};
extern "C" JNIEXPORT jobject JNICALL Java_org_pokelinux_camerapeer_Peer_createEndpoint(JNIEnv* env, jclass) {
    static auto peer = ndk::SharedRefBase::make<Peer>();
    return AIBinder_toJavaBinder(env, peer->asBinder().get());
}
