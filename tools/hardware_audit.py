#!/usr/bin/env python3
"""Static factory dependency evidence. No hardware access or Android execution."""
import argparse, hashlib, io, json, re, struct, subprocess, zipfile
from pathlib import Path


def dex_references(data):
    if len(data)<112 or data[:4]!=b'dex\n':
        raise ValueError('Not a standard DEX file')
    size=struct.unpack_from('<I',data,32)[0]
    if size!=len(data):
        raise ValueError('Truncated or unsupported DEX container')
    def table(offset,width):
        count,start=struct.unpack_from('<II',data,offset)
        if start+count*width>len(data):raise ValueError('DEX table outside file')
        return count,start
    count,start=table(56,4); strings=[]
    for i in range(count):
        pos=struct.unpack_from('<I',data,start+4*i)[0]
        # Skip the UTF-16 length ULEB128. Identifiers below are ASCII.
        for _ in range(5):
            if pos>=len(data):raise ValueError('Truncated DEX string length')
            b=data[pos];pos+=1
            if not b&128:break
        else:raise ValueError('Invalid DEX string length')
        end=data.find(b'\0',pos)
        if end<0:raise ValueError('Unterminated DEX string')
        strings.append(data[pos:end].decode('utf-8',errors='replace'))
    count,start=table(64,4); types=[]
    for i in range(count):
        idx=struct.unpack_from('<I',data,start+4*i)[0]
        if idx>=len(strings):raise ValueError('Invalid DEX type index')
        types.append(strings[idx])
    count,start=table(88,8); methods=[]
    for i in range(count):
        cls,proto,name=struct.unpack_from('<HHI',data,start+8*i)
        if cls>=len(types) or name>=len(strings):raise ValueError('Invalid DEX method index')
        methods.append(types[cls]+'->'+strings[name])
    return {'types':sorted(set(types)), 'methods':sorted(set(methods)),
            'dependency_strings':sorted(set(s for s in strings if re.search(r'^(?:/dev/|/vendor/|/system/)|(?:^lib[^ /]+\.so$)|config_ims_|com\.shannon|pixelims',s,re.I)))}


def candidates(rows,name):
    return sorted({r['path'] for r in rows if r['path'].rsplit('/',1)[-1]==name and 'needed' in r})


def native_graph(rows,roots):
    nodes={r['path']:r for r in rows if 'needed' in r}
    todo=list(roots);seen=set();edges=[];missing=[];ambiguous=[]
    while todo:
        path=todo.pop()
        if path in seen:continue
        if path not in nodes:raise ValueError('Missing graph root: '+path)
        seen.add(path)
        for name in nodes[path]['needed']:
            matches=candidates(rows,name)
            edge={'from':path,'needed':name,'candidates':matches};edges.append(edge)
            if not matches:missing.append({'from':path,'needed':name})
            elif len(matches)>1:ambiguous.append(edge)
            todo.extend(matches)
    return {'roots':roots,'files':sorted(seen),'edges':edges,'unresolved':missing,
            'ambiguous':ambiguous,'runtime_resolved':False,
            'scope':'Conservative basename candidate graph only; Android linker namespaces, symbol versions, dlopen, Binder and Java dependencies are not resolved.'}


def init_services(text):
    result=[];current=None
    for line in text.splitlines():
        words=line.strip().split()
        if not words or words[0].startswith('#'):continue
        if words[0]=='service' and len(words)>=3:
            current={'name':words[1],'command':' '.join(words[2:]),'options':[]};result.append(current)
        elif words[0] in ('on','import'):
            current=None
        elif current is not None:current['options'].append(line.strip())
    return result


