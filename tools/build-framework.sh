#!/bin/bash
set -euo pipefail
cd /workspace
sdk=android-framework/sdk
bt=$sdk/build-tools/android-37.0
jar=$sdk/platform/android-37.0/android.jar
out=android-framework/apks
mkdir -p "$out" android-framework/home
# Disposable emulator test identities; never used for deployment or stock APKs.
for identity in trusted untrusted; do
 if [ ! -f "$out/$identity.jks" ]; then
 keytool -genkeypair -keystore "$out/$identity.jks" -storepass android -keypass android -alias test -dname "CN=Disposable emulator $identity" -keyalg RSA -validity 7 -noprompt >/dev/null 2>&1
 fi
done
build_apk() {
 local name=$1 src=$2 manifest=$3 identity=$4
 local tmp
 tmp=$(mktemp -d)
 mkdir -p "$tmp/classes" "$tmp/dex"
 if [ "$name" = bridge-legacy-fixture ]; then
  mkdir -p "$tmp/source"
  cp "$src"/*.java "$tmp/source/"
  cp framework/legacy-fixture/GatewayIdentity.java "$tmp/source/GatewayIdentity.java"
  src="$tmp/source"
 fi
 javac -source 8 -target 8 -Xlint:-options -bootclasspath "$jar" -d "$tmp/classes" "$src"/*.java
 "$bt/d8" --lib "$jar" --min-api 31 --output "$tmp/dex" "$tmp"/classes/org/omarchy/*/*.class
 "$bt/aapt2" link -I "$jar" --manifest "$manifest" -o "$tmp/unsigned.apk"
 if [ "$name" = provisioner ]; then
  mkdir -p "$tmp/assets"
  for file in identity.p12 trust.p12 tls-password client-sha256; do cp "android-framework/transport-fixture/$file" "$tmp/assets/"; done
  (cd "$tmp" && zip -q unsigned.apk assets/*)
 fi
 (cd "$tmp/dex" && zip -q "$tmp/unsigned.apk" classes.dex)
 "$bt/zipalign" -f 4 "$tmp/unsigned.apk" "$tmp/aligned.apk"
 if [ "$name" = bridge ]; then cp "$tmp/aligned.apk" "$out/bridge-unsigned.apk"; fi
 "$bt/apksigner" sign --ks "$out/$identity.jks" --ks-pass pass:android --out "$out/$name.apk" "$tmp/aligned.apk"
 "$bt/apksigner" verify "$out/$name.apk"
 rm -rf "$tmp"
}
for who in trusted untrusted norequest; do
 cat > "$out/$who.xml" <<XML
<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="org.omarchy.$who">
<uses-sdk android:minSdkVersion="31" android:targetSdkVersion="37" />
<uses-permission android:name="org.omarchy.bridge.ACCESS" />
<queries><package android:name="org.omarchy.bridge" /><package android:name="org.omarchy.untrusted" /><package android:name="org.omarchy.imsfixture" /></queries>
<application android:label="Framework test client" android:debuggable="true"><service android:name="org.omarchy.probe.RelayService" android:exported="true" /></application>
<instrumentation android:name="org.omarchy.probe.Probe" android:targetPackage="org.omarchy.$who" />
</manifest>
XML
 identity=$who
 if [ "$who" = norequest ]; then
  identity=trusted
  sed -i '/<uses-permission android:name="org.omarchy.bridge.ACCESS"/d' "$out/$who.xml"
 fi
 build_apk "$who" framework/client "$out/$who.xml" "$identity"
done
if [ -f framework/bridge/AndroidManifest.xml ]; then
 build_apk bridge framework/bridge framework/bridge/AndroidManifest.xml trusted
 python3 tools/verify-release-apk.py
 build_apk bridge-legacy-fixture framework/bridge framework/bridge/AndroidManifest.xml trusted
fi
if [ -f framework/ims/AndroidManifest.xml ]; then
 jar=$sdk/system-android17.jar
 build_apk ims framework/ims framework/ims/AndroidManifest.xml trusted
 sed -e 's/package="org.omarchy.imsfixture"/package="org.omarchy.imsbad"/' -e 's/android:name=".TestImsService"/android:name="org.omarchy.imsfixture.TestImsService"/' -e 's/BIND_IMS_SERVICE/BIND_INCALL_SERVICE/' framework/ims/AndroidManifest.xml > "$out/imsbad.xml"
 build_apk imsbad framework/ims "$out/imsbad.xml" trusted
fi

jar=$sdk/platform/android-37.0/android.jar
python3 tools/prepare-transport-fixture.py
build_apk provisioner framework/provisioner framework/provisioner/AndroidManifest.xml trusted
build_apk enrollmentprobe framework/enrollmentprobe framework/enrollmentprobe/AndroidManifest.xml trusted
