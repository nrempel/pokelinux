#!/usr/bin/env python3
"""Read-only kernel Binder interoperability check in the owned emulator."""
from pathlib import Path
import subprocess
import time

binary=Path('artifacts/native-radio/x86_64-unknown-linux-musl/release/pokelinux-radio')
assert binary.is_file(), 'native Linux Binder client has not been built'
adb=['android-framework/sdk/platform-tools/adb','-s','emulator-5554']
def run(*args):
 return subprocess.check_output(adb+list(args),text=True,stderr=subprocess.STDOUT,timeout=45)
run('wait-for-device')
end=time.monotonic()+180
while run('shell','getprop','sys.boot_completed').strip()!='1':
 assert time.monotonic()<end,'emulator boot timeout'
 time.sleep(2)
assert run('shell','getenforce').strip()=='Enforcing'
target='/data/local/tmp/pokelinux-binder-probe'
run('push',str(binary),target)
run('shell','chmod','755',target)
try:
 output=run('shell',target)
except subprocess.CalledProcessError as error:
 print(error.output,flush=True)
 raise
assert output.splitlines()==['kernel-binder\tandroid.os.IServiceManager\tandroid.app.IActivityManager\tPASS'],output
Path('artifacts/native-radio/kernel-binder-output.txt').write_text(output)
print('Static Linux Rust client: real Binder driver, service lookup and remote ping passed',flush=True)