def audit_file(path,root,raw_dir):
    data=path.read_bytes();name='/'+str(path.relative_to(root))
    row={'path':name,'size':len(data),'sha256':hashlib.sha256(data).hexdigest()}
    if path.suffix in ('.apk','.jar') and zipfile.is_zipfile(path):
        with zipfile.ZipFile(path) as z:
            row['dex']={n:dex_references(z.read(n)) for n in z.namelist() if re.fullmatch(r'classes\d*\.dex',n)}
            row['bundled_native']=[n for n in z.namelist() if n.endswith('.so')]
        if path.suffix=='.apk':
            result=subprocess.run(['aapt','dump','xmltree',str(path),'AndroidManifest.xml'],capture_output=True,text=True,timeout=60)
            if result.returncode:raise RuntimeError(f'aapt failed for {name}: {result.stderr}')
            out=raw_dir/path.relative_to(root).with_suffix('.manifest.txt');out.parent.mkdir(parents=True,exist_ok=True);out.write_text(result.stdout)
            row['manifest']=result.stdout
    elif data.startswith(b'\x7fELF'):
        from elftools.elf.elffile import ELFFile
        elf=ELFFile(io.BytesIO(data));dyn=elf.get_section_by_name('.dynamic')
        row['needed']=[t.needed for t in dyn.iter_tags() if t.entry.d_tag=='DT_NEEDED'] if dyn else []
        row['runtime_strings']=sorted(set(s.decode(errors='replace') for s in re.findall(rb'[ -~]{5,}',data) if re.search(rb'/dev/|/mnt/|/vendor/|/data/vendor/|^lib[^ /]+\.so$|^android\.hardware\.|^vendor\..*\.(?:service|radio)',s)))
    elif path.suffix in ('.xml','.rc','.prop'):
        row['text']=data.decode(errors='replace')
        if path.suffix=='.rc':row['services']=init_services(row['text'])
    return row


def check_telephony_evidence(evidence,resources):
    errors=[]
    if evidence.get('hardware_verified') is not False:errors.append('hardware_claim_not_allowed')
    files={r['path']:r for r in evidence['files']}
    requirements={
        '/system_ext/priv-app/ShannonIms/ShannonIms.apk':('missing_shannon_ims',['com.shannon.imsservice.ShannonImsService','android.permission.BIND_IMS_SERVICE','android.telephony.ims.ImsService']),
        '/system/priv-app/TeleService/TeleService.apk':('missing_phone_process',['com.android.phone','android.uid.phone']),
        '/system_ext/priv-app/PixelImsMediaService/PixelImsMediaService.apk':('missing_ims_media',['com.android.telephony.imsmedia.ImsMediaController','android.uid.phone']),
        '/product/priv-app/ImsServiceEntitlement/ImsServiceEntitlement.apk':('missing_entitlement',['com.android.imsserviceentitlement']),
        '/product/priv-app/EuiccGoogle/EuiccGoogle.apk':('missing_esim_manager',['com.google.android.euicc'])}
    for path,(error,needles) in requirements.items():
        manifest=files.get(path,{}).get('manifest','')
        if not all(n in manifest for n in needles):errors.append(error)
    selected=[r for r in resources if r['path']=='/vendor/overlay/TeleService__frankel__auto_generated_rro_vendor.apk']
    text=json.dumps(selected)
    if 'config_ims_mmtel_package' not in text or 'com.shannon.imsservice' not in text:errors.append('missing_device_ims_selection')
    shannon=files.get('/system_ext/priv-app/ShannonIms/ShannonIms.apk',{})
    methods={m for d in shannon.get('dex',{}).values() for m in d['methods']}
    for ref in ['Landroid/net/ConnectivityManager;->requestNetwork','Landroid/telephony/CarrierConfigManager;->getConfigForSubId','Landroid/os/PowerManager;->newWakeLock']:
        if ref not in methods:errors.append('missing_reference:'+ref)
    return errors


def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('input',type=Path);ap.add_argument('output',type=Path);args=ap.parse_args()
    args.output.mkdir(parents=True,exist_ok=True)
    rows=[audit_file(p,args.input,args.output/'manifests') for p in sorted(args.input.rglob('*')) if p.is_file()]
    result={'hardware_verified':False,'analysis':'Static references are not proof of execution or complete runtime dependencies. DEX method lists include definitions and references; reflection/native dependencies may be absent.', 'files':rows}
    (args.output/'evidence.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'files':len(rows),'apks':sum('manifest' in r for r in rows),'hardware_verified':False}))
if __name__=='__main__':main()
