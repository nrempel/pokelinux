#!/usr/bin/env python3
"""Own an isolated original PipeWire/WirePlumber stack and Rust authority."""
import json
import os
import pathlib
import signal
import subprocess
import sys
import time
import array
import math
import hashlib

root = pathlib.Path('/tmp/pokelinux-native-audio')
root.mkdir(mode=0o755)
private = root / 'private'
private.mkdir(mode=0o700)
control = root / 'control'
control.mkdir(mode=0o755)
config = root / 'config'
config.mkdir(mode=0o700)
state = root / 'state'
state.mkdir(mode=0o700)
wp_config = config / 'wireplumber' / 'wireplumber.conf.d'
wp_config.mkdir(parents=True)
(wp_config / 'native-audio.conf').write_text('''wireplumber.profiles = {
  native-audio-test = {
    inherits = [ base ]
    metadata.sm-settings = required
    metadata.sm-objects = required
    policy.standard = disabled
    hardware.audio = disabled
    hardware.bluetooth = disabled
    hardware.video-capture = disabled
    support.logind = disabled
    support.portal-permissionstore = disabled
  }
}
''')

objects = ['''{ factory = spa-node-factory args = {
  factory.name = support.node.driver node.name = Test-Driver priority.driver = 20000
} }''']
for name, factory, media_class in [
    ('source.media', 'audiotestsrc', 'Audio/Source'),
    ('source.microphone', 'audiotestsrc', 'Audio/Source'),
    ('route.speaker', 'support.null-audio-sink', 'Audio/Sink'),
    ('route.earpiece', 'support.null-audio-sink', 'Audio/Sink'),
    ('route.capture', 'support.null-audio-sink', 'Audio/Sink'),
]:
    extra = '' if factory != 'audiotestsrc' else 'node.param.PortConfig = { direction = Output mode = dsp format = { mediaType = audio mediaSubtype = raw format = F32 rate = 48000 channels = 1 position = [ MONO ] } }'
    if factory == 'audiotestsrc':
        extra += ' node.param.Props = { waveType = 0 volume = 0.25 }'
    monitor = 'true' if name in ('route.speaker', 'route.earpiece') else 'false'
    objects.append('''{ factory = adapter args = {
        factory.name = "%s" node.name = "%s" media.class = "%s"
        audio.format = F32 audio.rate = 48000 audio.channels = 1 audio.position = [ MONO ]
        adapter.auto-port-config = { mode = dsp monitor = %s position = preserve }
        %s
    } }''' % (factory, name, media_class, monitor, extra))
pw_config = config / 'pipewire.conf'
pw_config.write_text('''context.properties = {
  core.daemon = true core.name = pokelinux-audio-test
  default.clock.rate = 48000 default.clock.quantum = 256
}
context.spa-libs = {
  audio.convert.* = audioconvert/libspa-audioconvert
  audiotestsrc = audiotestsrc/libspa-audiotestsrc
  support.* = support/libspa-support
}
context.modules = [
  { name = libpipewire-module-protocol-native }
  { name = libpipewire-module-spa-node-factory }
  { name = libpipewire-module-client-node }
  { name = libpipewire-module-adapter }
  { name = libpipewire-module-link-factory }
  { name = libpipewire-module-metadata }
  { name = libpipewire-module-access args = { access.socket = { pokelinux-audio-test = "unrestricted" pokelinux-audio-test-manager = "unrestricted" } } }
]
context.objects = [\n''' + '\n'.join(objects) + '\n]\n')
profile = config / 'profile.json'
profile.write_text(json.dumps({'sources': {'media': 'source.media', 'call': 'source.call', 'capture': 'source.microphone'},
                               'routes': [
                                   {'id':'media-z2','label':'Headphones (virtual)','target':'route.earpiece','roles':['media']},
                                   {'id':'call-only','label':'Call (virtual)','target':'route.earpiece','roles':['call']},
                                   {'id':'media-a7','label':'Speaker (virtual)','target':'route.speaker','roles':['media']},
                                   {'id':'capture-only','label':'Capture (virtual)','target':'route.capture','roles':['capture']}] }))
