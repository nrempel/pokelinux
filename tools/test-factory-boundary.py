#!/usr/bin/env python3
"""Exercise installed stock EGL up to its observed missing-GPU boundary."""
import json
import subprocess
from pathlib import Path
from sim import validate_factory_boundary

project = Path(__file__).resolve().parents[1]
logs = project / 'artifacts/logs'
logs.mkdir(exist_ok=True)
runtime = project / 'artifacts/runtime'
command = ['sh', str(project / 'tools/with-runtime.sh'), 'qemu-aarch64', '-cpu', 'max',
           '-L', str(runtime), '-strace', str(project / 'artifacts/linux-aarch64/bin/runtime-probe'),
           str(runtime), '--egl', 'initialize']
completed = subprocess.run(command, text=True, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=60)
(logs / 'factory-egl.stdout').write_text(completed.stdout)
(logs / 'factory-egl.syscalls').write_text(completed.stderr)
validate_factory_boundary(completed.returncode, completed.stdout, completed.stderr)
report = {'status': 'expected_missing_gpu_boundary', 'exit_code': completed.returncode,
          'factory_config_opened': True, 'render_nodes_absent': True,
          'hardware_verified': False, 'rendering_verified': False}
(logs / 'factory-boundary.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
