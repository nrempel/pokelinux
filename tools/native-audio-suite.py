#!/usr/bin/env python3
"""Exercise real daemons in one owned, network-none, unprivileged container."""
import argparse
import json
import os
import pathlib
import select
import subprocess
import time

CLIENT = '''import json,socket,sys
s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
s.connect('/tmp/pokelinux-native-audio/control/audio.sock')
f=s.makefile('rwb',buffering=0)
for line in sys.stdin.buffer:
 f.write(line)
 answer=f.readline()
 if not answer: raise RuntimeError('authority disconnected')
 sys.stdout.buffer.write(answer);sys.stdout.buffer.flush()
'''


def read_line(process, seconds=12):
    if not select.select([process.stdout], [], [], seconds)[0]:
        raise TimeoutError('owned process response deadline expired')
    line = process.stdout.readline()
    if not line:
        raise RuntimeError('owned process ended unexpectedly')
    return json.loads(line)


class Client:
    def __init__(self, container, uid):
        self.process = subprocess.Popen(['docker', 'exec', '-i', '--user', str(uid), container,
                                         'python3', '-u', '-c', CLIENT], stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

    def request(self, request):
        self.process.stdin.write(json.dumps(request) + '\n')
        self.process.stdin.flush()
        return read_line(self.process)

    def close(self):
        if self.process.poll() is None:
            self.process.stdin.close()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)


