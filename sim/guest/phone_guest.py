#!/usr/bin/env python3
"""Install the actual client package, render under Hyprland, and inject guest touch."""
import hashlib,json,os,platform,re,subprocess,time,traceback,uuid
from pathlib import Path
from evdev import UInput,AbsInfo,ecodes as E
OUT=Path('/results');processes=[];logs=[]
result={'passed':False,'hardware_verified':False,'architecture':platform.machine(),'graphics':'virtio-gpu/virgl/host llvmpipe, not Pixel PowerVR'}
def run(command,env=None):return subprocess.check_output(command,env=env,text=True,stderr=subprocess.STDOUT,timeout=90)
def start(command,name,env=None):
    log=(OUT/name).open('w');logs.append(log);proc=subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT,env=env);processes.append(proc);return proc
def wait(predicate,timeout=180):
    end=time.monotonic()+timeout
    while time.monotonic()<end:
        value=predicate()
        if value:return value
        time.sleep(.25)
    raise TimeoutError('Guest condition')
def user(command,env):return ['runuser','-u','alarm','--','env',*[f'{k}={v}' for k,v in env.items()],*command]
def gui(op,**values):
    token=uuid.uuid4().hex;temp=OUT/'gui-request.tmp';temp.write_text(json.dumps({'id':token,'op':op,**values}));os.chmod(temp,0o644);temp.replace(OUT/'gui-request.json')
    def response():
        try:value=json.loads((OUT/'gui-response.json').read_text());return value if value['id']==token else None
        except (FileNotFoundError,json.JSONDecodeError):return None
    return wait(response,15)
