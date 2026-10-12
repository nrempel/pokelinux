#!/usr/bin/env python3
"""Own an isolated original PipeWire/WirePlumber stack and Rust authority."""
import json
import os
import pathlib
import subprocess
import sys
import time

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
    ('source.call', 'audiotestsrc', 'Audio/Source'),
    ('source.microphone', 'audiotestsrc', 'Audio/Source'),
    ('route.speaker', 'support.null-audio-sink', 'Audio/Sink'),
    ('route.earpiece', 'support.null-audio-sink', 'Audio/Sink'),
    ('route.capture', 'support.null-audio-sink', 'Audio/Sink'),
]:
    extra = '' if factory != 'audiotestsrc' else 'node.param.PortConfig = { direction = Output mode = dsp format = { mediaType = audio mediaSubtype = raw format = F32 rate = 48000 channels = 1 position = [ MONO ] } }'
    objects.append('''{ factory = adapter args = {
        factory.name = "%s" node.name = "%s" media.class = "%s"
        audio.format = F32 audio.rate = 48000 audio.channels = 1 audio.position = [ MONO ]
        adapter.auto-port-config = { mode = dsp monitor = false position = preserve }
        %s
    } }''' % (factory, name, media_class, extra))
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
                               'routes': {'route.speaker': ['media', 'call'], 'route.earpiece': ['media', 'call'], 'route.capture': ['capture']}}))
env = dict(os.environ, XDG_RUNTIME_DIR=str(private), PIPEWIRE_RUNTIME_DIR=str(private),
           PIPEWIRE_REMOTE='pokelinux-audio-test-manager', XDG_CONFIG_HOME=str(config),
           XDG_STATE_HOME=str(state), DISABLE_RTKIT='1', PIPEWIRE_DEBUG='3')
children = {}
logs = []
race_pids = set()
race_results = {}

def start(name, command):
    log = open('/results/' + name + '.log', 'ab', buffering=0)
    logs.append(log)
    children[name] = subprocess.Popen(command, env=env, stdout=log, stderr=subprocess.STDOUT)
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
        if op == 'stop_pipewire':
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