env = dict(os.environ, XDG_RUNTIME_DIR=str(private), PIPEWIRE_RUNTIME_DIR=str(private),
           PIPEWIRE_REMOTE='pokelinux-audio-test-manager', XDG_CONFIG_HOME=str(config),
           XDG_STATE_HOME=str(state), DISABLE_RTKIT='1', PIPEWIRE_DEBUG='3')
children = {}
logs = []
race_pids = set()
race_results = {}

def start(name, command, input_file=None):
    log = open('/results/' + name + '.log', 'ab', buffering=0)
    logs.append(log)
    children[name] = subprocess.Popen(command, env=env, stdin=input_file, stdout=log, stderr=subprocess.STDOUT)
    return children[name]

def stop(name):
    child = children.get(name)
    if child is None:
        return
    if child.poll() is None:
        child.terminate()
        try:
            child.wait(timeout=5)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=5)

try:
    bus = subprocess.Popen(['dbus-daemon', '--session', '--nofork', '--print-address=1'],
                           stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    children['bus'] = bus
    env['DBUS_SESSION_BUS_ADDRESS'] = bus.stdout.readline().strip()
    start('pipewire', ['pipewire', '-c', str(pw_config)])
    deadline = time.monotonic() + 10
    while not (private / 'pokelinux-audio-test').exists():
        if children['pipewire'].poll() is not None or time.monotonic() >= deadline:
            raise RuntimeError('PipeWire did not start')
        time.sleep(0.03)
    # Finite authored PCM drives the call node through the installed pw-cat API.
    assert sys.byteorder == 'little'
    tone = array.array('f', (0.25 * math.sin(2 * math.pi * 880 * i / 48000) for i in range(45 * 48000)))
    tone_path = pathlib.Path('/results/call-tone.f32le')
    tone_path.write_bytes(tone.tobytes())
    tone_args = ['pw-cat', '--playback', '--raw', '--format', 'f32', '--rate', '48000', '--channels', '1', '--channel-map', 'MONO', '--target', '0', '-P', '{"node.name":"source.call","node.autoconnect":false,"node.dont-reconnect":true}', '-']
    tone_input = open(tone_path, 'rb')
    logs.append(tone_input)
    tone_child = start('tone-call', tone_args, tone_input)
    pathlib.Path('/results/tone-source.json').write_text(json.dumps({'frequency':880,'amplitude':0.25,'frames':len(tone),'format':'F32LE','rate':48000,'channels':1,'path':str(tone_path),'sha256':hashlib.sha256(tone_path.read_bytes()).hexdigest(),'actual_child_pid':tone_child.pid,'argv':tone_args}, indent=2))
    setup_calls = []
    def setup_tool(name, argv):
        child = subprocess.Popen(argv, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        children[name] = child
        try:
            out, err = child.communicate(timeout=2)
        except subprocess.TimeoutExpired:
            child.kill(); out, err = child.communicate(timeout=2)
            raise RuntimeError('bounded stream setup timeout')
        pathlib.Path('/results/' + name + '.stdout').write_bytes(out)
        pathlib.Path('/results/' + name + '.stderr').write_bytes(err)
        setup_calls.append({'argv':argv,'pid':child.pid,'actual_wait_exit':child.returncode,'stdout_EOF':True,'stderr_EOF':True})
        assert child.returncode == 0
        return out
    stream_until = time.monotonic() + 2
    stream_seq = 0
    while True:
        stream_seq += 1
        stream_graph = json.loads(setup_tool('call-port-graph-%02d' % stream_seq, ['pw-dump']))
        streams = [x for x in stream_graph if x['type'].endswith(':Node') and x.get('info', {}).get('props', {}).get('node.name') == 'source.call']
        if len(streams) == 1:
            break
        assert time.monotonic() < stream_until
        time.sleep(.03)
    setup_tool('call-port-format', ['pw-cli', 'set-param', str(streams[0]['id']), 'PortConfig', '{ direction = Output mode = dsp monitor = false format = { mediaType = audio mediaSubtype = raw format = F32 rate = 48000 channels = 1 position = [ MONO ] } }'])
    pathlib.Path('/results/call-stream-setup.json').write_text(json.dumps(setup_calls, indent=2))
    start('wireplumber', ['wireplumber', '--profile', 'native-audio-test'])
    start('authority', ['/target/debug/pokelinux-audio', str(control / 'audio.sock'), str(profile), '1001', '1002'])
    deadline = time.monotonic() + 10
    while not (control / 'audio.sock').exists():
        if children['authority'].poll() is not None or time.monotonic() >= deadline:
            raise RuntimeError('audio authority did not start')
        time.sleep(0.03)
    print(json.dumps({'ready': True, 'pid': os.getpid(), 'children': {name: child.pid for name, child in children.items()}}), flush=True)
    for line in sys.stdin:
        request = json.loads(line)
        op = request['op']
        if op == 'pause_pipewire':
            children['pipewire'].send_signal(signal.SIGSTOP)
        elif op == 'resume_pipewire':
            children['pipewire'].send_signal(signal.SIGCONT)
        elif op == 'pause_backend':
            parent = children['authority'].pid
            candidates = []
            for text_pid in pathlib.Path('/proc/%d/task/%d/children' % (parent, parent)).read_text().split():
                pid = int(text_pid)
                try:
                    command = pathlib.Path('/proc/%d/cmdline' % pid).read_bytes().split(b'\0')
                    if b'--backend-worker' in command:
                        candidates.append(pid)
                except FileNotFoundError:
                    pass
            assert len(candidates) == 1, 'expected one owned backend process'
            os.kill(candidates[0], signal.SIGSTOP)
            print(json.dumps({'op': op, 'backend_pid': candidates[0]}), flush=True)
            continue
        elif op == 'stop_pipewire':
            stop('pipewire')
        elif op == 'restart_pipewire':
            start('pipewire', ['pipewire', '-c', str(pw_config)])
        elif op == 'register_race':
            race_pids.add(int(request['pid']))
        elif op == 'wait_race':
            pid = int(request['pid'])
            assert pid in race_pids
            deadline = time.monotonic() + 3
            while True:
                reaped, status = os.waitpid(pid, os.WNOHANG)
                if reaped:
                    race_results[pid] = os.waitstatus_to_exitcode(status)
                    race_pids.remove(pid)
                    break
                if time.monotonic() > deadline:
                    raise RuntimeError('owned parent-death race worker did not exit')
                time.sleep(0.01)
        elif op == 'kill_authority':
            children['authority'].kill()
            children['authority'].wait(timeout=5)
        elif op == 'stop_wireplumber':
            stop('wireplumber')
        elif op == 'stop':
            print(json.dumps({'stopping': True}), flush=True)
            break
        else:
            raise ValueError('unsupported fixture command')
        print(json.dumps({'op': op, 'children': {name: child.poll() for name, child in children.items()}, 'race_results': race_results}), flush=True)
finally:
    for name in reversed(children):
        stop(name)
    orphan_results = {}
    deadline = time.monotonic() + 3
    while True:
        try:
            pid, status = os.waitpid(-1, os.WNOHANG)
        except ChildProcessError:
            break
        if pid:
            orphan_results[pid] = os.waitstatus_to_exitcode(status)
        elif time.monotonic() < deadline:
            time.sleep(0.01)
        else:
            raise RuntimeError('owned orphan worker remains alive')
    for log in logs:
        log.close()
    pathlib.Path('/results/fixture-cleanup.json').write_text(json.dumps({'children_reaped': {name: child.poll() for name, child in children.items()}, 'host_audio': False, 'host_dbus': False, 'hardware': False, 'orphan_workers_reaped': orphan_results, 'parent_race': race_results}))
