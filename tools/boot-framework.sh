#!/bin/bash
set -euo pipefail
cd /workspace
export ANDROID_AVD_HOME=/workspace/android-framework/home/avd
mkdir -p "$ANDROID_AVD_HOME/pixel-test.avd" android-framework/results
cat > "$ANDROID_AVD_HOME/pixel-test.ini" <<INI
avd.ini.encoding=UTF-8
path=$ANDROID_AVD_HOME/pixel-test.avd
target=android-37
INI
cat > "$ANDROID_AVD_HOME/pixel-test.avd/config.ini" <<INI
AvdId=pixel-test
abi.type=x86_64
hw.cpu.arch=x86_64
hw.cpu.ncore=2
hw.ramSize=3072
hw.lcd.width=720
hw.lcd.height=1280
hw.lcd.density=320
hw.gpu.enabled=yes
hw.gpu.mode=swiftshader
hw.keyboard=yes
hw.audioInput=no
hw.audioOutput=no
image.sysdir.1=/workspace/android-framework/sdk/system-image/x86_64/
disk.dataPartition.size=3G
showDeviceFrame=no
tag.id=google_apis
PlayStore.enabled=false
INI
exec android-framework/sdk/emulator/emulator -avd pixel-test -no-window -no-audio -no-boot-anim -no-snapshot -gpu swiftshader -accel on -port 5554 -no-metrics -feature -Vulkan -show-kernel -wipe-data -verbose -debug modem,radio
