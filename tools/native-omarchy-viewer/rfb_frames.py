"""One fixed RFB3.8/raw receipt; no viewer UI, input injection or proxy."""
import hashlib
import struct


def receive_once(wire):
    banner = wire.read(12)
    assert banner == b'RFB 003.008\n', 'unexpected RFB version'
    wire.write(banner)
    count = wire.read(1)[0]
    assert 0 < count <= 16, 'missing/bounded security types'
    types = wire.read(count)
    assert 1 in types, 'local None security type unavailable'
    wire.write(b'\x01')
    assert struct.unpack('>I', wire.read(4))[0] == 0, 'security negotiation failed'
    wire.write(b'\x00')  # Exactly one exclusive client on the private socket.
    initial = wire.read(24)
    width, height = struct.unpack('>HH', initial[:4])
    assert (width, height) == (1080, 2424), 'unexpected Xvfb canvas'
    name_bytes = struct.unpack('>I', initial[20:])[0]
    assert name_bytes <= 4096, 'bounded server name'
    name_hash = hashlib.sha256(wire.read(name_bytes)).hexdigest()
    pixel_format = struct.pack('>BBBBHHHBBB3x', 32, 24, 0, 1, 255, 255, 255, 16, 8, 0)
    wire.write(b'\x00\x00\x00\x00' + pixel_format)
    wire.write(struct.pack('>BBHi', 2, 0, 1, 0))  # Raw is the sole encoding.
    wire.write(struct.pack('>BBHHHH', 3, 0, 0, 0, width, height))
    return receive_raw_update(wire, width, height, name_hash)


def request_raw_frame(wire, width, height, name_hash):
    wire.write(struct.pack('>BBHHHH', 3, 0, 0, 0, width, height))
    return receive_raw_update(wire, width, height, name_hash)


def receive_raw_update(wire, width, height, name_hash):
    frame = bytearray(width * height * 4)
    covered = bytearray(width * height)
    bells = 0
    while True:
        kind = wire.read(1)[0]
        if kind == 2:
            bells += 1
            assert bells <= 8, 'bounded bell notifications'
            continue
        assert kind == 0, 'unexpected server message'
        padding, count = struct.unpack('>BH', wire.read(3))
        # RFC 6143 section 7: recipients must not assume a padding value.
        assert 0 < count <= 256, f'bounded rectangle count: padding={padding}, rectangles={count}'
        rectangles = []
        raw_bytes = 0
        for _ in range(count):
            x, y, w, h, encoding = struct.unpack('>HHHHi', wire.read(12))
            assert encoding == 0 and w > 0 and h > 0, 'expected raw rectangle'
            assert x + w <= width and y + h <= height, 'rectangle exceeds canvas'
            size = w * h * 4
            raw_bytes += size
            assert raw_bytes <= 12 * 1024 * 1024, 'bounded raw pixels'
            pixels = wire.read(size)
            stride = w * 4
            for row in range(h):
                start = ((y + row) * width + x) * 4
                frame[start:start + stride] = pixels[row * stride:(row + 1) * stride]
                c = (y + row) * width + x
                covered[c:c + w] = b'\x01' * w
            rectangles.append({'x': x, 'y': y, 'width': w, 'height': h})
        assert all(covered), 'nonincremental update did not cover canvas'
        return bytes(frame), {
            'RFB_version': '3.8', 'security_type': 1, 'exclusive_client': True,
            'server_name_sha256': name_hash, 'width': width, 'height': height,
            'requested_pixel_format': 'little-endian32/depth24/R16G8B0',
            'sole_encoding': 'raw', 'rectangles': rectangles, 'bell_count': bells,
            'framebuffer_update_padding_byte': padding,
            'framebuffer_update_rectangle_count': count,
            'raw_bytes_received': raw_bytes, 'frame_bytes': len(frame),
            'frame_sha256': hashlib.sha256(frame).hexdigest(),
            'frame_receipt_function_sends_input': False,
            'meaningful_guest_picture_required': False,
            'pixel_profile_or_input_qualified_by_frame_alone': False,
        }
