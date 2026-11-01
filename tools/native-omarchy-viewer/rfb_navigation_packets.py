"""Three fixed genuine-menu key plans; no socket, retry or input injection."""
import struct


def navigation_plan(action):
    keys = {'page-down': 0xff56, 'down': 0xff54, 'return': 0xff0d}
    assert action in keys
    packets = tuple(struct.pack('>BBHI', 4, down, 0, keys[action]) for down in (1, 0))
    return {'action': action, 'packets': packets, 'hold_after_packet_index': 0,
            'hold_ns': 100_000_000, 'guest_uinput': False}
