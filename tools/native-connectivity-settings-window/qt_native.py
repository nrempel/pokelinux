"""Normal Settings Qt6.12 build/install and unavailable component; no normal window."""
import hashlib,json,os,shlex,shutil,stat,struct
from pathlib import Path

def sha(p):
 with Path(p).open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()
def files(p):return {str(q.relative_to(p)):sha(q)for q in sorted(p.rglob('*'))if q.is_file()}
def elf(p):
 h=Path(p).read_bytes()[:20];assert h[:6]==b'\x7fELF\x02\x01'and struct.unpack_from('<H',h,18)[0]==62
 return {'sha256':sha(p),'bytes':Path(p).stat().st_size,'ELF_machine':62}
def link_line(body,name):
 rows=[]
 for line in body.splitlines():
  words=shlex.split(line)
  if '-o'in words and words[words.index('-o')+1]==name:rows.append((line,words))
 assert len(rows)==1;return rows[0]
def strong(body,prefix,expected):
 rows=[line.split()for line in body.splitlines()if line.startswith(prefix)]
 assert all(len(v)>=3 and v[1]=='T'for v in rows)
 names=[v[0]for v in rows];assert sorted(names)==expected and len(names)==len(set(names));return sorted(names)
def build(command,R,actual_receipts):
 R=Path(R);source=R/'source/qt-source';out=R/'native-settings-qt-build';out.mkdir();saved=R/'built-settings';saved.mkdir()
 S=json.loads((R/'selection.json').read_text());before=files(source);assert before==S['Qt_source_files']
 settings=S['settings_client_engine_provider'];appearance=S['appearance_client_engine_provider'];providers={}
 for name,provider,subdir in [('settings',settings,'native-settings-ui/client-core'),('appearance',appearance,'native-app/appearance-client-core')]:
  archive=R/provider['archive']['relative_path'];m=archive.lstat();assert stat.S_ISREG(m.st_mode)and not archive.is_symlink()and m.st_size==provider['archive']['bytes']and sha(archive)==provider['archive']['sha256']
  header=source/provider['compiled_header']['source_relative_path'];assert header.stat().st_size==provider['compiled_header']['bytes']and sha(header)==provider['compiled_header']['sha256']
  for path,row in provider['compiled_files'].items():
   p=source/subdir/path;assert p.stat().st_size==row['bytes']and sha(p)==row['sha256']
  providers[name]=(provider,archive,header)
 versions=command(['/usr/bin/pkg-config','--modversion','Qt6Core','Qt6Gui','Qt6Qml','Qt6Quick','Qt6QuickControls2'],'Settings-Qt-module-versions.log',seconds=5)
 assert versions.splitlines()==['6.12.0']*5
 configure=['/usr/bin/cmake','-S',str(source/'native-settings-ui'),'-B',str(out),'-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DCMAKE_INSTALL_PREFIX=/usr','-DCMAKE_CXX_FLAGS=-Wall -Wextra -Werror']
 for name,(p,a,h)in providers.items():configure.extend(['-DPOKELINUX_'+name.upper()+'_CLIENT_ARCHIVE='+str(a),'-DPOKELINUX_'+name.upper()+'_CLIENT_ARCHIVE_SHA256='+p['archive']['sha256']])
 command(configure,'Settings-Qt-configure.log',seconds=60)
 build_argv=['/usr/bin/cmake','--build',str(out),'--parallel','8','--target','pokelinux-settings','settings-client-component-test']
 command(build_argv,'Settings-Qt-build.log',seconds=180,limit=4194304)
 normal=out/'pokelinux-settings';component=out/'settings-client-component-test';binaries={p.name:elf(p)for p in [normal,component]}
 links=command(['/usr/bin/ninja','-C',str(out),'-t','commands','pokelinux-settings','settings-client-component-test'],'Settings-actual-Ninja-commands.log',seconds=5)
 normal_link,normal_words=link_line(links,'pokelinux-settings');component_link,component_words=link_line(links,'settings-client-component-test')
 for _,archive,_ in providers.values():assert normal_words.count(str(archive))==component_words.count(str(archive))==1
 assert not any('libpokelinux_messages_'in v or 'libpokelinux_battery_'in v for v in normal_words)
 compilation={}
 for relative in ['native-settings-ui/SettingsClient.cpp','native-settings-ui/main.cpp','native-app/PhoneAppearanceClient.cpp','native-settings-ui/tests/component.cpp']:
  rows=[]
  for line in links.splitlines():
   words=shlex.split(line)
   if '-c'in words and words[words.index('-c')+1]==str(source/relative):rows.append((line,words))
  assert len(rows)==1;line,words=rows[0];assert '-O3'in words and all(v in words for v in ['-Wall','-Wextra','-Werror'])and not any(v.startswith('-Wno-')for v in words)
  if relative.endswith('tests/component.cpp'):assert [v for v in words if v in ['-DNDEBUG','-UNDEBUG']]==['-DNDEBUG','-UNDEBUG']
  compilation[relative]=line
 nm=command(['/usr/bin/nm','--format=posix','--extern-only','--defined-only',str(normal)],'Settings-normal-defined-symbols.log',seconds=5)
 for name,(p,archive,header)in providers.items():
  expected=p['strong_C_API'];actual=strong(nm,'pr_'+name+'_',expected)
  archive_nm=command(['/usr/bin/nm','--format=posix','--extern-only','--defined-only',str(archive)],name+'-archive-defined-symbols.log',seconds=5)
  assert strong(archive_nm,'pr_'+name+'_',expected)==actual
 loader=command(['/usr/bin/ldd','-r',str(normal)],'Settings-ldd-r.log',seconds=5)
 assert loader.strip()and'not found'not in loader.lower()and'undefined symbol'not in loader.lower()
 command(['/usr/bin/cmake','--install',str(out)],'Settings-Qt-install.log',seconds=30)
 installed={}
 for dest,origin,mode in [(Path('/usr/libexec/pokelinux/pokelinux-settings'),normal,0o755),(Path('/usr/share/pokelinux/appearance/PhoneAppearance.qml'),source/'native-app/PhoneAppearance.qml',0o644),(Path('/usr/share/applications/org.omarchy.Settings.desktop'),source/'native-settings-ui/org.omarchy.Settings.desktop',0o644)]:
  m=dest.lstat();assert stat.S_ISREG(m.st_mode)and m.st_uid==m.st_gid==0 and stat.S_IMODE(m.st_mode)==mode and sha(dest)==sha(origin)
  installed[str(dest)]={'sha256':sha(dest),'UID':m.st_uid,'GID':m.st_gid,'mode':f'{mode:04o}'}
 runtime=Path('/tmp/settings-component-runtime');runtime.mkdir(mode=0o700);os.chown(runtime,11050,11050)
 env=dict(os.environ,QT_QPA_PLATFORM='offscreen',QSG_RHI_BACKEND='software',XDG_RUNTIME_DIR=str(runtime))
 component_body=command([str(component),'/usr/share/pokelinux/appearance'],'Settings-C++-component.log',seconds=10,limit=131072,env=env,identity={'user':11050,'group':11050,'extra_groups':[]})
 expected=['PASS C++ C ABI layout/refusal/actual_join','PASS real adapters and normal QML construction without window']
 assert [v for v in component_body.splitlines()if v.startswith('PASS ')]==expected
 component_receipts=[v for v in actual_receipts if v['role'].endswith('Settings-C++-component.log')];assert len(component_receipts)==1
 for path in [normal,component]:shutil.copyfile(path,saved/path.name);(saved/path.name).chmod(0o555);assert elf(saved/path.name)==binaries[path.name]
 assert files(source)==before
 for p,a,h in providers.values():assert sha(a)==p['archive']['sha256']and sha(h)==p['compiled_header']['sha256']
 result={'scope':'Fresh native x86 Qt6.12 normal Settings CMake build/install plus C++ ABI and actual unavailable adapter/QML construction; no normal window or actor/Source/driver integration','binaries':binaries,'Qt_module_versions':versions,'configure_argv':configure,'build_argv':build_argv,'actual_normal_link_command':normal_link,'actual_component_link_command':component_link,'actual_compile_commands':compilation,'actual_strong_C_API':{name:p['strong_C_API']for name,(p,a,h)in providers.items()},'exactly_one_each_selected_Rust_archive':True,'actual_ldd_r_no_missing_or_undefined':True,'native_install_manifest':installed,'source_files_before_and_after':before,'component_consumer':{'actual_UID':11050,'stdout_sha256':hashlib.sha256(component_body.encode()).hexdigest(),'actual_wait_EOF':component_receipts[0],'normal_QML_resource_created':True,'installed_shared_appearance_QML_created':True,'real_appearance_and_Settings_adapters':True,'window_shown':False},'normal_app_started_by_this_build_helper':False,'Source_compositor_GUI_auth_phone_claimed':False,'outer_compiler_build_wait_scope':'Actual CMake/Ninja child waits; no separate compiler direct-wait claim'}
 (R/'native-Qt-build-result.json').write_text(json.dumps(result,indent=2)+'\n');return saved/'pokelinux-settings',result
