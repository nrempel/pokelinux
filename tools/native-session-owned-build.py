#!/usr/bin/env python3
"""Preserve the owned-focus Rust source and prepare actual ARM kernel tests."""
import argparse,fcntl,hashlib,json,os,shutil,subprocess
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('label');args=parser.parse_args()
assert args.label and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in args.label)
repo=Path(__file__).resolve().parents[1];root=repo/'artifacts/native-session-observer';root.mkdir(exist_ok=True)
lock=(root/'build.lock').open('w');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
out=root/args.label;assert not out.exists(),'Preserve existing build';out.mkdir()
source=out/'source';source.mkdir()
for name in ('native-session','native-lifecycle','native-kernel-identity','native-camera'):
 shutil.copytree(repo/name,source/name,ignore=shutil.ignore_patterns('target','._*'))
sources={str(p.relative_to(source)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(source.rglob('*')) if p.is_file()}
docker=['env','-u','DOCKER_HOST','-u','DOCKER_CONTEXT','-u','DOCKER_TLS','-u','DOCKER_TLS_VERIFY','-u','DOCKER_CERT_PATH','docker','--host=unix:///var/run/docker.sock']
image='pixel-phone-rust:20261012-b472';image_id=subprocess.check_output(docker+['image','inspect',image,'--format','{{.Id}}']).decode().strip()
selection={'sources':sources,'container_image':image,'container_image_id':image_id,'target':'aarch64-unknown-linux-gnu','rustflags':'-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc','arm_kernel_execution_qualified':False,'observer_process_cases':38}
(out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
script="set -euo pipefail\ncargo fmt --manifest-path /source/native-session/Cargo.toml -- --check\ncargo fmt --manifest-path /source/native-lifecycle/Cargo.toml -- --check\ncargo clippy --offline --locked --manifest-path /source/native-session/Cargo.toml --all-targets --features isolated-fixture -- -D warnings\ncargo test --offline --locked --manifest-path /source/native-lifecycle/Cargo.toml --lib --no-run --message-format=json > /results/lifecycle-build.jsonl\ncargo build --offline --locked --manifest-path /source/native-lifecycle/Cargo.toml --bin lifecycle-daemon\nRUSTFLAGS='-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc' cargo test --offline --locked --manifest-path /source/native-session/Cargo.toml --features isolated-fixture --test process --target aarch64-unknown-linux-gnu --no-run --message-format=json > /results/arm-build.jsonl\nRUSTFLAGS='-C target-feature=+crt-static -C linker=aarch64-linux-gnu-gcc' cargo build --offline --locked --manifest-path /source/native-session/Cargo.toml --features isolated-fixture --bin session-observer-fixture --target aarch64-unknown-linux-gnu\n"
command=docker+['run','--rm','--pull=never','--name','pokelinux-session-owned-focus-build-20261013','--network','none','--cpus','12','--memory','12g','--pids-limit','128','--cap-drop','ALL','--security-opt','no-new-privileges','--user',f'{os.getuid()}:{os.getgid()}','-v',f'{source}:/source:ro','-v',f'{out}:/results:rw','-v',f'{repo}/sim/rust-cargo:/cargo:rw','-e','CARGO_HOME=/cargo','-e','CARGO_TARGET_DIR=/results/target','--entrypoint','bash',image,'-c',script]
with (out/'build.log').open('wb') as log:result=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
(out/'exit-code').write_text(str(result.returncode)+'\n')
if result.returncode:print((out/'build.log').read_text()[-4000:]);raise SystemExit(result.returncode)
entries=[json.loads(line) for line in (out/'arm-build.jsonl').read_text().splitlines() if line.startswith('{')]
executables=[e['executable'] for e in entries if e.get('reason')=='compiler-artifact' and e.get('executable') and e['target']['name']=='process']
assert len(executables)==1
bins=out/'bins';bins.mkdir()
for name,path in [('process-tests',executables[0]),('session-peer-fixture','/results/target/aarch64-unknown-linux-gnu/debug/session-peer-fixture'),('session-observer-fixture','/results/target/aarch64-unknown-linux-gnu/debug/session-observer-fixture')]:
 selected=out/Path(path).relative_to('/results');shutil.copyfile(selected,bins/name);(bins/name).chmod(0o755)
 elf=subprocess.check_output(docker+['run','--rm','--pull=never','--network','none','--cpus','1','--memory','128m','--pids-limit','32','--cap-drop','ALL','--security-opt','no-new-privileges','--read-only','--user',f'{os.getuid()}:{os.getgid()}','-v',f'{bins/name}:/probe:ro','--entrypoint','aarch64-linux-gnu-readelf',image_id,'-h','-l','-d','/probe']).decode();assert 'AArch64' in elf and 'INTERP' not in elf and '(NEEDED)' not in elf
 (bins/(name+'-elf.txt')).write_text(elf)
lifecycle=[json.loads(line) for line in (out/'lifecycle-build.jsonl').read_text().splitlines() if line.startswith('{')]
selected=[e['executable'] for e in lifecycle if e.get('reason')=='compiler-artifact' and e.get('executable') and e.get('profile',{}).get('test') and e['target']['kind']==['lib']]
assert len(selected)==1
unit=out/Path(selected[0]).relative_to('/results');helper=out/'target/debug/lifecycle-daemon'
selection['lifecycle_component_executables']={str(p.relative_to(out)):hashlib.sha256(p.read_bytes()).hexdigest() for p in (unit,helper)}
(out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
runtime='pixel-graphics-build:20261011-b472'
selection['component_runtime_image_id']=subprocess.check_output(docker+['image','inspect',runtime,'--format','{{.Id}}']).decode().strip()
with (out/'lifecycle-component.log').open('wb') as log:
 result=subprocess.run(docker+['run','--rm','--pull=never','--name','pokelinux-session-owned-focus-components-20261013','--network','none','--cpus','2','--memory','256m','--pids-limit','32','--cap-drop','ALL','--security-opt','no-new-privileges','--read-only','--tmpfs','/run:rw,nosuid,nodev','--tmpfs','/opt:rw,exec,nosuid,nodev','-e','NATIVE_LIFECYCLE_TEST_ROOT=/run','-e','NATIVE_LIFECYCLE_STORAGE_HELPER=/worker-source','-v',f'{unit}:/tests:ro','-v',f'{helper}:/worker-source:ro',runtime,'/tests','--test-threads=1'],stdout=log,stderr=subprocess.STDOUT)
selection['lifecycle_component_exit_code']=result.returncode
selection['lifecycle_component_log_sha256']=hashlib.sha256((out/'lifecycle-component.log').read_bytes()).hexdigest()
assert result.returncode==0,(out/'lifecycle-component.log').read_text()
selection['executables']={str(p.relative_to(out)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(bins.iterdir()) if p.is_file()}
(out/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
print(json.dumps({'build':str(out.relative_to(repo)),'arm_kernel_execution_qualified':False,'executables':selection['executables']},indent=2))
