#!/usr/bin/env python3
"""Separate same-UID actual kernel connect to a retired generation pathname."""
import json,os,socket,sys,time
r={'pid':os.getpid(),'uid':os.getuid(),'gid':os.getgid(),'path':sys.argv[1],'start_boottime_ns':time.clock_gettime_ns(time.CLOCK_BOOTTIME)}
with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as s:
 s.settimeout(.2)
 try:s.connect(sys.argv[1]);r['connected']=True
 except OSError as e:r.update(connected=False,errno=e.errno)
r['finish_boottime_ns']=time.clock_gettime_ns(time.CLOCK_BOOTTIME);print(json.dumps(r),flush=True)
