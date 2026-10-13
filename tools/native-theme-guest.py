#!/usr/bin/env python3
"""Actual original Omarchy + Hyprland + native Qt renderer, disposable ARM64 VM.
No host desktop/device access; no stubbed theme/font commands. Synthetic call.
"""
import hashlib,json,os,platform,signal,shutil,subprocess,sys,tarfile,time,traceback
from pathlib import Path
OUT=Path('/results'); project=Path('/workspace'); processes=[];logs=[]
result={'passed':False,'hardware_verified':False,'architecture':platform.machine(),
        'graphics':'virtio-gpu/virgl, container llvmpipe, ARM64 system TCG',
        'android_production_dependency':False,'desktop_commands_stubbed':[], 'steps':[]}
def run(args,env=None,timeout=60):
    # A launched desktop child can retain pipes: use an owned file, bounded wait.
    import tempfile
    with tempfile.TemporaryFile(mode='w+') as output:
        p=subprocess.run(args,env=env,stdout=output,stderr=subprocess.STDOUT,timeout=timeout)
        output.seek(0);text=output.read()
        if p.returncode:
            (OUT/'failed-command.log').write_text(str(args)+'\n'+text)
            raise RuntimeError(f'Command exit {p.returncode}: {args}: {text[-2000:]}')
        return text

def start(args,name,env=None):
    f=(OUT/name).open('w');logs.append(f)
    p=subprocess.Popen(args,stdout=f,stderr=subprocess.STDOUT,env=env,start_new_session=True)
    processes.append(p);return p

def wait(predicate,seconds=120,process=None):
    deadline=time.monotonic()+seconds
    while time.monotonic()<deadline:
        if process and process.poll() is not None:raise RuntimeError(f'Owned process exited {process.returncode}')
        v=predicate()
        if v:return v
        time.sleep(.2)
    raise TimeoutError('Bounded guest condition')
