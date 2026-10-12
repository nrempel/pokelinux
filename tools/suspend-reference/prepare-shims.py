#!/usr/bin/env python3
"""Create our empty include adapters, not copies of upstream headers."""
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent.parent
DESTINATION = ROOT / "artifacts/native-suspend/reference/shims"
HEADERS = [
    "android-base/result.h", "android-base/thread_annotations.h", "android-base/unique_fd.h",
    "android-base/file.h", "android-base/logging.h", "android-base/parseint.h",
    "android-base/properties.h", "android-base/stringprintf.h", "android-base/strings.h",
    "android/system/suspend/internal/SuspendInfo.h",
    "android/system/suspend/internal/ISuspendControlServiceInternal.h",
    "android/system/suspend/internal/WakeLockInfo.h", "android/system/suspend/internal/WakeupInfo.h",
    "utils/RefBase.h", "utils/Mutex.h", "binder/Trace.h", "binder/IPCThreadState.h",
    "android/binder_manager.h", "suspend_service_flags.h",
    "aidl/android/system/suspend/ISystemSuspend.h", "aidl/android/system/suspend/IWakeLock.h",
    "aidl/android/system/suspend/BnWakeLock.h", "aidl/android/system/suspend/BnSystemSuspend.h",
]
for name in HEADERS:
    path = DESTINATION / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text('#include "/harness/platform.h"\n')
