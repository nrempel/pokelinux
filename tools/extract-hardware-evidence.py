#!/usr/bin/env python3
"""Extract selected public factory files without mounting or executing images."""
import argparse, hashlib, json, re, stat
from pathlib import Path
from dissect.extfs import ExtFS
ap=argparse.ArgumentParser();ap.add_argument('image',type=Path);ap.add_argument('partition');ap.add_argument('output',type=Path);args=ap.parse_args()
pattern=re.compile(r'ims|telephony|carrier|euicc|ril|modem|cbd|rfsd|audio|camera|power|thermal',re.I)
rows=[]
def walk(node,prefix):
 for n in node.iterdir():
  if n.filename in ('.','..'):continue
  path=prefix+'/'+n.filename
  if n.filetype==stat.S_IFDIR:walk(n,path)
  elif n.filetype==stat.S_IFREG:
   row={'path':path,'size':n.size};rows.append(row)
   if (path.endswith(('.apk','.jar','.rc','.xml','.prop')) and pattern.search(path)) or (path.endswith('.apk') and '/overlay/' in path):
    data=n.open().read();out=args.output/'input'/path.lstrip('/');out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(data)
    row['extracted']=True;row['sha256']=hashlib.sha256(data).hexdigest()
with args.image.open('rb') as f:walk(ExtFS(f).root,'/'+args.partition)
(args.output/(args.partition+'-inventory.json')).write_text(json.dumps(rows,indent=2)+'\n')
print(json.dumps({'partition':args.partition,'files':len(rows),'extracted':sum(bool(r.get('extracted')) for r in rows)}))