try:
    os.chown(OUT,1000,1000)
    result['kernel']=run(['uname','-a'])
    result['packages']=run(['pacman','-Q','hyprland','quickshell','qt6-base','qt6-declarative','qt6-wayland','fontconfig'])
    config=json.loads((OUT/'run-config.json').read_text())
    consumer=project/'native-app'
    if config['consumer']=='baseline':
        baseline=project/'artifacts/native-theme/baseline'
        assert hashlib.sha256((baseline/'manifest.json').read_bytes()).hexdigest()==config['baseline_manifest_sha256']
        baseline_manifest=json.loads((baseline/'manifest.json').read_text())
        for name,digest in baseline_manifest['files'].items():assert hashlib.sha256((baseline/name).read_bytes()).hexdigest()==digest,name
        consumer=baseline/'native-app'
        result['consumer_baseline']=baseline_manifest
    result['consumer']=str(consumer)
    result['consumer_sources_sha256']={str(p.relative_to(consumer)):hashlib.sha256(p.read_bytes()).hexdigest() for p in consumer.rglob('*') if p.is_file()}
    manifest=json.loads((project/'native-theme/public-manifest.json').read_text())
    archive=project/'artifacts/native-theme/omarchy-public.tar.gz'
    result['archive_sha256']=hashlib.sha256(archive.read_bytes()).hexdigest()
    assert result['archive_sha256']==manifest['archive_sha256']
    source=Path('/tmp/native-theme-original');source.mkdir()
    with tarfile.open(archive) as t:t.extractall(source,filter='data')
    root=source/('omarchy-'+manifest['commit'])
    for name,digest in manifest['files'].items():assert hashlib.sha256((root/name).read_bytes()).hexdigest()==digest,name
    result['source_commit']=manifest['commit']
    sys.path.insert(0,str(project/'tools'))
    from omarchy_theme_patch import apply
    original=json.loads((project/'sim/omarchy-source/source-manifest.json').read_text())
    result['explicit_qt612_namespace_patch']=apply(root,original)
    result['own_sources_sha256']={str(p.relative_to(project)):hashlib.sha256(p.read_bytes()).hexdigest()
        for folder in ['native-app','native-theme'] for p in (project/folder).rglob('*')
        if p.is_file() and p.suffix in {'.cpp','.h','.qml','.js','.json'}}
    build=OUT/'build'
    result['own_tools_sha256']={str(p.relative_to(project)):hashlib.sha256(p.read_bytes()).hexdigest() for p in (project/'tools').glob('*native-theme*') if p.is_file()}
    result['build']=run(['python3',str(project/'tools/build-native-theme-guest.py'),str(build),str(consumer)],timeout=420)
    run(['modprobe','virtio_gpu']);run(['udevadm','settle'])
    run(['usermod','-a','-G','video,input','alarm'])
    run(['install','-d','-o','alarm','-g','alarm','-m','700','/run/user/1000'])
    start(['seatd','-g','video'],'seatd.log',dict(os.environ,SEATD_VTBOUND='0'))
    hypr=Path('/tmp/native-theme.lua')
    hypr.write_text('hl.monitor({ output = "", mode = "800x900@60", position = "auto", scale = 1 })\n'
        'hl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true } })\n'
        'hl.permission("/usr/bin/grim", "screencopy", "allow")\n')
    env={'HOME':'/home/alarm','XDG_RUNTIME_DIR':'/run/user/1000','LIBSEAT_BACKEND':'seatd',
        'AQ_NO_MODIFIERS':'1','XDG_SESSION_TYPE':'wayland','QT_QPA_PLATFORM':'wayland',
        'OMARCHY_PATH':str(root),'PATH':str(root/'bin')+':/usr/local/bin:/usr/bin',
        'OMARCHY_THEME_SKIP_BACKGROUND':'1'}
    def user(args):return ['runuser','-u','alarm','--','env',*[f'{k}={v}' for k,v in env.items()],*args]
    # Login-shell post-hooks see the genuine source, not a shadow stub directory.
    Path('/home/alarm/.bash_profile').write_text(f'export OMARCHY_PATH="{root}"\nexport PATH="$OMARCHY_PATH/bin:/usr/local/bin:/usr/bin"\n')
    plugin=Path('/home/alarm/.config/omarchy/plugins/pokelinux.theme-oracle')
    plugin.parent.mkdir(parents=True,exist_ok=True);shutil.copytree(project/'native-theme/oracle',plugin)
    cfg=Path('/home/alarm/.config/omarchy/shell.json')
    shell_config=json.loads((root/'config/omarchy/shell.json').read_text())
    shell_config['plugins'].append({'id':'pokelinux.theme-oracle'})
    cfg.write_text(json.dumps(shell_config))
    run(['chown','-R','alarm:alarm','/home/alarm/.config'])
    compositor=start(user(['Hyprland','--config',str(hypr)]),'hyprland.log')
    sock=wait(lambda:next((p for p in Path('/run/user/1000').glob('wayland-*') if p.is_socket()),None),process=compositor)
    env['WAYLAND_DISPLAY']=sock.name
    control=wait(lambda:next(Path('/run/user/1000/hypr').glob('*/.socket.sock'),None),process=compositor)
    env['HYPRLAND_INSTANCE_SIGNATURE']=control.parent.name
    run(['systemctl','start','user@1000.service'])
    wait(lambda:Path('/run/user/1000/bus').is_socket(),process=compositor)
    env['DBUS_SESSION_BUS_ADDRESS']='unix:path=/run/user/1000/bus'
    run(user(['systemctl','--user','import-environment',*env.keys()]))
    shell=start(user(['quickshell','-n','-p',str(root/'shell')]),'omarchy-shell.log')
    def ipc(target,method):return run(user(['quickshell','ipc','-n','-p',str(root/'shell'),'call',target,method]),timeout=15).strip()
    def ready():
        try:return ipc('shell','ping')=='ok'
        except RuntimeError:return False
    wait(ready,process=shell)
    def oracle():
        try:return json.loads(ipc('native-theme-oracle','snapshot'))
        except (RuntimeError,json.JSONDecodeError):return False
    wait(oracle,process=shell)
    result['plugins']=json.loads(ipc('shell','listPlugins'))
    assert any(p['id']=='omarchy.bar' for p in result['plugins'])
    # Original generation/switch, with real applyTheme IPC and post-hooks.
    result['theme_tokyo_log']=run(user(['omarchy-theme-set','tokyo-night']),timeout=90)
    result['initial_oracle']=wait(lambda:(o if (o:=oracle()) and o['background']!='#000000' else False),process=compositor)
    native=start(user([str(build/'native-theme-renderer'),env['HOME'],str(consumer),str(OUT)]),'native-renderer.log')
    def clients():return json.loads(run(user(['hyprctl','-j','clients'])))
    windows=wait(lambda:[c for c in clients() if c['pid']==native.pid or 'Native theme conformance' in c.get('title','')],process=native)
    first=windows[0];result['native_window']=first
    assert first['mapped'] and not first['xwayland']
    def snapshot(seq):
        temp=OUT/'control.tmp';temp.write_text(json.dumps({'sequence':seq}));temp.replace(OUT/'control.json')
        return wait(lambda:json.loads((OUT/f'native-{seq}.json').read_text()) if (OUT/f'native-{seq}.json').exists() else False,process=native)
    def compare(name,seq,check_tokens=True):
        # Original FileView replacement/reload and native watchers are async.
        # Require two measured agreements and the native reader's current file
        # generation; a source-derived expected value cannot complete this gate.
        observations=[];stable=0;chosen=None
        deadline=time.monotonic()+15
        for attempt in range(30):
            if time.monotonic()>=deadline:break
            o=oracle();n=snapshot(1000+seq*100+attempt)
            current_user=Path('/home/alarm/.config/omarchy/shell.toml')
            actual_user=current_user.read_text() if current_user.exists() else ''
            agree=bool(o) and n['available'] and n['sameRoot']
            agree=agree and n['background']==o['background'] and n['pixel']==o['background']
            agree=agree and n['files']['userShell']==actual_user
            if check_tokens and o:
                agree=agree and n['numberFont']['pixelSize']==o['display']
                agree=agree and n['numberFont']['resolvedFamily']==o['fontFamily']
                agree=agree and n['fontTokens']['heading']==o['heading']
                agree=agree and n['spacingTokens']['controlGap']==o['controlGap']
                agree=agree and abs(n['fontScale']*12-o['baseSize'])<0.001
            observations.append({'original_shell':o,'native_sequence':n['sequence'],
                'native_number_font':n['numberFont'],'native_heading':n['fontTokens']['heading'],
                'native_control_gap':n['spacingTokens']['controlGap'],'agreement':agree})
            stable=stable+1 if agree else 0
            if stable>=2:chosen=(o,n);break
            time.sleep(.2)
        result.setdefault('convergence_observations',[]).append({'name':name,'observations':observations})
        if chosen is None:
            raise RuntimeError('Original/native appearance did not stably converge: '+name)
        o,n=chosen
        step={'name':name,'original_shell':o,'native_renderer':n}
        result['steps'].append(step)
        assert n['draft']=='5550100' and n['intentId']=='a'*32 and n['unresolved']
        current=[c for c in clients() if c['address']==first['address']]
        assert len(current)==1 and current[0]['pid']==first['pid']
        run(user(['grim',str(OUT/f'desktop-{seq}.png')]))
        return o,n
    compare('original tokyo-night generation + native renderer',0)
    # Original setting updater preserves the own per-token override. No fixed palette.
    settings=Path('/home/alarm/.config/omarchy/shell.toml')
    settings.write_text('[font]\ndisplay = 37\nheading = 31\n[spacing]\ncontrol-gap = 19\n')
    os.chown(settings,1000,1000)
    result['text_size_log']=run(user(['omarchy-display-text-size','18']))
    o,n=compare('original text-size setter + token override',1)
    assert o['baseSize']==18 and o['display']==37
    assert n['fontTokens']['heading']==o['heading'] and n['spacingTokens']['controlGap']==o['controlGap']
    # Two installed fonts selected dynamically; original fc-list validates them.
    families=sorted(set(run(user(['fc-list','-f','%{family[0]}\n'])).splitlines()))
    mono=run(user(['fc-match','-f','%{family[0]}','monospace'])).strip()
    choices=[mono]+[f for f in families if f!=mono and 'Mono' not in f][:1]
    assert len(choices)==2, families
    result['font_choices']=choices
    for i,family in enumerate(choices,2):
        result[f'font_set_{i}_log']=run(user(['omarchy-font-set',family]),timeout=120)
        wait(ready,process=compositor)
        expected_family=run(user(['fc-match','-f','%{family[0]}','monospace'])).strip()
        wait(lambda:(o if (o:=oracle()) and o['fontFamily']==expected_family else False),process=compositor)
        compare('original font-set '+family,i)
    result['theme_latte_log']=run(user(['omarchy-theme-set','catppuccin-latte']),timeout=90)
    o,n=compare('original catppuccin-latte switch preserves settings/window/state',4)
    assert o['background']!=result['steps'][0]['original_shell']['background']
    assert o['baseSize']==18 and o['display']==37
    result['text_reset_log']=run(user(['omarchy-display-text-size','reset']))
    compare('original text-size reset preserves token override',5)
    # Real dependency failure inside the disposable guest, never a host mutation.
    # No replacement executable/stub: exec fails, last concrete native family stays.
    before=n['numberFont']['resolvedFamily']
    run(['mv','/usr/bin/fc-match','/usr/bin/fc-match.native-theme-held'])
    try:
        result['resolver_missing_font_set_log']=run(user(['omarchy-font-set',choices[0]]),timeout=120)
        wait(ready,process=compositor)
        time.sleep(2)
        failed=snapshot(6)
        result['resolver_failure_observation']=failed
        assert not failed['files']['fontResolutionAvailable']
        assert failed['numberFont']['resolvedFamily']==before
        assert failed['draft']=='5550100' and failed['unresolved'] and failed['sameRoot']
    finally:
        run(['mv','/usr/bin/fc-match.native-theme-held','/usr/bin/fc-match'])
    result['resolver_restored_font_set_log']=run(user(['omarchy-font-set',choices[0]]),timeout=120)
    wait(ready,process=compositor)
    expected_family=run(user(['fc-match','-f','%{family[0]}','monospace'])).strip()
    wait(lambda:(o if (o:=oracle()) and o['fontFamily']==expected_family else False),process=compositor)
    o,n=compare('same original setting after resolver restoration recovers live native font',7)
    assert n['files']['fontResolutionAvailable']
    # Genuine original selection validation leaves current native state alone.
    rejected=subprocess.run(user(['omarchy-theme-set','../invalid']),capture_output=True,text=True,timeout=20)
    result['invalid_theme_selection']={'exit_code':rejected.returncode,'output':rejected.stdout+rejected.stderr}
    assert rejected.returncode!=0
    compare('invalid original theme selection preserves current palette/window/draft',8)
    result['passed']=True
except Exception as e:
    result['error']=str(e);result['traceback']=traceback.format_exc()
finally:
    (OUT/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    try:result['restarted_shell_journal']=run(['journalctl','--no-pager','-t','omarchy-shell','-o','cat'],timeout=15)
    except Exception as e:result['journal_collection_error']=str(e)
    (OUT/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    # Kill/reap only owned process groups; guest teardown stops systemd-launched shell.
    for p in reversed(processes):
        if p.poll() is None:
            try:os.killpg(p.pid,signal.SIGTERM)
            except ProcessLookupError:pass
    for p in reversed(processes):
        try:p.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(p.pid,signal.SIGKILL);p.wait(timeout=5)
    for f in logs:f.close()
print(json.dumps(result,indent=2),flush=True)
raise SystemExit(0 if result['passed'] else 101)
