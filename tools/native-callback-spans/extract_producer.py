#!/usr/bin/env python3
from pathlib import Path
import hashlib,json,sys
source=Path(sys.argv[1]).read_bytes()
begin=b"// POKELINUX_CALLBACK_SPAN_PRODUCER_BEGIN\n"
end=b"// POKELINUX_CALLBACK_SPAN_PRODUCER_END\n"
assert source.count(begin)==source.count(end) and source.count(begin) in (0,1)
if begin in source:
    first=source.index(begin)+len(begin);last=source.index(end)
    assert first < last
    block=source[first:last]
else:
    block=b""
Path(sys.argv[2]).write_bytes(block)
print(json.dumps({'source_bytes':len(source),'source_sha256':hashlib.sha256(source).hexdigest(),
                 'marked_block_present':bool(block),'extracted_bytes':len(block),'extracted_sha256':hashlib.sha256(block).hexdigest()},sort_keys=True))
