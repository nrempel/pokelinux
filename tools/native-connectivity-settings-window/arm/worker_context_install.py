"""Install one qualified diagnostic worker only after full old-image verification."""
from pathlib import Path
import hashlib,json,os,stat

def pin(p):
 with Path(p).open("rb") as f:return {"bytes":Path(p).stat().st_size,"sha256":hashlib.file_digest(f,"sha256").hexdigest()}

def install(results,selection,run):
 R=Path(results);Q=selection["worker_error_context"]
 assert os.geteuid()==os.getegid()==0 and Q is not None and Q["Root_closed_GREEN"] is True
 old=dict(selection["providers"]["worker"]);assert old==Q["old_worker"]
 dst=Path(old["guest_path"])
 assert str(dst)==Q["guest_path"]=="/usr/libexec/pokelinux-fixture/pokelinux-connectivity-service"
 def exact(path,expected,mode,root):
  info=path.lstat();assert stat.S_ISREG(info.st_mode) and stat.S_IMODE(info.st_mode)==mode
  if root:assert info.st_uid==info.st_gid==0
  assert pin(path)=={k:expected[k]for k in ["bytes","sha256"]}
 for parent in dst.parents:
  info=parent.lstat();assert stat.S_ISDIR(info.st_mode) and info.st_uid==info.st_gid==0 and not info.st_mode&0o022
 exact(dst,old,0o755,True)
 assert Q["source"]["relative_path"]=="source/worker-error-context.elf"
 src=R/Q["source"]["relative_path"];exact(src,Q["source"],0o444,False)
 with src.open("rb") as f:header=f.read(20)
 assert header[:6]==b"\x7fELF\x02\x01" and int.from_bytes(header[18:20],"little")==183
 for role in ["Root_build","Root_CLI"]:
  row=Q[role];proof=R/row["relative_path"]
  assert pin(proof)=={k:row[k]for k in ["bytes","sha256"]}
  assert json.loads(proof.read_text())["passed"] is True
 run(["/usr/bin/install","-o","0","-g","0","-m","755",str(src),str(dst)],"install-qualified-worker-error-context")
 exact(dst,Q["source"],0o755,True)
 with dst.open("rb") as f:header=f.read(20)
 assert header[:6]==b"\x7fELF\x02\x01" and int.from_bytes(header[18:20],"little")==183
 loader=run(["/usr/bin/ldd","-r",str(dst)],"qualified-worker-error-context-loader")
 assert loader.strip() and "not found" not in loader.lower() and "undefined symbol" not in loader.lower()
 selection["providers"]["worker"]={**pin(dst),"guest_path":str(dst),"mode":0o755,"ELF_machine":183}
 return {"after_full_old_image_verification":True,"same_guest_path":True,"old_worker":old,"new_worker":selection["providers"]["worker"],"qualified_source":Q["source"],"Root_build":Q["Root_build"],"Root_CLI":Q["Root_CLI"],"loader_sha256":hashlib.sha256(loader.encode()).hexdigest()}