try:
    assert 'pixel.mode=phone' in Path('/proc/cmdline').read_text()
    os.chown(OUT,1000,1000)
    package=Path('/workspace/artifacts/client/omarchy-phone-0.1.0-1-any.pkg.tar')
    result['package_sha256']=hashlib.sha256(package.read_bytes()).hexdigest()
    result['package_install']=run(['pacman','-U','--noconfirm',str(package)])
    result['package_version']=run(['pacman','-Q','omarchy-phone']).strip()
    run(['/usr/bin/omarchy-phone-pair','--help'])
    config=json.loads(Path('/workspace/android-framework/enrollment-client.json').read_text());config['host']='10.0.2.2'
    path=Path('/tmp/phone-client.json');path.write_text(json.dumps(config));os.chmod(path,0o600);os.chown(path,1000,1000)
    run(['modprobe','virtio_gpu']);run(['modprobe','uinput']);run(['install','-d','-o','alarm','-g','alarm','-m','700','/run/user/1000']);run(['udevadm','settle'])
    start(['seatd','-g','video'],'seatd.log',dict(os.environ,SEATD_VTBOUND='0'))
    config_path=Path('/tmp/phone-hyprland.lua');config_path.write_text('''hl.monitor({ output = "", mode = "800x600@60", position = "auto", scale = 1 })
hl.config({ general = { gaps_in = 0, gaps_out = 0, border_size = 0 }, animations = { enabled = false }, decoration = { rounding = 0, blur = { enabled = false }, shadow = { enabled = false } }, misc = { disable_hyprland_logo = true, disable_splash_rendering = true }, debug = { enable_stdout_logs = true } })
hl.permission("/usr/bin/grim", "screencopy", "allow")
''')
    env={'XDG_RUNTIME_DIR':'/run/user/1000','HOME':'/home/alarm','LIBSEAT_BACKEND':'seatd','AQ_NO_MODIFIERS':'1','XDG_SESSION_TYPE':'wayland','GDK_BACKEND':'wayland'}
    axes=lambda:AbsInfo(value=0,min=0,max=32767,fuzz=0,flat=0,resolution=0)
    capabilities={E.EV_KEY:[E.BTN_TOUCH],E.EV_ABS:[(E.ABS_X,axes()),(E.ABS_Y,axes()),(E.ABS_MT_SLOT,AbsInfo(0,0,9,0,0,0)),(E.ABS_MT_TRACKING_ID,AbsInfo(0,0,65535,0,0,0)),(E.ABS_MT_POSITION_X,axes()),(E.ABS_MT_POSITION_Y,axes())]}
    with UInput(capabilities,name='Omarchy phone test touchscreen',input_props=[E.INPUT_PROP_DIRECT]) as touch:
        run(['udevadm','settle']);compositor=start(user(['Hyprland','--config',str(config_path)],env),'hyprland.log')
        socket=wait(lambda:next((p for p in Path('/run/user/1000').glob('wayland-*') if p.is_socket()),None));env['WAYLAND_DISPLAY']=socket.name
        control=wait(lambda:next((p for p in Path('/run/user/1000/hypr').glob('*/.socket.sock') if p.is_socket()),None));env['HYPRLAND_INSTANCE_SIGNATURE']=control.parent.name
        def compositor_ready():
            if compositor.poll() is not None:raise RuntimeError('Hyprland stopped')
            try:return bool(json.loads(run(user(['hyprctl','-j','monitors'],env))))
            except (subprocess.CalledProcessError,json.JSONDecodeError):return False
        wait(compositor_ready)
        (OUT/'devices.json').write_text(run(user(['hyprctl','-j','devices'],env)))
        errors=run(user(['hyprctl','configerrors'],env)).strip();assert not errors,errors
        app=start(user(['dbus-run-session','--','python3','/workspace/sim/guest/phone_ui_probe.py'],env),'phone-ui.log')
        wait(lambda:(OUT/'gui-ready.json').exists())
        result.update(json.loads((OUT/'gui-ready.json').read_text()))
        (OUT/'ready.json').write_text(json.dumps(result))
        last=None;end=time.monotonic()+700;tracking=0
        while time.monotonic()<end:
            if app.poll() is not None:raise RuntimeError('Phone UI stopped')
            try:request=json.loads((OUT/'request.json').read_text())
            except (FileNotFoundError,json.JSONDecodeError):time.sleep(.1);continue
            if request['id']==last:time.sleep(.1);continue
            last=request['id'];response={'id':last}
            if request['op']=='stop':gui('quit');result['passed']=True;break
            if request['op'] in ('inspect','launch','theme'):response.update(gui(request['op']));response['id']=last
            elif request['op']=='screenshot':
                name=request['name'];assert re.fullmatch(r'[a-z-]+\.png',name),name
                run(user(['grim',str(OUT/name)],env));response['saved']=name
            elif request['op']=='touch':
                geometry=gui('geometry',button=request['button']);assert 'center' in geometry,geometry
                x,y=geometry['center'];x=int(x*32767/800);y=int(y*32767/600);tracking+=1
                for code,value in [(E.ABS_MT_SLOT,0),(E.ABS_MT_TRACKING_ID,tracking),(E.ABS_MT_POSITION_X,x),(E.ABS_MT_POSITION_Y,y),(E.ABS_X,x),(E.ABS_Y,y)]:touch.write(E.EV_ABS,code,value)
                touch.write(E.EV_KEY,E.BTN_TOUCH,1);touch.syn();time.sleep(.2);touch.write(E.EV_ABS,E.ABS_MT_TRACKING_ID,-1);touch.write(E.EV_KEY,E.BTN_TOUCH,0);touch.syn();response['touched']=request['button'];response['center']=geometry['center']
                (OUT/'last-touch.json').write_text(json.dumps(response))
            else:raise ValueError('Unknown test operation')
            temp=OUT/'response.tmp';temp.write_text(json.dumps(response));temp.replace(OUT/'response.json')
        else:raise TimeoutError('Host did not finish UI suite')
        run(user(['hyprctl','dispatch','hl.dsp.exit()'],env));compositor.wait(timeout=30)
except Exception:
    result['error']=traceback.format_exc();print(result['error'],flush=True)
finally:
    for proc in reversed(processes):
        if proc.poll() is None:proc.terminate()
    for log in logs:log.close()
    (OUT/'result.json').write_text(json.dumps(result,indent=2)+'\n')
