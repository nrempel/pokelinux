"""Install one qualified local compositor/SDK cohort in the fresh guest before any SDK build."""
import copy,hashlib,json,os,re,shlex,shutil,stat,tarfile
from pathlib import Path
from qt_native import elf


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()


def regular(path,limit):
    info=Path(path).lstat()
    assert stat.S_ISREG(info.st_mode) and 0<info.st_size<=limit,str(path)
    return info


def root_directory(path,mode=0o755):
    path=Path(path);path.mkdir(parents=True,exist_ok=True)
    assert path.is_dir() and not path.is_symlink()
    os.chown(path,0,0);path.chmod(mode);return path


def actual_manifest(root):
    rows={}
    for path in sorted(Path(root).rglob('*')):
        assert not path.is_symlink(),str(path)
        info=path.lstat();assert info.st_uid==info.st_gid==0,str(path)
        if path.is_dir():assert stat.S_IMODE(info.st_mode)==0o755
        else:
            assert stat.S_ISREG(info.st_mode) and stat.S_IMODE(info.st_mode)==0o644
            rows[str(path.relative_to(root))]={'bytes':info.st_size,'sha256':sha(path)}
    return rows


def install(R,S,command):
    R=Path(R);A=S['source_built_compositor']
    assert A['Root_qualified_local_source_build_only'] is True and A['new_ELF_is_signed_Arch_package'] is False
    assert A['source_commit']=='efb50993780079460b0cbed1363e2166a2de1d9f'
    qualification=R/'local-compositor/Root-build-qualification.json';regular(qualification,8*1024**2)
    assert sha(qualification)==A['Root_qualification_sha256']
    # Host staging must authenticate the closed Root proof and all exact artifact rows first.
    proof_path=R/'local-compositor/build-result.json';regular(proof_path,8*1024**2)
    assert sha(proof_path)==A['build_result_sha256'];B=json.loads(proof_path.read_text())
    assert B['passed'] and B['new_ELF_and_SDK_single_source_cohort'] and not B['compositor_ELF_executed'] and not B['new_ELF_is_signed_Arch_package']
    assert B['phase_manifest_sha256']=='2aa3ce48ad808f5c519c3eaf4a244231b53f34b79410dbbcf8419dd58fabfc37'
    assert B['source_commit']==A['source_commit']
    assert B['exact_one_allowlist_entry_source_delta']==['src/managers/ProtocolManager.cpp']
    assert B['experimental_renderer_AQ_Rust_inputs_selected'] is False
    assert B['patched_files']=={'src/managers/ProtocolManager.cpp':{'bytes':21244,'sha256':'78e9c141146e4b3db19a41b7677c4791b2bf59940e04d89dfb7ed5253edf5269'}}
    expected=B['matching_staged_SDK_manifest'];assert expected==A['SDK_file_manifest'] and expected
    binary=R/'local-compositor/Hyprland';archive=R/'local-compositor/sdk.tar'
    regular(binary,64*1024**2);regular(archive,256*1024**2)
    assert elf(binary)['sha256']==A['new_ELF_sha256']==B['new_ELF']['sha256']
    assert sha(archive)==A['SDK_tar_sha256']==B['SDK_tar']['sha256']
    assert sha('/usr/bin/Hyprland')==A['original_signed_ELF_sha256']!=A['new_ELF_sha256']
    for name,digest in A['original_signed_SDK_pins'].items():assert sha(name)==digest,name
    for name,row in B['direct_installed_library_manifest'].items():
        path=Path(row['resolved_path']);assert path.is_absolute() and str(path).startswith('/usr/lib/') and sha(path)==row['sha256'],name
        assert path.stat().st_size==row['bytes'],name
    stash=root_directory('/var/lib/pokelinux-local-compositor',0o700)
    extracted=root_directory(stash/'new-sdk',0o755)
    allowed=set(expected);parents={str(parent) for n in allowed for parent in Path(n).parents if str(parent)!='.'}
    installed={};count=0;total=0
    with tarfile.open(archive,'r:') as tar:
        for member in tar:
            count+=1;assert count<=8192
            name=Path(member.name);assert name.parts and not name.is_absolute() and '..' not in name.parts
            key=str(name);destination=extracted/name
            assert member.isdir() or member.isfile(),'SDK links/devices are refused'
            if member.isdir():assert key in parents;root_directory(destination);continue
            assert key in allowed and key not in installed and member.size==expected[key]['bytes'] and 0<=member.size<=8*1024**2
            assert key.startswith('usr/include/hyprland/') or key=='usr/share/pkgconfig/hyprland.pc'
            total+=member.size;assert total<=256*1024**2
            root_directory(destination.parent)
            source=tar.extractfile(member);assert source is not None
            with destination.open('xb') as target:
                remaining=member.size
                while remaining:
                    chunk=source.read(min(65536,remaining));assert chunk;target.write(chunk);remaining-=len(chunk)
                assert not source.read(1)
            source.close();os.chown(destination,0,0);destination.chmod(0o644)
            assert sha(destination)==expected[key]['sha256'];installed[key]={'bytes':destination.stat().st_size,'sha256':sha(destination)}
    assert installed==expected and actual_manifest(extracted)==expected
    # Preserve signed originals only in this disposable guest; never compile against the old SDK.
    old_headers=Path('/usr/include/hyprland');old_elf=Path('/usr/bin/Hyprland');old_pc=Path('/usr/share/pkgconfig/hyprland.pc')
    assert old_headers.is_dir() and not old_headers.is_symlink()
    old_headers.rename(stash/'signed-original-headers');old_elf.rename(stash/'signed-original-Hyprland');old_pc.rename(stash/'signed-original-hyprland.pc')
    (extracted/'usr/include/hyprland').rename(old_headers)
    shutil.copyfile(extracted/'usr/share/pkgconfig/hyprland.pc',old_pc);old_pc.chmod(0o644);os.chown(old_pc,0,0)
    shutil.copyfile(binary,old_elf);old_elf.chmod(0o755);os.chown(old_elf,0,0)
    actual={}
    for name,row in expected.items():
        path=Path('/')/name;info=path.lstat();assert stat.S_ISREG(info.st_mode) and info.st_size==row['bytes'] and 0<=info.st_size<=8*1024**2
        assert info.st_uid==info.st_gid==0 and stat.S_IMODE(info.st_mode)==0o644 and sha(path)==row['sha256'];actual[str(path)]=row['sha256']
    new_header_rows={'usr/include/hyprland/'+n:v for n,v in actual_manifest(old_headers).items()}
    assert new_header_rows=={n:v for n,v in expected.items() if n.startswith('usr/include/hyprland/')}
    assert elf(old_elf)['sha256']==A['new_ELF_sha256']
    version=Path('/usr/include/hyprland/src/version.h').read_text()
    assert re.search(r'GIT_COMMIT_HASH\s+"'+A['source_commit']+'"',version)
    assert 'pokelinux-local-logical-output-metadata' in version and re.search(r'GIT_DIRTY\s+"dirty"',version)
    pcdir=command(['/usr/bin/pkg-config','--variable=pcfiledir','hyprland'],'local-SDK-pcfiledir.log',seconds=5).strip()
    assert pcdir=='/usr/share/pkgconfig' and sha(Path(pcdir)/'hyprland.pc')==A['SDK_file_manifest']['usr/share/pkgconfig/hyprland.pc']['sha256']
    prefix=command(['/usr/bin/pkg-config','--variable=prefix','hyprland'],'local-SDK-prefix.log',seconds=5).strip()
    assert prefix==B['generated_hyprland_pc']['declared_variables']['prefix']
    assert sha(Path(pcdir)/'hyprland.pc')==B['generated_hyprland_pc']['sha256']
    flags=shlex.split(command(['/usr/bin/pkg-config','--cflags','hyprland'],'local-SDK-actual-cflags.log',seconds=5));assert '-I/usr/include/hyprland' in flags
    probe=R/'local-SDK-dependency-probe.cpp';probe.write_text('#include <hyprland/src/version.h>\n#include <hyprland/src/event/EventBus.hpp>\n')
    dependencies=command(['/usr/bin/g++','-std=c++23','-M','-MT','local-sdk-pins',*flags,str(probe)],'local-SDK-compiler-dependencies.log',seconds=30,limit=4*1024**2)
    for path in ['/usr/include/hyprland/src/version.h','/usr/include/hyprland/src/event/EventBus.hpp']:assert path in dependencies
    signed_api=json.loads((R/'source/public-api-pins.json').read_text())
    runtime_api=copy.deepcopy(signed_api)
    runtime_api['hyprland']['guest_elf_sha256']=A['new_ELF_sha256']
    runtime_api['installed_public_sdk']={path:digest for path,digest in signed_api['installed_public_sdk'].items() if not path.startswith('/usr/include/hyprland/') and path!='/usr/share/pkgconfig/hyprland.pc'}
    for path,digest in runtime_api['installed_public_sdk'].items():assert sha(path)==digest,path
    runtime_api['installed_public_sdk'].update(actual)
    runtime_api['scope']='Root-qualified local one-entry Hyprland ELF/full matching SDK on the independently preflighted signed367 base; local ELF is not a signed Arch package; runtime unqualified until original assertions pass'
    runtime_api['source_built_compositor']={'phase_manifest_sha256':B['phase_manifest_sha256'],'Root_qualification_sha256':A['Root_qualification_sha256'],'build_result_sha256':A['build_result_sha256'],'new_ELF_is_signed_Arch_package':False}
    linkage=command(['/usr/bin/ldd','-r','/usr/bin/Hyprland'],'local-compositor-loader-relocations.log',seconds=30,limit=1048576)
    assert 'not found' not in linkage and 'undefined symbol:' not in linkage
    return {'scope':'Actual fresh-guest local ELF+matching SDK installation before all SDK consumers; later compositor runtime separate','new_ELF':elf(old_elf),'SDK_actual_file_hashes':actual,'SDK_file_count':len(actual),'SDK_owner_mode':'root:root directories0755/files0644; ELF0755; archive ownership/modes ignored','pkgconfig_actual_file':str(Path(pcdir)/'hyprland.pc'),'pkgconfig_prefix':prefix,'pkgconfig_flags':flags,'compiler_dependency_output_sha256':hashlib.sha256(dependencies.encode()).hexdigest(),'original_signed_cohort_preserved_at':str(stash),'selected_ELF_is_signed_Arch_package':False,'compositor_main_started_by_install':False,'dynamic_loader_exercised_by_ldd_r':True,'new_cohort_no_old_header_overlay':True,'loader_relocation_output_sha256':hashlib.sha256(linkage.encode()).hexdigest(),'runtime_public_api':runtime_api}
