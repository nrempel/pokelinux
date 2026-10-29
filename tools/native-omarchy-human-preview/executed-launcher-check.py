from pathlib import Path
import hashlib,json,subprocess,sys
launcher=Path('/tmp/pokelinux-native-unlock-root/tools/native-omarchy-human-preview/launch.py')
base=Path('/tmp/pokelinux-native-x86-composition-human-preview-root-1')
name=sys.argv[1];out=Path('/tmp/pokelinux-native-x86-composition-'+name)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
source=json.loads((base/'selection.json').read_text());before={n:sha(base/n)for n in source['files']}
command=[sys.executable,'-B',str(launcher),'--base',str(base),'--name',name,'--prepare-only']
p=subprocess.run(command,capture_output=True,text=True,timeout=60)
print(p.stdout,end='');print(p.stderr,end='',file=sys.stderr)
assert p.returncode==0,('launch command unavailable or preparation failed',p.returncode)
s=json.loads((out/'selection.json').read_text());cmd=json.loads((out/'command.json').read_text())
assert not any((out/n).exists()for n in ['execution.json','result.json','owned-container-id','human-smoke-result.json'])
assert s['human_preview']['host_case_dir']==str(out)and s['human_preview']['dwell_ns']==300000000000
assert str(out)+':/results:rw'in cmd and str(base)+':/results:rw'not in cmd
assert all(sha(out/n)==h and (out/n).stat().st_nlink==1 for n,h in s['files'].items())
assert all(row['path']==str(out/'deployment/root'/n)for n,row in s['cold_installed_bundle']['files'].items())
assert s['files']['bins/pokelinux-audio']==source['files']['bins/pokelinux-audio']and s['files']['bins/pokelinux-session-owner']==source['files']['bins/pokelinux-session-owner']
frozen={str(p.relative_to(out)):sha(p)for p in out.rglob('*')if p.is_file()}
again=subprocess.run(command,capture_output=True,text=True,timeout=10);assert again.returncode!=0
assert frozen=={str(p.relative_to(out)):sha(p)for p in out.rglob('*')if p.is_file()}
assert before=={n:sha(base/n)for n in source['files']}
print(json.dumps({'passed':True,'scope':'actual prepared runtime selection and launcher CLI; no new VM','base_unchanged':True,'existing_output_refused_unchanged':True,'output':str(out)}))
