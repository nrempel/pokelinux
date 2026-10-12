#!/usr/bin/env python3
"""Bounded QEMU debugger probe, only inside the isolated audit container."""
import os,subprocess
from pathlib import Path
if not Path('/.dockerenv').exists():raise SystemExit('Run inside the audit container')
env=dict(os.environ,PIXEL_RUNTIME='/workspace/artifacts/radio-runtime',HYBRIS_ENABLE_LINKER_DEBUG_MAP='1',HYBRIS_LD_DEBUG='1')
cmd=['sh','tools/with-runtime.sh','qemu-aarch64','-g','1234','-L',env['PIXEL_RUNTIME'],'artifacts/linux-aarch64/bin/runtime-probe',env['PIXEL_RUNTIME'],'--load','/vendor/lib64/libsitril.so']
p=subprocess.Popen(cmd,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
try:
 r=subprocess.run(['gdb-multiarch','-q','-batch','-ex','set pagination off','-ex','set sysroot /','-ex','set solib-search-path /workspace/artifacts/linux-aarch64/lib:/workspace/artifacts/linux-aarch64/lib/libhybris/linker:/workspace/artifacts/radio-runtime/vendor/lib64','-ex','set tcp auto-retry on','-ex','set tcp connect-timeout 10','-ex','target remote 127.0.0.1:1234','-ex','continue','-ex','thread apply all bt','-ex','info sharedlibrary','-ex',f'shell cat /proc/{p.pid}/maps','artifacts/linux-aarch64/bin/runtime-probe'],capture_output=True,text=True,timeout=40)
 print(r.stdout);print(r.stderr)
finally:
 p.kill();out=p.communicate(timeout=5)[0];print(out.decode(errors='replace'))
