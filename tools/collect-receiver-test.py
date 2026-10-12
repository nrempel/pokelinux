#!/usr/bin/env python3
"""Copy the exact static test executable named by this Cargo invocation."""
import json
from pathlib import Path
import shutil
root=Path('artifacts/native-radio')
executables=[]
for line in (root/'receiver-test-build.json').read_text().splitlines():
 row=json.loads(line)
 if row.get('reason')=='compiler-artifact' and row.get('target',{}).get('name')=='response_receiver' and row.get('profile',{}).get('test') and row.get('executable'):
  executables.append(row['executable'])
assert len(executables)==1,executables
relative=Path(executables[0]).relative_to('/target')
assert '..' not in relative.parts
shutil.copyfile(root/relative,root/'response-receiver-tests')
