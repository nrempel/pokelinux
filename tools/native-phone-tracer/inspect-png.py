#!/usr/bin/env python3
"""Decode an unchanged RGB/RGBA8 PNG for independent capture review; never edit it."""
import argparse
from collections import Counter
import hashlib
from pathlib import Path
import struct
import zlib


def decode(path):
    data = Path(path).read_bytes()
    assert 0 < len(data) <= 16 * 1024 * 1024 and data[:8] == b'\x89PNG\r\n\x1a\n'
    position, chunks, compressed, ended = 8, [], bytearray(), False
    width = height = channels = None
    while position < len(data):
        assert position + 12 <= len(data) and len(chunks) < 256
        length, kind = struct.unpack('!I4s', data[position:position + 8])
        end = position + 12 + length
        assert end <= len(data)
        body = data[position + 8:position + 8 + length]
        assert zlib.crc32(kind + body) & 0xffffffff == struct.unpack('!I', data[end - 4:end])[0]
        if not chunks:
            assert kind == b'IHDR' and length == 13
            width, height, bits, color, compression, filtering, interlace = struct.unpack('!IIBBBBB', body)
            assert (width, height) == (1080, 2424), 'selected native capture dimensions required'
            assert bits == 8 and color in (2, 6) and (compression, filtering, interlace) == (0, 0, 0)
            channels = 3 if color == 2 else 4
        elif kind == b'IHDR':
            raise ValueError('duplicate PNG header')
        elif kind == b'IDAT':
            assert not ended
            compressed.extend(body)
        elif kind == b'IEND':
            assert length == 0 and compressed and end == len(data)
            ended = True
        else:
            assert kind[0] & 32, 'unsupported critical PNG chunk'
        chunks.append(kind.decode('ascii'))
        position = end
    assert ended and width and height and channels
    stride = width * channels
    decoder = zlib.decompressobj()
    raw = decoder.decompress(bytes(compressed), (stride + 1) * height + 1)
    assert decoder.eof and not decoder.unused_data and not decoder.unconsumed_tail
    assert len(raw) == (stride + 1) * height
    previous, rgb, nonopaque = bytearray(stride), bytearray(), 0
    for row in range(height):
        start = row * (stride + 1)
        method = raw[start]
        assert method <= 4
        current = bytearray(raw[start + 1:start + 1 + stride])
        if method:
            for index in range(stride):
                left = current[index - channels] if index >= channels else 0
                above = previous[index]
                upper_left = previous[index - channels] if index >= channels else 0
                if method == 1:
                    predict = left
                elif method == 2:
                    predict = above
                elif method == 3:
                    predict = (left + above) // 2
                else:
                    base = left + above - upper_left
                    distances = (abs(base - left), abs(base - above), abs(base - upper_left))
                    predict = (left, above, upper_left)[distances.index(min(distances))]
                current[index] = (current[index] + predict) & 255
        if channels == 3:
            rgb.extend(current)
        else:
            for index in range(0, stride, 4):
                rgb.extend(current[index:index + 3])
                nonopaque += current[index + 3] != 255
        previous = current
    return data, bytes(rgb), nonopaque, chunks


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('png', type=Path)
    p.add_argument('--expect-color', action='append', default=[])
    a = p.parse_args()
    data, rgb, nonopaque, chunks = decode(a.png)
    colors = Counter(rgb[index:index + 3] for index in range(0, len(rgb), 3))
    expected = {}
    for color in a.expect_color:
        assert len(color) == 7 and color.startswith('#')
        value = bytes.fromhex(color[1:])
        assert len(value) == 3
        expected[color] = colors[value]
        assert colors[value] > 0, 'expected rendered color absent: ' + color
    import json
    print(json.dumps({'png_sha256': hashlib.sha256(data).hexdigest(),
        'decoded_rgb_sha256': hashlib.sha256(rgb).hexdigest(),
        'pixels': {'width': 1080, 'height': 2424}, 'complete_png_decode': True,
        'nonopaque_pixels': nonopaque, 'expected_color_counts': expected,
        'dominant_colors': [{'color': '#' + color.hex(), 'pixels': count}
                            for color, count in colors.most_common(10)],
        'chunks': chunks, 'image_edited_or_resized': False,
        'literal_text_or_app_consumption_qualified': False}, indent=2))


if __name__ == '__main__':
    main()
