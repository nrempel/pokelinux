#!/usr/bin/env python3
"""Record compiled Android resource values relevant to stock IMS selection."""
import json,re,subprocess
from pathlib import Path
root=Path('artifacts/telephony-audit/input');out=Path('artifacts/telephony-audit/results/resources');out.mkdir(parents=True,exist_ok=True)
rows=[]
for p in sorted(root.rglob('*.apk')):
 if '/overlay/' not in str(p) and not re.search('ims|carrier|telephony|euicc',p.name,re.I):continue
 r=subprocess.run(['audit/sdk-tools/aapt2','dump','resources',str(p)],capture_output=True,text=True,errors="replace",timeout=60)
 if r.returncode:raise RuntimeError(str(p)+r.stderr)
 lines=r.stdout.splitlines(); hits=[]
 for i,line in enumerate(lines):
  if re.search('config_ims_|com.shannon.imsservice|imsserviceentitlement|ims_mmtel|ims_rcs|pixelims|ImsMediaController',line,re.I):
   hits.append({'line':i+1,'context':lines[max(0,i-2):i+7]})
 if hits:
  dest=out/(p.stem+'.txt');dest.write_text(r.stdout);rows.append({'path':'/'+str(p.relative_to(root)),'hits':hits})
(out/'index.json').write_text(json.dumps(rows,indent=2)+'\n');print(len(rows),'resource files with IMS evidence')
