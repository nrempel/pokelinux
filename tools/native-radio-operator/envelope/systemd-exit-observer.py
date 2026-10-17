#!/usr/bin/env python3
"""Private fixture receipt of systemd's actual Main outcome; no independent reap."""
import json
import os
from pathlib import Path
import re
import sys
import time

path = Path(sys.argv[1])
rows = json.loads(path.read_text()) if path.exists() else []
assert isinstance(rows, list) and len(rows) < 4
invocation = os.environ.get('INVOCATION_ID', '')
assert re.fullmatch('[0-9a-f]{32}', invocation)
rows.append({'INVOCATION_ID': invocation,
    **{key: os.environ.get(key) for key in ['EXIT_CODE', 'EXIT_STATUS', 'SERVICE_RESULT']},
    'observer_pid': os.getpid(), 'observer_ppid': os.getppid(),
    'observer_uid': os.getuid(), 'observer_gid': os.getgid(),
    'observed_boottime_ns': time.clock_gettime_ns(time.CLOCK_BOOTTIME),
    'authority': 'systemd ExecStopPost Main outcome environment; observer performs no waitpid'})
fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_CLOEXEC, 0o600)
with os.fdopen(fd, 'w') as output:
    json.dump(rows, output, indent=2)
    output.write('\n')
    output.flush()
    os.fsync(output.fileno())
