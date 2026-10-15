#!/usr/bin/env python3
"""Freeze a fresh public-only source/bin/input selection. Never mutates a label."""
import hashlib,json,shutil,subprocess,sys
from pathlib import Path
ROOT=Path('/workspace');BASE=ROOT/'artifacts/native-radio-load-only';label=sys.argv[1] if len(sys.argv)>1 else 'freeze';assert label.replace('-','').isalnum();OUT=BASE/label
OUT.mkdir()
files={}
def digest(p):
    h=hashlib.sha256()
    with p.open('rb') as f:
        for chunk in iter(lambda:f.read(1048576),b''):h.update(chunk)
    return h.hexdigest()
def copy(p,name):
    dst=OUT/name;dst.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,dst);files[name]=digest(dst)
for source in sorted((ROOT/'tools/native-radio-load-only').rglob('*')):
    if source.is_file() and source.suffix in ['.rs','.py','.toml','.lock','.md','.sh']:copy(source,'source/'+str(source.relative_to(ROOT)))
for crate in ['native-process-owner','native-radio','native-ims','native-kernel-identity','native-phone-state']:
    for source in sorted((ROOT/crate).rglob('*')):
        if source.is_file() and source.suffix in ['.rs','.aidl','.toml','.lock']:copy(source,'source/'+str(source.relative_to(ROOT)))
for name in ['radio-containment-red','radio-harmless-child','radio-load-owner']:
    copy(Path('/target/aarch64-unknown-linux-gnu/debug')/name,'bin/'+name);(OUT/'bin'/name).chmod(0o755)
inputs={name:{'sha256':digest(Path('/inputs')/name)} for name in ['Image','initramfs-linux.img','root.img']}
assert inputs['Image']['sha256']=='2b069424bf66ffd5fe9e4f134ecb61c0381cd4356db3110940f0500e0ae2f45b'
assert inputs['initramfs-linux.img']['sha256']=='a9bb29d29abda94748c9948f903c3c1704f829eea477652b9a4201a61f36a647'
assert inputs['root.img']['sha256']=='973a19f9661e915b2d9eb88753ef4d59585cc6ebd4565e43a03b43e74b0f7579'
selection={'realm':'public harmless children; experimental containment owner; service-free Arch ARM64 kernel','source_commit_claimed':False,'binary_to_source_build_binding_qualified':False,'files':files,'inputs':inputs,'builder_image':'sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6','runtime_image':'sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6','original_runtime_admitted':False,'registration_admitted':False,'prior_genuine_red_selection':'c21a20abfe2b16f27db834d928fd34dc20dbf20843c1ab4543fbf5a716e9f358','prior_genuine_red_result':'1625852f72cbde0a09a4404cce12a351288f29e09444af664c99acbc66433fbf','controls':['held-responsive-stop-real-waits','outside-root-ip-unix-readonly-device-namespace-fd-memory','second-exec-before-dispatch','task-cgroup-real-thread-waits','withheld-queued-ready-after-original-deadline','parent-death-after-final-credentials']}
(OUT/'selection.json').write_text(json.dumps(selection,indent=2)+'\n')
print(json.dumps({'selection_sha256':digest(OUT/'selection.json'),'files':len(files),'inputs':inputs},indent=2))
