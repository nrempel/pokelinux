"""Fixed test packet plans only. No socket, injection, retry, proxy or clock."""
import struct

WIDTH, HEIGHT = 1080, 2424
SUPER_L, W_KEY = 0xffeb, 0x77
ACTIONS = ('bar-1', 'apps-1', 'sounds-1', 'close-1', 'bar-2', 'apps-2', 'sounds-2')


def click_plan(action, x, y, viewport):
    assert action in ACTIONS and action != 'close-1'
    assert type(x) is int and type(y) is int
    assert 0 < x < WIDTH and 0 < y < HEIGHT
    assert viewport == {'x': 0, 'y': 0, 'width': WIDTH, 'height': HEIGHT}, 'no assumed scaling/origin'
    packets = tuple(struct.pack('>BBHH', 5, mask, x, y) for mask in (0, 1, 0))
    return {'action': action, 'packets': packets, 'hold_after_packet_index': 1,
            'hold_ns': 100_000_000, 'guest_uinput': False}


def close_plan(action):
    assert action == 'close-1'
    keys = ((1, SUPER_L), (1, W_KEY), (0, W_KEY), (0, SUPER_L))
    packets = tuple(struct.pack('>BBHI', 4, down, 0, key) for down, key in keys)
    return {'action': action, 'packets': packets, 'hold_after_packet_index': 1,
            'hold_ns': 100_000_000, 'binding': 'SUPER + W', 'guest_uinput': False}
