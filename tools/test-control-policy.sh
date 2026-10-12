#!/bin/sh
set -eu
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
javac -d "$out" framework/bridge/ControlPolicy.java framework/unit/ControlPolicyTest.java framework/bridge/CallRevision.java framework/unit/CallRevisionTest.java framework/bridge/Admission.java framework/unit/AdmissionTest.java framework/bridge/DialNumber.java framework/unit/DialNumberTest.java framework/bridge/DialAttempt.java framework/unit/DialAttemptTest.java framework/bridge/DialPolicy.java framework/unit/DialPolicyTest.java
javac -d "$out" framework/bridge/CallChanges.java framework/unit/CallChangesTest.java
javac -source 8 -target 8 -Xlint:-options -d "$out" framework/bridge/RingingRegistry.java framework/unit/RingingRegistryTest.java
java -cp "$out" org.omarchy.bridge.RingingRegistryTest
java -cp "$out" org.omarchy.bridge.CallChangesTest
java -cp "$out" org.omarchy.bridge.ControlPolicyTest

java -cp "$out" org.omarchy.bridge.CallRevisionTest

java -cp "$out" org.omarchy.bridge.AdmissionTest

java -cp "$out" org.omarchy.bridge.DialNumberTest
java -cp "$out" org.omarchy.bridge.DialAttemptTest
java -cp "$out" org.omarchy.bridge.DialPolicyTest

javac -source 8 -target 8 -Xlint:-options -d "$out" framework/bridge/CallChanges.java framework/bridge/RingingRegistry.java framework/bridge/CallWatch.java framework/unit/CallWatchTest.java
java -cp "$out" org.omarchy.bridge.CallWatchTest

javac -source 8 -target 8 -Xlint:-options -d "$out" framework/bridge/BridgeTransport.java framework/unit/BridgeTransportTest.java
java -cp "$out" org.omarchy.bridge.BridgeTransportTest

javac -source 8 -target 8 -Xlint:-options -d "$out" framework/bridge/WatchWorkers.java framework/unit/WatchWorkersTest.java
java -cp "$out" org.omarchy.bridge.WatchWorkersTest

javac -source 8 -target 8 -Xlint:-options -d "$out" framework/bridge/Admission.java framework/bridge/CredentialExpiry.java framework/unit/CredentialExpiryTest.java
java -cp "$out" org.omarchy.bridge.CredentialExpiryTest
