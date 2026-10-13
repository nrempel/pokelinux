#!/usr/bin/env python3
"""Synthetic unit ExecStopPost records only systemd's actual Main exit outcome."""
import json,os,sys
from pathlib import Path
path=Path(sys.argv[1]);rows=json.loads(path.read_text()) if path.exists() else []
assert len(rows)<4
rows.append({key:os.environ.get(key) for key in ['EXIT_CODE','EXIT_STATUS','SERVICE_RESULT']})
path.write_text(json.dumps(rows,indent=2)+'\n')