def run(output, stop_only=False):
    repo = pathlib.Path(__file__).resolve().parent.parent
    output.mkdir(parents=True, exist_ok=False)
    name = 'pokelinux-native-audio-test-' + str(os.getpid())
    clients = []
    stages = []
    snapshots = 0
    fixture_log = open(output / 'fixture-stderr.log', 'w')
    command = ['docker', 'run', '--rm', '-i', '--name', name, '--network', 'none',
               '--read-only', '--cpus', '2', '--memory', '1g', '--pids-limit', '128',
               '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges', '--user', '1000:1000',
               '--tmpfs', '/tmp:rw,exec,nosuid,nodev,size=64m,mode=1777',
               '--mount', 'type=bind,src=' + str(repo / 'tools/native-audio-fixture.py') + ',dst=/fixture.py,readonly',
               '--mount', 'type=bind,src=' + str(repo / 'artifacts/native-audio/target') + ',dst=/target,readonly',
               '--mount', 'type=bind,src=' + str(output) + ',dst=/results',
               'pokelinux-native-audio:20261012', 'python3', '-u', '/fixture.py']
    fixture = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=fixture_log, text=True)

    def fixture_command(op, **fields):
        fixture.stdin.write(json.dumps(dict(op=op, **fields)) + '\n')
        fixture.stdin.flush()
        return read_line(fixture)

    def client(uid):
        c = Client(name, uid)
        clients.append(c)
        return c

    def graph():
        nonlocal snapshots
        result = subprocess.run(['docker', 'exec', '--user', '1000', name, 'env',
                                 'PIPEWIRE_RUNTIME_DIR=/tmp/pokelinux-native-audio/private',
                                 'PIPEWIRE_REMOTE=pokelinux-audio-test-manager', 'pw-dump'],
                                check=True, capture_output=True, text=True, timeout=5)
        objects = json.loads(result.stdout)
        (output / ('graph-%03d.json' % snapshots)).write_text(result.stdout)
        snapshots += 1
        nodes = {x['id']: x.get('info', {}).get('props', {}).get('node.name')
                 for x in objects if x['type'].endswith(':Node')}
        links = {(nodes.get(x['info']['output-node-id']), nodes.get(x['info']['input-node-id']))
                 for x in objects if x['type'].endswith(':Link') and x.get('info', {}).get('state') == 'active'}
        return objects, links

    def expect_graph(expected):
        deadline = time.monotonic() + 5
        while True:
            objects, actual = graph()
            states = {x.get('info', {}).get('props', {}).get('node.name'): x.get('info', {}).get('state')
                      for x in objects if x['type'].endswith(':Node')}
            if actual == expected and all(states.get(node) == 'running' for edge in expected for node in edge):
                return objects
            if time.monotonic() > deadline:
                raise AssertionError({'expected_graph': sorted(expected), 'actual_graph': sorted(actual)})
            time.sleep(0.05)

    def accepted(response):
        assert response.get('ok'), response
        return response

    result = {'passed': False, 'scope': 'isolated native Rust authority and original PipeWire/WirePlumber virtual graph',
              'hardware_verified': False, 'vendor_executed': False, 'network': 'none',
              'uids': {'audio_owner': 1000, 'call_owner': 1001, 'app': 1002}, 'stages': stages}
    try:
        ready = read_line(fixture)
        assert ready['ready'], ready
        (output / 'fixture-ready.json').write_text(json.dumps(ready, indent=2))
        objects = expect_graph(set())
        assert len([x for x in objects if x['type'].endswith(':Node')]) >= 7
        assert any(x.get('info', {}).get('props', {}).get('application.process.binary') == 'wireplumber'
                   for x in objects if x['type'].endswith(':Client'))
        stages.append('original_daemons_and_virtual_nodes')
        if stop_only:
            app = client(1002)
            call = client(1001)
            accepted(app.request({'op': 'acquire', 'role': 'media', 'route': 'route.speaker'}))
            expect_graph({('source.media', 'route.speaker')})
            denied = app.request({'op': 'stop'})
            assert not denied.get('ok'), denied
            expect_graph({('source.media', 'route.speaker')})
            fixture_command('pause_pipewire')
            try:
                time.sleep(0.35)
                started = time.monotonic()
                stopped = accepted(call.request({'op': 'stop'}))
                elapsed = time.monotonic() - started
                assert elapsed < 0.35, ('stop waited on actual PipeWire', elapsed, stopped)
                assert stopped['desired'] == [] and not stopped['observed']['available'], stopped
                assert stopped['phase'] == 'stopping' and stopped['cleanup'] == 'pending', stopped
                result['stalled_backend_stop_seconds'] = elapsed
            finally:
                fixture_command('resume_pipewire')
            expect_graph(set())
            stages.append('authenticated_stop_reclaims_stalled_backend_without_detach')
            result['passed'] = True
            return
        # Own and register a worker before its actual parent exits, then execute
        # the guard after that death. This exercises the fork-before-prctl race.
        race_child = """import os,sys,time
parent=int(sys.argv[1]);deadline=time.monotonic()+2
while os.getppid()==parent:
 if time.monotonic()>deadline: raise RuntimeError('race parent did not exit')
 time.sleep(0.01)
os.execv(sys.argv[2],sys.argv[2:])
"""
        race_parent = """import json,os,subprocess,sys
log=open('/results/parent-race-worker.log','wb')
command=['/target/debug/pokelinux-audio','--tool-worker',str(os.getpid()),'/usr/bin/pw-link','-m','source.media:capture_MONO','route.speaker:playback_MONO']
child=subprocess.Popen(['python3','-c',sys.argv[1],str(os.getpid()),*command],stdin=subprocess.DEVNULL,stdout=log,stderr=log)
print(json.dumps({'child':child.pid,'parent':os.getpid()}),flush=True)
sys.stdin.readline()
"""
        race = subprocess.Popen(['docker','exec','-i','--user','1000',name,'python3','-u','-c',race_parent,race_child],
                                stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        race_ready = read_line(race)
        fixture_command('register_race',pid=race_ready['child'])
        race.stdin.close()
        assert race.wait(timeout=5) == 0
        race_result = fixture_command('wait_race',pid=race_ready['child'])
        assert race_result['race_results'][str(race_ready['child'])] != 0, race_result
        assert 'parent already died' in (output/'parent-race-worker.log').read_text()
        expect_graph(set())
        stages.append('real_parent_exits_before_worker_guard')

        # Exhaust application connections before opening the privileged caller.
        # Kernel credentials must preserve call capacity, not client role labels.
        occupied = []
        for _ in range(32):
            candidate = client(1002)
            try:
                accepted(candidate.request({'op': 'status'}))
            except RuntimeError:
                candidate.close()
                break
            occupied.append(candidate)
        assert occupied
        reserved_call = client(1001)
        privileged = accepted(reserved_call.request({'op': 'acquire', 'role': 'call', 'route': 'route.earpiece'}))['lease']
        expect_graph({('source.call', 'route.earpiece')})
        accepted(reserved_call.request({'op': 'release', 'lease': privileged}))
        reserved_call.close()
        for candidate in occupied:
            candidate.close()
        expect_graph(set())
        stages.append('application_connection_exhaustion_preserves_call_admission')

        budget_app = client(1002)
        admitted = 0
        for _ in range(65):
            response = budget_app.request({'op': 'acquire', 'role': 'media', 'route': 'route.speaker'})
            if not response.get('ok'):
                assert response.get('error') == 'lease_limit', response
                break
            admitted += 1
        assert 0 < admitted < 65
        budget_call = client(1001)
        privileged = accepted(budget_call.request({'op': 'acquire', 'role': 'call', 'route': 'route.earpiece'}))['lease']
        expect_graph({('source.call', 'route.earpiece')})
        accepted(budget_call.request({'op': 'release', 'lease': privileged}))
        expect_graph({('source.media', 'route.speaker')})
        budget_call.close()
        budget_app.close()
        expect_graph(set())
        stages.append('application_lease_exhaustion_preserves_call_admission')

        observer = client(1002)
        accepted(observer.request({'op': 'status'}))
        fixture_command('pause_pipewire')
        try:
            # The real daemon stays stopped while an actual pw-dump read is pending.
            time.sleep(0.35)
            started = time.monotonic()
            status = accepted(observer.request({'op': 'status'}))
            elapsed = time.monotonic() - started
            assert elapsed < 0.35, ('status blocked behind actual PipeWire read', elapsed, status)
            result['stalled_backend_status_seconds'] = elapsed
        finally:
            fixture_command('resume_pipewire')
            observer.close()
        stages.append('status_responsive_during_actual_daemon_stall')

        app = client(1002)
        call = client(1001)
        media = accepted(app.request({'op': 'acquire', 'role': 'media', 'route': 'route.speaker'}))['lease']
        expect_graph({('source.media', 'route.speaker')})
        stages.append('media_link_active')
        spoof = client(1002)
        denied = spoof.request({'op': 'acquire', 'role': 'call', 'media_role': 'Phone', 'uid': 1001, 'route': 'route.earpiece'})
        assert denied.get('error') == 'denied', denied
        expect_graph({('source.media', 'route.speaker')})
        # This client cannot bypass the service through the private graph socket.
        bypass = subprocess.run(['docker', 'exec', '--user', '1002', name, 'env',
                                 'PIPEWIRE_RUNTIME_DIR=/tmp/pokelinux-native-audio/private',
                                 'PIPEWIRE_REMOTE=pokelinux-audio-test-manager', 'pw-dump'], capture_output=True, timeout=5)
        assert bypass.returncode != 0
        stages.append('kernel_peer_auth_and_private_graph_access')
        lease = accepted(call.request({'op': 'acquire', 'role': 'call', 'route': 'route.earpiece'}))['lease']
        expect_graph({('source.call', 'route.earpiece')})
        stages.append('call_interrupts_media')
        assert app.request({'op': 'release', 'lease': lease}).get('error') == 'denied'
        accepted(call.request({'op': 'route', 'lease': lease, 'route': 'route.speaker'}))
        expect_graph({('source.call', 'route.speaker')})
        accepted(call.request({'op': 'release', 'lease': lease}))
        expect_graph({('source.media', 'route.speaker')})
        stages.append('owned_route_and_media_restoration')
        paused = fixture_command('pause_backend')
        deadline = time.monotonic() + 6
        while True:
            started = time.monotonic()
            recovered = accepted(app.request({'op': 'status'}))
            assert time.monotonic() - started < 0.35, recovered
            if recovered['application'] == 'uncertain':
                break
            assert time.monotonic() < deadline, recovered
            time.sleep(0.05)
        assert recovered['desired'] and not recovered['observed']['available'], recovered
        assert app.request({'op': 'acquire', 'role': 'media', 'route': 'route.speaker'}).get('error') == 'explicit_recovery_required'
        expect_graph(set())
        # Kernel reaping is a separate local observation, not remote-device cleanup.
        probe = subprocess.run(['docker', 'exec', '--user', '1000', name, 'python3', '-c',
                                'import os,sys;sys.exit(1 if os.path.exists("/proc/"+sys.argv[1]) else 0)',
                                str(paused['backend_pid'])], timeout=3)
        assert probe.returncode == 0, 'retired backend not reaped'
        accepted(call.request({'op': 'reconcile'}))
        expect_graph({('source.media', 'route.speaker')})
        stages.append('permanent_backend_stall_reaped_without_implicit_recovery')
        accepted(call.request({'op': 'grant_capture'}))
        accepted(app.request({'op': 'acquire', 'role': 'capture', 'route': 'route.capture'}))
        expect_graph({('source.media', 'route.speaker'), ('source.microphone', 'route.capture')})
        assert app.request({'op': 'revoke_capture'}).get('error') == 'denied'
        accepted(call.request({'op': 'revoke_capture'}))
        expect_graph({('source.media', 'route.speaker')})
        assert app.request({'op': 'acquire', 'role': 'capture', 'route': 'route.capture'}).get('error') == 'capture_revoked'
        stages.append('capture_revocation_unlinks_real_graph')
        other_app = client(1002)
        assert other_app.request({'op': 'release', 'lease': media}).get('error') == 'denied'
        app.close()
        expect_graph(set())
        stages.append('connection_death_releases_owned_links')
        accepted(other_app.request({'op': 'acquire', 'role': 'media', 'route': 'route.speaker'}))
        expect_graph({('source.media', 'route.speaker')})
        before = accepted(other_app.request({'op': 'status'}))
        fixture_command('stop_pipewire')
        deadline = time.monotonic() + 8
        while True:
            status = accepted(other_app.request({'op': 'status'}))
            if not status['observed']['available']:
                break
            assert time.monotonic() < deadline, status
            time.sleep(0.05)
        assert status['desired'] and not status['observed']['links'], status
        assert status['observed']['epoch'] > before['observed']['epoch']
        stages.append('daemon_death_preserves_intent_and_invalidates_observation')
        fixture_command('restart_pipewire')
        deadline = time.monotonic() + 8
        while True:
            status = accepted(other_app.request({'op': 'status'}))
            if status['observed']['available']:
                break
            assert time.monotonic() < deadline, status
            time.sleep(0.05)
        expect_graph(set())
        spoof.close()
        expect_graph(set())
        stages.append('unrelated_disconnect_does_not_reapply_uncertain_intent')
        accepted(call.request({'op': 'reconcile'}))
        expect_graph({('source.media', 'route.speaker')})
        stages.append('explicit_reconciliation_after_daemon_restart')
        other_app.close()
        expect_graph(set())
        accepted(call.request({'op': 'grant_capture'}))
        capture_app = client(1002)
        accepted(capture_app.request({'op': 'acquire', 'role': 'capture', 'route': 'route.capture'}))
        expect_graph({('source.microphone', 'route.capture')})
        fixture_command('kill_authority')
        expect_graph(set())
        stages.append('authority_kill_removes_capture_links')
        result['passed'] = True
    finally:
        for c in clients:
            c.close()
        if fixture.poll() is None:
            try:
                fixture_command('stop')
                fixture.stdin.close()
                fixture.wait(timeout=10)
            except Exception:
                subprocess.run(['docker', 'stop', '--time', '3', name], capture_output=True, timeout=8)
                fixture.wait(timeout=8)
        fixture_log.close()
        result['fixture_exit'] = fixture.returncode
        remaining = subprocess.run(['docker', 'ps', '-a', '--filter', 'name=' + name, '--format', '{{.Names}}'],
                                   capture_output=True, text=True, check=True).stdout.splitlines()
        result['owned_container_absent'] = name not in remaining
        result['graph_snapshots'] = snapshots
        (output / 'result.json').write_text(json.dumps(result, indent=2))
        print(json.dumps(result, indent=2), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('--stop-only', action='store_true')
    args = parser.parse_args()
    run(args.output.resolve(), args.stop_only)
