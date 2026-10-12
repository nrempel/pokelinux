"""Executed by actual Arch glibc/Python on the Android emulator kernel."""
import json,os,platform,ssl,sys
from pathlib import Path
from omarchy_bridge import Client
config=json.loads(Path(sys.argv[1]).read_text());request=json.loads(sys.argv[2]);client=Client(config)
if request['op']=='status':value=client.status()
elif request['op']=='dial_status':value=client.dial_status()
elif request['op']=='dial':value=client.dial(**request['command'])
elif request['op']=='controls':value=client.controls()
elif request['op']=='control':value=client.control(**request['command'])
else:raise ValueError('operation')
print(json.dumps({'value':value,'architecture':platform.machine(),'python':sys.version,'openssl':ssl.OPENSSL_VERSION,'libc':platform.libc_ver(),'uid':os.getuid(),'net_namespace':os.readlink('/proc/self/ns/net')}))
