#!/usr/bin/env python3
"""One real parent owns and actually waits one pipe-held child; no proc mocking."""
import json,select,subprocess,sys,time,os
from pathlib import Path
root=Path(sys.argv[1]);until=int(sys.argv[2]);now=lambda:time.clock_gettime_ns(time.CLOCK_BOOTTIME)
child=subprocess.Popen([sys.executable,'-c','import sys; sys.stdin.buffer.read(1)'],stdin=subprocess.PIPE,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
waited=False
def publish(name,value):
 with (root/name).open('x')as stream:json.dump(value,stream)
def finish_child():
 global waited
 if waited:return
 child.stdin.close()
 try:code=child.wait(timeout=3)
 except subprocess.TimeoutExpired:
  child.kill();code=child.wait(timeout=3)
 publish('child-wait.json',{'child_pid':child.pid,'original_parent_pid':os.getpid(),'returncode':code,'actual_original_parent_wait':True})
 waited=True
try:
 publish('ready.json',{'original_parent_pid':os.getpid(),'child_pid':child.pid})
 while now()<until:
  if (root/'exit-child').exists():finish_child()
  if (root/'stop').exists():break
  select.select([],[],[],.002)
finally:finish_child()
