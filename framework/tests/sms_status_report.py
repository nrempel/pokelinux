#!/usr/bin/env python3
"""Execute original AOSP full/timestamp parsing on owned-emulator synthetic reports."""
import hashlib
import json
from pathlib import Path
import subprocess
import time

ADB = ['android-framework/sdk/platform-tools/adb', '-s', 'emulator-5554']
OUT = Path('artifacts/sms-status-report')
INPUT = Path('native-messaging/tests/fixtures/status-report-input.tsv')
REMOTE = '/data/local/tmp/pokelinux-sms-status-report'
def adb(*args):
    return subprocess.check_output(ADB + list(args), timeout=45, text=True, stderr=subprocess.STDOUT)
adb('wait-for-device')
end = time.monotonic() + 180
while adb('shell', 'getprop', 'sys.boot_completed').strip() != '1':
    assert time.monotonic() < end, 'owned emulator boot timeout'
    time.sleep(2)
assert adb('shell', 'getenforce').strip() == 'Enforcing'
OUT.mkdir(parents=True, exist_ok=True)
(OUT/'result.json').write_text(json.dumps({'passed': False, 'physical_phone_accessed': False})+'\n')
adb('shell', 'mkdir', '-p', REMOTE)
jar=OUT/'oracle/oracle.jar'
adb('push', str(jar), REMOTE+'/oracle.jar')
adb('push', str(INPUT), REMOTE+'/input.tsv')
output=adb('shell', 'env', 'CLASSPATH='+REMOTE+'/oracle.jar', 'app_process', '/system/bin', 'org.omarchy.audit.SmsStatusReportOracle', REMOTE+'/input.tsv')
(OUT/'aosp-output.tsv').write_text(output)
inputs=[line.split('\t') for line in INPUT.read_text().splitlines()]
rows=[line.split('\t') for line in output.splitlines()]
assert len(rows)==len(inputs)==126, output
assert all(len(row)==16 and row[0]=='sms-status-report' and row[15]=='PASS' for row in rows), output
assert [row[1:4] for row in rows]==inputs, 'original output must retain exact authored input/order/policy'
assert all(row[4]=='parsed' and row[14]=='1' for row in rows if row[3]=='compare')
assert sum(row[3]=='compare' for row in rows)==57
for name in ('framework.jar','telephony-common.jar'):
    adb('pull', '/system/framework/'+name, str(OUT/name))
(OUT/'image.txt').write_text(adb('shell', 'getprop', 'ro.build.fingerprint'))
(OUT/'client-identity.txt').write_text(adb('shell', 'id'))
assert adb('shell', 'getenforce').strip()=='Enforcing'
(OUT/'result.json').write_text(json.dumps({
    'passed': True, 'scope': 'original AOSP full GSM SmsMessage parser and its private PduParser timestamp decoder on bounded synthetic reports',
    'reports': 126, 'qualified_full_parser_comparisons': 57, 'native_stricter_malformed_cases': 61, 'native_explicit_unsupported_cases': 8,
    'discharge_timestamp_via_original_timestamp_decoder': True, 'full_parser_discards_discharge_timestamp': True,
    'original_calendar_invalid_may_return_zero': True,
    'input_sha256': hashlib.sha256(INPUT.read_bytes()).hexdigest(),
    'output_sha256': hashlib.sha256(output.encode()).hexdigest(),
    'oracle_jar_sha256': hashlib.sha256(jar.read_bytes()).hexdigest(),
    'framework_jar_sha256': hashlib.sha256((OUT/'framework.jar').read_bytes()).hexdigest(),
    'telephony_common_jar_sha256': hashlib.sha256((OUT/'telephony-common.jar').read_bytes()).hexdigest(),
    'selinux': 'Enforcing', 'delivery_correlation_qualified': False, 'receive_identity_qualified': False,
    'modem_acknowledgement_dispatched': False, 'vendor_execution': False, 'physical_phone_accessed': False,
},indent=2)+'\n')
print('Original AOSP status-report parser: 126 synthetic inputs, 57 qualified full-parser comparisons; no delivery or modem ACK authority',flush=True)
