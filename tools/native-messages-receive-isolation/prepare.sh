#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 - <<'PY'
from pathlib import Path
import hashlib,json,shutil
source=Path('/home/ubuntu/Projects/pokelinux/artifacts/native-messages-incoming-binder/selections/incoming-green-final-v2/binaries')
base=Path('artifacts/native-messages-receive-isolation');target=base/'input';target.mkdir(parents=True,exist_ok=True)
pins={
'incoming-peer':'d3ace7cd833d10aa81217bae20d7a8765d06c3160167dbf69d848a9ce72560f4',
'property-info':'7c81ebd6b1b2bde547675aed2f6ee4cdc0e0b2a4892160c5af08b92b0d0f4f85',
'property-init':'27ba656934837f802ee05a7817ab83dd25fc1bbac57bd491a269dc5d338067a2',
'incoming-manager':'40395c1363de8057735ae0a883f6d9498a73e7a4f2a119bf1157332a5eec6dbd',
'incoming-probe':'270195bacdabd702a564bb2eb8a9f011bafd045e086afc7c1f5a16074dab5992'}
for name,pin in pins.items():
 f=source/name;assert hashlib.sha256(f.read_bytes()).hexdigest()==pin,'frozen public-generated peer/helper identity mismatch'
 t=target/name
 if t.exists():assert hashlib.sha256(t.read_bytes()).hexdigest()==pin
 else:shutil.copyfile(f,t)
(target/'input-pins.json').write_text(json.dumps(pins,indent=2)+'\n')
(base/'vm').mkdir(exist_ok=True)
# Empty nested mountpoint: the frozen public AIDL input is mounted readonly during builds.
Path('artifacts/sms-binder/frozen-aidl').mkdir(parents=True,exist_ok=True)
PY
