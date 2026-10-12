#!/usr/bin/env python3
"""Start the pinned released Omarchy shell on actual guest Hyprland."""
import hashlib,json,os,platform,shutil,signal,subprocess,sys,tarfile,time,traceback
from pathlib import Path
OUT=Path('/results');processes=[];logs=[]
phone_plugin='--phone-plugin' in sys.argv
phone_android='--android' in sys.argv
phone_upgrade='--phone-upgrade' in sys.argv
phone_lifecycle='--phone-lifecycle' in sys.argv or phone_upgrade
phone_broken='--phone-broken' in sys.argv
phone_recovery='--phone-recovery' in sys.argv
assert not (phone_recovery and (phone_android or phone_upgrade or phone_lifecycle))
result={'passed':False,'hardware_verified':False,'architecture':platform.machine(),'graphics':'virtio-gpu/virgl/host llvmpipe, not Pixel PowerVR'}
def run(command,env=None):
    try:return subprocess.check_output(command,env=env,text=True,stderr=subprocess.STDOUT,timeout=30)
    except subprocess.CalledProcessError as error:
        (OUT/'failed-command.txt').write_text(str(command)+'\n'+error.output)
        raise
def start(command,name,env=None):
    log=(OUT/name).open('w');logs.append(log);proc=subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT,env=env,start_new_session=True);processes.append(proc);return proc
def wait(predicate,timeout=150):
    end=time.monotonic()+timeout
    while time.monotonic()<end:
        value=predicate()
        if value:return value
        time.sleep(.3)
    raise TimeoutError('Guest condition')
