#!/usr/bin/env python3
"""Build a conservative static graph; never claim linker namespace resolution."""
import json
from pathlib import Path
from hardware_audit import native_graph
root=Path('artifacts/telephony-audit')
rows=json.loads((root/'native-index.json').read_text())
roots=['/vendor/bin/hw/rild_exynos','/vendor/bin/cbd','/vendor/bin/rfsd','/vendor/lib64/libsitril.so','/system_ext/lib64/libpixelimsmedia.so','/vendor/bin/hw/android.hardware.audio.service-aidl.aoc','/vendor/bin/hw/android.hardware.power-service.pixel-libperfmgr','/apex/com.google.pixel.camera.hal/bin/hw/android.hardware.camera.provider@2.7-service-google']
graph=native_graph(rows,roots)
(root/'results/native-graph.json').write_text(json.dumps(graph,indent=2)+'\n')
print({k:len(graph[k]) for k in ['files','edges','unresolved','ambiguous']})
