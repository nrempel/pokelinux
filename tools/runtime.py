#!/usr/bin/env python3
"""Assemble an offline Pixel graphics payload. Never mounts or contacts devices."""
import argparse, hashlib, io, json, posixpath, stat, zipfile
from pathlib import Path, PurePosixPath

BIONIC='/apex/com.android.runtime/lib64/bionic'
def safe_path(path):
 if not path.startswith('/') or '..' in PurePosixPath(path).parts:
  raise ValueError(f'Unsafe image path: {path}')
 return path.lstrip('/')
def resolve_closure(nodes, roots):
 files=set();edges={};todo=list(roots)
 while todo:
  path=todo.pop();safe_path(path)
  if path in files:continue
  if path not in nodes:raise ValueError(f'Missing root: {path}')
  files.add(path);edges[path]={}
  for name in nodes[path]:
   if '/' in name:raise ValueError(f'Unexpected dependency path: {name}')
   # Keep framework and vendor copies distinct. This is a staging policy,
   # not a substitute for Android's generated linker namespaces.
   dirs=[BIONIC] if name in ('libc.so','libdl.so','libm.so') else []
   dirs+=['/vendor/lib64','/system/lib64'] if path.startswith('/vendor/') else ['/system/lib64','/vendor/lib64']
   dirs+=[BIONIC,'/apex/com.android.runtime/lib64']
   target=next((d+'/'+name for d in dirs if d+'/'+name in nodes),None)
   if target is None:raise ValueError(f'{path}: unresolved {name}')
   edges[path][name]=target;todo.append(target)
 return files,edges

class ImageFiles:
 def __init__(self, directory):
  from dissect.extfs import ExtFS
  self.entries={};self.handles=[]
  for part in ('vendor','system','system_ext'):
   f=(directory/(part+'.img')).open('rb');self.handles.append(f)
   fs=ExtFS(f)
   def walk(node,prefix):
    for n in node.iterdir():
     if n.filename in ('.','..'):continue
     path=prefix+'/'+n.filename
     if n.filetype==stat.S_IFDIR:walk(n,path)
     elif n.filetype in (stat.S_IFREG,stat.S_IFLNK):self.entries[path]=n
   walk(fs.root,'' if part=='system' else '/'+part)
  apex=zipfile.ZipFile(io.BytesIO(self.read('/system/apex/com.android.runtime.apex')))
  payload=io.BytesIO(apex.read('apex_payload.img'));self.handles.append(payload)
  fs=ExtFS(payload)
  def walk_apex(node,prefix):
   for n in node.iterdir():
    if n.filename in ('.','..'):continue
    path=prefix+'/'+n.filename
    if n.filetype==stat.S_IFDIR:walk_apex(n,path)
    elif n.filetype in (stat.S_IFREG,stat.S_IFLNK):self.entries[path]=n
  walk_apex(fs.root,'/apex/com.android.runtime')
 def actual(self,path):
  seen=set()
  while path in self.entries and self.entries[path].filetype==stat.S_IFLNK:
   if path in seen:raise ValueError('Image symlink cycle')
   seen.add(path);target=self.entries[path].link
   path=posixpath.normpath(target if target.startswith('/') else posixpath.join(posixpath.dirname(path),target))
  safe_path(path)
  return path
 def read(self,path):return self.entries[self.actual(path)].open().read()

class Dependencies:
 def __init__(self,images):self.images=images;self.cache={}
 def __contains__(self,path):return self.images.actual(path) in self.images.entries
 def __getitem__(self,path):
  from elftools.elf.elffile import ELFFile
  if path not in self.cache:
   e=ELFFile(io.BytesIO(self.images.read(path)))
   if e['e_machine']!='EM_AARCH64':raise ValueError(f'Wrong ELF architecture: {path}')
   dyn=e.get_section_by_name('.dynamic')
   self.cache[path]=[t.needed for t in dyn.iter_tags() if t.entry.d_tag=='DT_NEEDED'] if dyn else []
  return self.cache[path]

ROOTS=['/vendor/lib64/egl/libGLESv1_CM_powervr.so','/vendor/lib64/egl/libEGL_powervr.so','/vendor/lib64/egl/libGLESv2_powervr.so',
 '/vendor/lib64/hw/mapper.pixel.so','/vendor/bin/hw/pixel.gralloc.allocator-service',
 '/system/bin/servicemanager','/apex/com.android.runtime/bin/linker64']

def assemble(images_dir,dest,evidence_file):
 if dest.exists() and any(dest.iterdir()):raise ValueError('Destination must be empty')
 evidence=json.loads(evidence_file.read_text())
 expected={line.split()[1]:line.split()[0] for line in evidence['partition_sha256'].splitlines()}
 for part in ('vendor','system','system_ext'):
  with (images_dir/(part+'.img')).open('rb') as f:actual=hashlib.file_digest(f,'sha256').hexdigest()
  if actual!=expected[part+'.img']:raise ValueError(f'Image checksum mismatch: {part}')
 images=ImageFiles(images_dir);nodes=Dependencies(images)
 files,edges=resolve_closure(nodes,ROOTS)
 extras=[p for p in images.entries if p.startswith('/vendor/firmware/rgx.')]
 extras+=['/vendor/build.prop','/system/build.prop','/vendor/etc/powervr.ini',
 '/vendor/etc/init/pixel.gralloc.allocator-service.rc','/vendor/etc/vintf/manifest/manifest_allocator_aidl.xml',
 '/vendor/etc/vintf/manifest/manifest_mapper_framework.xml','/vendor/etc/linker.config.pb','/system/etc/linker.config.pb']
 files.update(extras);manifest=[]
 for path in sorted(files):
  data=images.read(path);out=dest/safe_path(path);out.parent.mkdir(parents=True,exist_ok=True)
  out.write_bytes(data);out.chmod(0o755 if '/bin/' in path else 0o644)
  manifest.append({'path':path,'source':images.actual(path),'size':len(data),'sha256':hashlib.sha256(data).hexdigest()})
 # Android executables use this interpreter; symlink remains inside the payload.
 link=dest/'system/bin/linker64';link.parent.mkdir(parents=True,exist_ok=True)
 if not link.exists():link.symlink_to('../../apex/com.android.runtime/bin/linker64')
 report={'schema':1,'build':evidence['build'],'status':'assembled; service startup and graphics unverified',
 'roots':ROOTS,'files':manifest,'needed_edges':edges,
 'limits':['Dependency search is explicit staging policy, not generated linker namespaces.',
 'No Android init/property service or Binder device is supplied by this payload.',
 'Stock Bionic may need libhybris compatibility patches; no binary replacement is assumed.',
 'No service or device is started by this assembler.']}
 (dest/'runtime-manifest.json').write_text(json.dumps(report,indent=2)+'\n')
 return report
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--images',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--evidence',type=Path,required=True);a=p.parse_args()
 r=assemble(a.images,a.output,a.evidence);print(f"Staged {len(r['files'])} files for {r['build']}")