def user(command,env):return ['runuser','-u','alarm','--','env',*[f'{k}={v}' for k,v in env.items()],*command]
try:
    os.chown(OUT,1000,1000)
    root=Path('/tmp/omarchy');root.mkdir()
    archive=Path('/workspace/sim/omarchy-source.tar')
    result['source_archive_sha256']=hashlib.sha256(archive.read_bytes()).hexdigest()
    with tarfile.open(archive) as source:source.extractall(root,filter='data')
    manifest=json.loads(Path('/workspace/sim/omarchy-source/source-manifest.json').read_text())
    result['source_commit']=manifest['commit']
    sys.path.insert(0,'/workspace/tools')
    from omarchy_theme_patch import apply
    result['patched_sources']=apply(root,manifest)
    result['compatibility_patch_sha256']=hashlib.sha256(Path('/workspace/sim/omarchy-qt612.patch').read_bytes()).hexdigest()
    run(['modprobe','virtio_gpu']);run(['install','-d','-o','alarm','-g','alarm','-m','700','/run/user/1000']);run(['udevadm','settle'])
    start(['seatd','-g','video'],'seatd.log',dict(os.environ,SEATD_VTBOUND='0'))
    config=Path('/tmp/omarchy-hyprland.lua');config.write_text('''hl.monitor({ output = "", mode = "800x600@60", position = "auto", scale = 1 })
hl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true } })
hl.permission("/usr/bin/grim", "screencopy", "allow")
''')
    env={'XDG_RUNTIME_DIR':'/run/user/1000','HOME':'/home/alarm','LIBSEAT_BACKEND':'seatd','AQ_NO_MODIFIERS':'1','XDG_SESSION_TYPE':'wayland','QT_QPA_PLATFORM':'wayland','OMARCHY_PATH':str(root),'PATH':str(root/'bin')+':/usr/bin'}
    compositor=start(user(['Hyprland','--config',str(config)],env),'hyprland.log')
    sock=wait(lambda:next((p for p in Path('/run/user/1000').glob('wayland-*') if p.is_socket()),None));env['WAYLAND_DISPLAY']=sock.name
    control=wait(lambda:next((p for p in Path('/run/user/1000/hypr').glob('*/.socket.sock') if p.is_socket()),None));env['HYPRLAND_INSTANCE_SIGNATURE']=control.parent.name
    def ready():
        if compositor.poll() is not None:raise RuntimeError('Hyprland stopped')
        try:return bool(json.loads(run(user(['hyprctl','-j','monitors'],env))))
        except (subprocess.CalledProcessError,json.JSONDecodeError):return False
    wait(ready)
    if phone_plugin:
        package=Path('/workspace/artifacts/client/omarchy-phone-0.6.0-1-any.pkg.tar')
        result['package_sha256']=hashlib.sha256(package.read_bytes()).hexdigest()
        initial_package=Path('/workspace/artifacts/client/omarchy-phone-0.2.0-1-any.pkg.tar') if phone_upgrade else package
        run(['pacman','-U','--noconfirm',str(initial_package)])
        plugin_package=Path('/workspace/artifacts/client/omarchy-phone-plugin-0.4.0-1-any.pkg.tar')
        result['plugin_package_sha256']=hashlib.sha256(plugin_package.read_bytes()).hexdigest()
        initial_plugin=Path('/workspace/artifacts/client/omarchy-phone-plugin-0.3.0-1-any.pkg.tar') if phone_upgrade else plugin_package
        run(['pacman','-U','--noconfirm',str(initial_plugin)])
        plugin=Path('/home/alarm/.config/omarchy/plugins/nbrempel.phone')
        assert not plugin.exists(), 'Package installation must not register for a user'
        for action in ('register','register','unregister','unregister','register'):
            run(user(['omarchy-phone-plugin',action],env))
        assert plugin.is_symlink() and plugin.resolve()==Path('/usr/share/omarchy-phone/plugin')
        def verify_plugin_archive(selected):
            installed={str(p.relative_to(plugin)):hashlib.sha256(p.read_bytes()).hexdigest()
                for p in plugin.rglob('*') if p.is_file()}
            prefix='usr/share/omarchy-phone/plugin/'
            with tarfile.open(selected) as archive:
                expected={m.name[len(prefix):]:hashlib.sha256(archive.extractfile(m).read()).hexdigest()
                    for m in archive if m.isfile() and m.name.startswith(prefix)}
            assert installed==expected,(installed,expected)
            assert len(installed)==8
            return installed
        result['installed_plugin_sources']=verify_plugin_archive(initial_plugin)
        result['plugin_registration_lifecycle']=True
        if phone_broken:
            assert not phone_android
            (plugin/'Panel.qml').write_text('import MissingPhoneTestModule 1.0\nBrokenPanel {}\n')
        if phone_android:
            sys.path.insert(0,'/workspace/framework/tests')
            import installed_service_probe
            pairing=json.loads(Path('/workspace/android-framework/enrollment-client.json').read_text())
            pairing['host']='10.0.2.2'
            result['installed_service']=installed_service_probe.prepare(pairing)
            result['installed_files']={p.name:hashlib.sha256(p.read_bytes()).hexdigest()
                for p in Path('/usr/lib/omarchy-phone').glob('*.py')}
            def admissions():
                import sqlite3
                with sqlite3.connect('file:/home/alarm/.local/state/omarchy-phone/operations.sqlite?mode=ro',uri=True) as db:
                    return db.execute('SELECT epoch,id,state,result FROM operations ORDER BY rowid').fetchall()
            result['native_outgoing']=installed_service_probe.probe_outgoing()
            baseline_admissions=admissions()
            assert len(baseline_admissions)==2
        else:
            base=Path('/tmp/phone-shell-fixture');base.mkdir(mode=0o700);os.chown(base,1000,1000)
            fixture=start(user(['python3','/workspace/omarchy-plugin/tests/service-fixture.py',
                        '/run/user/1000/omarchy-phone','--idle',*(['--recovery','--hold-answer'] if phone_recovery else []),*(['--seed-history'] if phone_upgrade else [])],env),'phone-fixture.log')
            wait(lambda:(base/'ready').exists(),15)
    shell=start(user(['dbus-run-session','--','quickshell','-p',str(root/'shell')],env),'omarchy-shell.log')
    def ping():
        if shell.poll() is not None:raise RuntimeError('Omarchy shell stopped; inspect omarchy-shell.log')
        try:return run(user(['quickshell','ipc','-n','-p',str(root/'shell'),'call','shell','ping'],env)).strip()=='ok'
        except subprocess.CalledProcessError:return False
    wait(ping,90)
    def plugins_ready():
        if shell.poll() is not None:raise RuntimeError('Omarchy shell stopped during discovery')
        raw=run(user(['quickshell','ipc','-n','-p',str(root/'shell'),'call','shell','listPlugins'],env))
        try:plugins=json.loads(raw)
        except json.JSONDecodeError:return False
        if any(p.get('id')=='omarchy.bar' for p in plugins):
            result['plugins']=plugins
            return True
        return False
    wait(plugins_ready,90)
    if phone_plugin:
        def ipc(*args):return run(user(['quickshell','ipc','-n','-p',str(root/'shell'),'call','shell',*args],env)).strip()
        assert any(p.get('id')=='nbrempel.phone' for p in result['plugins']), 'Phone plugin not registered'
        if not phone_upgrade:
            disabled=subprocess.run(user(['omarchy-phone'],env),text=True,capture_output=True,timeout=15)
            assert disabled.returncode==1 and 'disabled' in disabled.stderr, disabled
            result['launcher_disabled_rejected']=True
        assert ipc('enablePlugin','nbrempel.phone','{}')=='ok'
        if phone_broken:
            broken=subprocess.run(user(['omarchy-phone'],env),text=True,capture_output=True,timeout=15)
            result['broken_panel_launcher']={'returncode':broken.returncode,'stderr':broken.stderr}
            assert broken.returncode==1, 'Launcher reported success for unloadable panel'
            assert 'could not' in broken.stderr.lower(), broken.stderr
            assert not (base/'transmissions.json').exists()
            result['passed']=True
            sys.exit(0)
        config_file=Path('/home/alarm/.config/omarchy/shell.json')
        wait(lambda:config_file.exists(),5)
        assert 'nbrempel.phone' in config_file.read_text(), 'Plugin enable was not saved'
        def layers():return json.loads(run(user(['hyprctl','-j','layers'],env)))
        def phone_visible():return 'nbrempel-phone' in json.dumps(layers())
        def panel_state(state):
            view=json.loads(ipc('call','nbrempel.phone','status',''))
            return view if view['opened'] and view['callState']==state and not view['busy'] and (view['available'] or state==0) else None
        def recovery_panel(state):
            view=json.loads(ipc('call','nbrempel.phone','status',''))
            return view if view['opened'] and view['callState']==state and view.get('canAcknowledge') else None
        time.sleep(2)
        assert not phone_visible(), 'Idle panel opened without user request'
        if phone_upgrade:
            before=json.loads(run(user(['omarchy-phone-local'],env)))['snapshot']['epoch']
            import sqlite3
            def history():
                with sqlite3.connect('file:'+str(base/'state/operations.sqlite')+'?mode=ro',uri=True) as db:
                    return db.execute('SELECT epoch,id,digest,state,result FROM operations ORDER BY rowid').fetchall()
            prior_history=history();assert len(prior_history)==1 and prior_history[0][3]=='finished'
            result['upgrade_before']=run(['pacman','-Q','omarchy-phone','omarchy-phone-plugin'])
            os.killpg(shell.pid,signal.SIGTERM);shell.wait(timeout=15)
            os.killpg(fixture.pid,signal.SIGTERM);fixture.wait(timeout=15)
            (base/'ready').unlink()

            (OUT/'upgrade-pacman.txt').write_text(run(['pacman','-U','--noconfirm',str(package),str(plugin_package)]))
            result['upgrade_after']=run(['pacman','-Q','omarchy-phone','omarchy-phone-plugin'])
            assert 'omarchy-phone 0.6.0-1' in result['upgrade_after']
            assert 'omarchy-phone-plugin 0.4.0-1' in result['upgrade_after']
            assert Path('/usr/lib/omarchy-phone/phone_launch.py').read_bytes()==Path('/workspace/transport/phone_launch.py').read_bytes()
            result['installed_plugin_sources']=verify_plugin_archive(plugin_package)
            for name,digest in result['installed_plugin_sources'].items():
                assert digest==hashlib.sha256((Path('/workspace/omarchy-plugin')/name).read_bytes()).hexdigest(),name
            fixture=start(user(['python3','/workspace/omarchy-plugin/tests/service-fixture.py',
                '/run/user/1000/omarchy-phone','--idle'],env),'phone-fixture-upgraded.log')
            wait(lambda:(base/'ready').exists(),15)
            assert history()==prior_history, 'Upgrade changed retained command history'
            result['upgrade_preserved_history']=prior_history
            result['upgrade_daemon_epoch_before']=before

        if phone_lifecycle:
            assert not phone_android, 'Lifecycle fixture currently uses synthetic Android'
            if shell.poll() is None:
                os.killpg(shell.pid,signal.SIGTERM);shell.wait(timeout=15)
            wait(lambda:not phone_visible(),10)
            remaining=subprocess.run(['pgrep','-u','1000','-x','quickshell'],capture_output=True,text=True)
            old=[]
            for pid in remaining.stdout.split():
                status=Path('/proc')/pid/'status'
                if status.exists():
                    state=next(line for line in status.read_text().splitlines() if line.startswith('State:'))
                    if 'Z (zombie)' not in state:old.append({'pid':pid,'state':state})
            assert not old, 'Old shell still live: '+str(old)
            (base/'incoming').touch()
            def observed_ringing():
                value=json.loads(run(user(['omarchy-phone-local'],env)))
                calls=value.get('snapshot',{}).get('controls',{}).get('value',{}).get('calls',[])
                return value.get('available') and len(calls)==1 and calls[0]['state']==2
            wait(observed_ringing,10)
            assert not (base/'transmissions.json').exists()
            shell=start(user(['dbus-run-session','--','quickshell','-p',str(root/'shell')],env),'omarchy-shell-restarted.log')
            wait(ping,90);wait(plugins_ready,90)
            assert any(p.get('id')=='nbrempel.phone' and p.get('enabled') for p in result['plugins'])
            result['shell_restart_observed_new_call']=True
            if phone_upgrade:
                after=json.loads(run(user(['omarchy-phone-local'],env)))['snapshot']['epoch']
                assert after!=before
                result['upgrade_daemon_epoch_after']=after
                result['upgrade_restarted_daemon']=True
        if phone_android:
            (OUT/'ready.json').write_text(json.dumps({'architecture':platform.machine(),'mode':'native-omarchy-android','installed_files':result['installed_files']}))
        elif not phone_lifecycle:(base/'incoming').touch()
        wait(phone_visible,90 if phone_android else 15)
        wait(lambda:recovery_panel(2) if phone_recovery else panel_state(2),15)
        run(user(['grim',str(OUT/'phone-incoming.png')],env))
        ipc('hide','nbrempel.phone')
        wait(lambda:not phone_visible(),10)
        time.sleep(2)
        assert not phone_visible(), 'Same ringing call reopened a dismissed panel'
        assert (admissions()==baseline_admissions if phone_android else not (base/'transmissions.json').exists()), 'Visibility change submitted a call command'
        assert 'Exec=omarchy-phone\n' in Path('/usr/share/applications/omarchy-phone.desktop').read_text()
        launcher=['gio','launch','/usr/share/applications/omarchy-phone.desktop'] if phone_lifecycle else ['omarchy-phone']
        run(user(launcher,env))
        result['installed_native_launcher']=True
        wait(phone_visible,10)
        run(user(['grim',str(OUT/'phone-reopened.png')],env))
        if phone_lifecycle:
            run(user(launcher,env));time.sleep(1)
            assert phone_visible(), 'Repeated launch hid the panel'
            count=sum(1 for monitor in layers().values() for entries in monitor['levels'].values()
                      for entry in entries if entry.get('namespace')=='nbrempel-phone')
            assert count==1,count
            assert not (base/'transmissions.json').exists(), 'Desktop launch sent a call command'
            result['desktop_entry_activation']={'method':'gio launch','repeated_launch_one_panel':True,'no_call_commands':True}

        from evdev import UInput,AbsInfo,ecodes as E
        axes=lambda:AbsInfo(value=0,min=0,max=32767,fuzz=0,flat=0,resolution=0)
        capabilities={E.EV_KEY:[E.BTN_TOUCH],E.EV_ABS:[(E.ABS_X,axes()),(E.ABS_Y,axes()),
            (E.ABS_MT_SLOT,AbsInfo(0,0,9,0,0,0)),(E.ABS_MT_TRACKING_ID,AbsInfo(0,0,65535,0,0,0)),
            (E.ABS_MT_POSITION_X,axes()),(E.ABS_MT_POSITION_Y,axes())]}
        def phone_layer():
            matches=[]
            for monitor in layers().values():
                for entries in monitor['levels'].values():
                    matches.extend(p for p in entries if p.get('namespace')=='nbrempel-phone')
            assert len(matches)==1,matches
            return matches[0]
        def snapshot():
            return json.loads(run(user(['/usr/bin/omarchy-phone-local','snapshot'],env)))['snapshot']
        def state_is(state):
            s=snapshot(); calls=s['controls']['value']['calls'] if s['controls']['fresh'] else None
            return calls is not None and (not calls if state==0 else len(calls)==1 and calls[0]['state']==state)
        with UInput(capabilities,name='Omarchy plugin test touchscreen',input_props=[E.INPUT_PROP_DIRECT]) as touch:
            run(['udevadm','settle'])
            wait(lambda:'Omarchy plugin test touchscreen'.lower().replace(' ','-') in run(user(['hyprctl','-j','devices'],env)).lower(),10)
            (OUT/'touch-devices.json').write_text(run(user(['hyprctl','-j','devices'],env)))
            result['touches']=[]
            def tap(action,tracking):
                layer=phone_layer()
                x=layer['x']+(40+(layer['w']-96)/4 if action=='answer' else layer['w']/2)
                y=layer['y']+layer['h']-72
                result['touches'].append({'action':action,'x':x,'y':y,'layer':layer})
                x=int(x*32767/800);y=int(y*32767/600)
                for code,value in [(E.ABS_MT_SLOT,0),(E.ABS_MT_TRACKING_ID,tracking),(E.ABS_MT_POSITION_X,x),
                                   (E.ABS_MT_POSITION_Y,y),(E.ABS_X,x),(E.ABS_Y,y)]:touch.write(E.EV_ABS,code,value)
                touch.write(E.EV_KEY,E.BTN_TOUCH,1);touch.syn();time.sleep(.15)
                touch.write(E.EV_ABS,E.ABS_MT_TRACKING_ID,-1);touch.write(E.EV_KEY,E.BTN_TOUCH,0);touch.syn()
            time.sleep(1)
            if phone_recovery:
                result['initial_recovery']=wait(lambda:recovery_panel(2),15)
                tap('acknowledge',10);wait(lambda:panel_state(2),15)
                assert not (base/'transmissions.json').exists(), 'Acknowledgment dispatched a call command'
            tap('answer',1)
            if phone_recovery:
                wait(lambda:(base/'transmissions.json').exists(),10)
                before_restart=json.loads((base/'transmissions.json').read_text())
                assert len(before_restart)==1
                os.killpg(shell.pid,signal.SIGKILL);shell.wait(timeout=15)
                wait(lambda:not phone_visible(),10)
                shell=start(user(['dbus-run-session','--','quickshell','-p',str(root/'shell')],env),'omarchy-shell-recovery.log')
                wait(ping,90);wait(plugins_ready,90)
                run(user(['omarchy-phone'],env));wait(phone_visible,10)
                def pending_recovery():
                    view=json.loads(ipc('call','nbrempel.phone','status',''))
                    return view if view['opened'] and view.get('recoveryCount')==1 and view['busy'] and not view['canAcknowledge'] else None
                pending_view=wait(pending_recovery,15)
                result['recovered_pending_worker']=pending_view
                assert json.loads((base/'transmissions.json').read_text())==before_restart
                run(user(['grim',str(OUT/'phone-pending-recovered.png')],env))
                (base/'answer-release').touch()
                recovered=wait(lambda:recovery_panel(4),15)
                assert json.loads(recovered['recoveryKey'])[2:]==json.loads(pending_view['recoveryKey'])[2:]
                assert json.loads(recovered['recoveryKey'])[3]==before_restart[0]['operation_id']
                assert json.loads((base/'transmissions.json').read_text())==before_restart
                run(user(['grim',str(OUT/'phone-recovered.png')],env))
                result['recovery_after_shell_restart']=recovered
                tap('acknowledge',11);wait(lambda:panel_state(4),15)
                assert json.loads((base/'transmissions.json').read_text())==before_restart

            wait(lambda:state_is(4),10)
            result['active_panel']=wait(lambda:panel_state(4),15)
            time.sleep(.3)
            run(user(['grim',str(OUT/'phone-active.png')],env))
            if phone_android:
                (OUT/'active.json').write_text(json.dumps(result['active_panel']))
                wait(lambda:(OUT/'active-verified.json').exists(),45)
            tap('disconnect',2);wait(lambda:state_is(0),10)
            result['idle_panel']=wait(lambda:panel_state(0),15)
            assert result['idle_panel']['title']=='No current call',result['idle_panel']
        if phone_android:
            records=admissions()
            assert records[:2]==baseline_admissions and len(records)==4 and all(row[2]=='finished' and json.loads(row[3])=={'outcome':'submitted','reason':'accepted'} for row in records[2:]),records
            result['admissions']=records
        else:
            transmissions=json.loads((base/'transmissions.json').read_text())
            assert [v['action'] for v in transmissions]==['answer','disconnect'],transmissions
            result['transmissions']=transmissions
        result['phone_plugin']={'registered':True,'incoming_from_closed_panel':True,
            'dismissed_stays_closed':True,'manual_reopen':True,'no_automatic_commands':True,
            'enabled_config_saved':True,'touch_answer_active_end_idle':True,
            'android_adapter':'real enrolled Android emulator' if phone_android else 'synthetic; installed CLI and Unix daemon are real'}
        result['phone_sources']={str(p.relative_to('/workspace/omarchy-plugin')):hashlib.sha256(p.read_bytes()).hexdigest()
            for p in Path('/workspace/omarchy-plugin').rglob('*') if p.suffix in ('.qml','.json','.py')}
        if phone_upgrade:
            assert ipc('setPluginEnabled','nbrempel.phone','false')=='ok'
            wait(lambda:not phone_visible(),10)
            run(user(['omarchy-phone-plugin','unregister'],env))
            assert not plugin.is_symlink()
            os.killpg(shell.pid,signal.SIGTERM);shell.wait(timeout=15)
            (OUT/'remove-pacman.txt').write_text(run(['pacman','-R','--noconfirm','omarchy-phone-plugin']))
            remaining_files=[str(p) for p in Path('/usr/share/omarchy-phone/plugin').rglob('*') if p.is_file() or p.is_symlink()]
            assert not remaining_files, remaining_files
            assert not Path('/usr/bin/omarchy-phone-plugin').exists()
            assert not Path('/usr/lib/omarchy-phone/plugin_install.py').exists()
            result['removed_plugin_files']=True
            run(['pacman','-U','--noconfirm',str(plugin_package)])
            run(user(['omarchy-phone-plugin','register'],env))
            shell=start(user(['dbus-run-session','--','quickshell','-p',str(root/'shell')],env),'omarchy-shell-reinstalled.log')
            wait(ping,90);wait(plugins_ready,90)
            assert any(p.get('id')=='nbrempel.phone' and p.get('enabled') is False for p in result['plugins'])
            disabled=subprocess.run(user(['omarchy-phone'],env),text=True,capture_output=True,timeout=15)
            assert disabled.returncode==1 and 'disabled' in disabled.stderr
            assert json.loads((base/'transmissions.json').read_text())==transmissions
            result['remove_reinstall_preserved_disable']=True
    result['dependencies']=run(['pacman','-Q','quickshell','inotify-tools','jq'])
    result['diagnostics']={}
    for name,imports,expression in (
        ('plain','import qs.Commons','Color'),
        ('qualified','import qs.Commons as C','C.Color'),
        ('versioned','import qs.Commons 1.0 as C','C.Color'),
        ('relative','import "Commons" as C','C.Color')):
        probe=root/'shell'/('color-'+name+'.qml')
        probe.write_text('import QtQuick\nimport Quickshell\n'+imports+'\nShellRoot { Timer { interval: 500; running: true; onTriggered: { var c='+expression+'; console.log("COLOR_PROBE", typeof c, String(c), "background", String(c.background), "bar", String(c.bar), "shellValues", String(c.shellValues)); Qt.quit(); } } }\n')
        output=subprocess.run(user(['dbus-run-session','--','quickshell','-p',str(probe)],env),text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=30)
        (OUT/('color-'+name+'.log')).write_text(output.stdout)
        result['diagnostics'][name]={'returncode':output.returncode,'output':output.stdout}
    time.sleep(2)
    run(user(['grim',str(OUT/'shell.png')],env))
    result['passed']=True
except Exception:
    result['error']=traceback.format_exc();print(result['error'],flush=True)
    if phone_plugin:
        try:run(user(['grim',str(OUT/'phone-failure.png')],env))
        except Exception:pass
finally:
    if phone_android and 'installed_service_probe' in globals():
        try:
            result['final_snapshot']=installed_service_probe.cli()
            result['admissions']=admissions()
        except Exception as error:result['final_capture_error']=type(error).__name__
        try:installed_service_probe.ctl('stop','omarchy-phone.service')
        except Exception:pass
    for proc in reversed(processes):
        if proc.poll() is None:proc.terminate()
    for log in logs:log.close()
    (OUT/'result.json').write_text(json.dumps(result,indent=2)+'\n')
