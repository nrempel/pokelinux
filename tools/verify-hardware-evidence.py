#!/usr/bin/env python3
"""Verify real factory audit evidence and demonstrate negative controls."""
import copy,hashlib,json
from pathlib import Path
from hardware_audit import check_telephony_evidence
root=Path('artifacts/telephony-audit');e=json.loads((root/'results/evidence.json').read_text());r=json.loads((root/'results/resources/index.json').read_text())
errors=check_telephony_evidence(e,r)
if errors:raise SystemExit('Evidence checks failed: '+str(errors))
for row in e['files']:
 p=root/'input'/row['path'].lstrip('/')
 if hashlib.sha256(p.read_bytes()).hexdigest()!=row['sha256']:raise SystemExit('Input changed: '+row['path'])
removed=copy.deepcopy(e);removed['files']=[x for x in removed['files'] if not x['path'].endswith('/ShannonIms.apk')]
if 'missing_shannon_ims' not in check_telephony_evidence(removed,r):raise SystemExit('Missing APK negative control failed')
if 'missing_device_ims_selection' not in check_telephony_evidence(e,[]):raise SystemExit('Missing overlay negative control failed')
claim=copy.deepcopy(e);claim['hardware_verified']=True
if 'hardware_claim_not_allowed' not in check_telephony_evidence(claim,r):raise SystemExit('Unsupported hardware claim accepted')
result={'static_evidence_checks':'pass','hashed_files':len(e['files']),'apk_manifests':sum('manifest' in x for x in e['files']),'negative_controls':['remove IMS APK','remove IMS selection overlay','claim hardware verified'],'hardware_verified':False,'android_framework_booted':False,'carrier_verified':False}
(root/'results/verification.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
