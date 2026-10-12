#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
umask 077
for peer in server client; do
 keytool -genkeypair -alias identity -keyalg RSA -keysize 2048 -validity 2 -dname "CN=$peer" -ext SAN=dns:localhost -storetype PKCS12 -keystore "$out/$peer.p12" -storepass disposable-test -keypass disposable-test > "$out/keytool-$peer.log" 2>&1
done
javac -source 8 -target 8 -Xlint:-options -d "$out/classes" framework/bridge/BridgeTransport.java framework/unit/BridgeTransportTlsTest.java
java -cp "$out/classes" org.omarchy.bridge.BridgeTransportTlsTest "$out/server.p12" "$out/client.p12"
