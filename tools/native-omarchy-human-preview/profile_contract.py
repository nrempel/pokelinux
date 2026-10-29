import argparse,hashlib,json
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('profile');a=p.parse_args();path=Path(a.profile);data=path.read_bytes();profile=json.loads(data);apps=profile['apps'];assert len(apps)==1;app=apps[0]
expected=['/var/lib/pokelinux-sounds','/run/user/11042','/run/user/1000/wayland-1','/run/pokelinux/appearance.sock','/run/pokelinux/session/app.sock','/usr/share/pokelinux/appearance']
assert app['app']=='org.omarchy.Sounds'and app['uid']==11042 and app['args']==expected
record={'scope':'Actual installed profile configuration contract; not native app/Owner runtime execution','path':str(path),'profile_sha256':hashlib.sha256(data).hexdigest(),'same_UID11042_and_exact_six_args':True,'actual_executable':app['executable'],'required_executable':'/usr/libexec/pokelinux/pokelinux-sounds','passed':app['executable']=='/usr/libexec/pokelinux/pokelinux-sounds'}
print(json.dumps(record),flush=True);raise SystemExit(0 if record['passed']else 1)
