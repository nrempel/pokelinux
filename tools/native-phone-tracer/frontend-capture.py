#!/usr/bin/env python3
"""Owned SDL/X11 capture or fixed pre-Source host-window origin placement."""
import argparse
import ctypes as C
import hashlib
import json
import os
from pathlib import Path
import re
import select
import signal
import socket
import stat
import struct
import time
import zlib

W, H = 1080, 2424
GPU = 'virtio-gpu-gl-pci,id=phone-display,xres=1080,yres=2424'
DISPLAY_ARGUMENT = 'sdl,gl=on'
U = C.c_ulong
P = C.c_void_p
I = C.c_int


class Attributes(C.Structure):
    _fields_ = [('x', I), ('y', I), ('width', I), ('height', I), ('border_width', I),
                ('depth', I), ('visual', P), ('root', U), ('window_class', I),
                ('bit_gravity', I), ('win_gravity', I), ('backing_store', I),
                ('backing_planes', U), ('backing_pixel', U), ('save_under', I),
                ('colormap', U), ('map_installed', I), ('map_state', I),
                ('all_event_masks', C.c_long), ('your_event_mask', C.c_long),
                ('do_not_propagate_mask', C.c_long), ('override_redirect', I), ('screen', P)]


class XImage(C.Structure):
    _fields_ = [('width', I), ('height', I), ('xoffset', I), ('format', I), ('data', P),
                ('byte_order', I), ('bitmap_unit', I), ('bitmap_bit_order', I),
                ('bitmap_pad', I), ('depth', I), ('bytes_per_line', I), ('bits_per_pixel', I),
                ('red_mask', U), ('green_mask', U), ('blue_mask', U), ('obdata', P),
                ('functions', P * 6)]


class XErrorEvent(C.Structure):
    _fields_ = [('type', I), ('display', P), ('resourceid', U), ('serial', U),
                ('error_code', C.c_ubyte), ('request_code', C.c_ubyte), ('minor_code', C.c_ubyte)]


XErrorHandler = C.CFUNCTYPE(I, P, C.POINTER(XErrorEvent))


def digest(data):
    return hashlib.sha256(data).hexdigest()


def process(pid):
    path = Path('/proc') / str(pid)
    fields = (path / 'stat').read_text().rsplit(')', 1)[1].split()
    raw = (path / 'cmdline').read_bytes()
    assert 0 < len(raw) <= 16384, 'bounded nonempty owned process argv'
    return {'pid': pid, 'ppid': int(fields[1]), 'start_ticks': int(fields[19]),
            'uid': path.stat().st_uid, 'cmdline_sha256': digest(raw),
            'argv': [os.fsdecode(v) for v in raw.split(b'\0') if v]}


def ancestry(pid, ancestor):
    rows = []
    for _ in range(16):
        row = process(pid)
        rows.append(row)
        if pid == ancestor:
            return rows
        pid = row['ppid']
        assert pid > 0, 'owned process did not reach expected ancestor'
    raise AssertionError('bounded process ancestry')


def watch(pid):
    fd = os.pidfd_open(pid, 0)
    poll = select.poll()
    poll.register(fd, select.POLLIN)
    assert poll.poll(0) == [], 'owned process already exited'
    return fd, poll


def alive(row, deathwatch):
    assert deathwatch[1].poll(0) == [], 'owned process death observed'
    assert process(row['pid']) == row, 'owned process identity or argv changed'


def chunk(kind, data):
    return struct.pack('!I', len(data)) + kind + data + struct.pack('!I', zlib.crc32(kind + data) & 0xffffffff)


def png(rgb):
    assert len(rgb) == W * H * 3
    stride = W * 3
    rows = b''.join(b'\0' + rgb[offset:offset + stride] for offset in range(0, len(rgb), stride))
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('!IIBBBBB', W, H, 8, 2, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(rows, 6)) + chunk(b'IEND', b''))


def exclusive(path, data):
    with path.open('xb') as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--container-id', required=True)
    parser.add_argument('--qemu-pid', type=int, required=True, help='PID in the container PID namespace')
    parser.add_argument('--qemu-start-ticks', type=int, required=True)
    parser.add_argument('--qemu-cmdline-sha256', required=True)
    parser.add_argument('--qmp-argument', required=True)
    parser.add_argument('--output-prefix', type=Path, required=True)
    parser.add_argument('--place-only', action='store_true', help='pre-Source fixed host-window origin0,0 placement; no pixels')
    args = parser.parse_args()
    assert re.fullmatch('[0-9a-f]{64}', args.container_id)
    assert re.fullmatch('[0-9a-f]{64}', args.qemu_cmdline_sha256)
    assert args.output_prefix.is_absolute() and args.output_prefix.parent.is_dir()
    assert args.output_prefix.parent.stat().st_uid == os.getuid() == 1000
    assert not args.output_prefix.parent.is_symlink()
    signal.signal(signal.SIGALRM, signal.SIG_DFL)
    signal.alarm(12)  # Kernel termination also bounds a stalled native Xlib call.
    prefix = args.output_prefix
    exclusive(Path(str(prefix) + '.claim'), b'one selected frontend operation; no implicit retry\n')
    result = {'ok': False, 'method': ('owned pre-Source native SDL/X11 fixed origin placement' if args.place_only
                                     else 'owned native SDL/X11 whole-window XGetImage'),
              'container_id': args.container_id, 'container_binding': 'caller exact-CID Docker exec ownership proof required',
              'helper_process': process(os.getpid()),
              'started_monotonic_ns': time.monotonic_ns(), 'resize_used': False, 'crop_used': False,
              'Wayland_native_buffer_pixel_equality_qualified': False,
              'guest_UI_injection_used': False, 'host_frontend_origin_placement_used': False,
              'place_only_requested': args.place_only, 'actual_pixels_captured': False,
              'hardware_verified': False, 'Xlib_errors': [], 'expected_display_argument': DISPLAY_ARGUMENT}
    display = image = None
    watches = []
    x = None
    previous_error_handler = error_handler = None
    error_handler_installed = False
    stage = {'operation': 'initial ownership qualification'}
    try:
        qemu = process(args.qemu_pid)
        assert qemu['uid'] == 1000 and qemu['start_ticks'] == args.qemu_start_ticks
        assert qemu['cmdline_sha256'] == args.qemu_cmdline_sha256
        assert Path(qemu['argv'][0]).name == 'qemu-system-aarch64'
        assert qemu['argv'].count(GPU) == 1 and qemu['argv'].count(DISPLAY_ARGUMENT) == 1
        assert args.qmp_argument in qemu['argv']
        qa = ancestry(qemu['pid'], 1)
        candidates = [r for r in qa[1:] if any(Path(v).name == 'xvfb-run' for v in r['argv'][:2])]
        assert len(candidates) == 1, 'one exact QEMU xvfb-run ancestor required'
        wrapper = candidates[0]
        qw, ww = watch(qemu['pid']), watch(wrapper['pid'])
        watches.extend([qw, ww])
        alive(qemu, qw)
        alive(wrapper, ww)
        envraw = (Path('/proc') / str(qemu['pid']) / 'environ').read_bytes()
        assert len(envraw) <= 256 * 1024
        selected = {}
        for value in envraw.split(b'\0'):
            if value.startswith(b'DISPLAY=') or value.startswith(b'XAUTHORITY='):
                key, val = value.split(b'=', 1)
                selected[os.fsdecode(key)] = os.fsdecode(val)
        del envraw  # Never log or persist process environment or authorization contents.
        assert re.fullmatch(r':[0-9]+(?:\.0)?', selected.get('DISPLAY', '')), 'local owned X11 display only'
        authority = Path(selected['XAUTHORITY'])
        assert authority.is_absolute() and stat.S_ISREG(authority.lstat().st_mode)
        assert authority.stat().st_uid == 1000 and authority.stat().st_mode & 0o077 == 0
        os.environ['DISPLAY'] = selected['DISPLAY']
        os.environ['XAUTHORITY'] = selected['XAUTHORITY']
        result['Xauthority'] = {'authorization_consumed_locally_by_Xlib': True,
                                'authorization_exported': False,
                                'private_authority_path_may_be_in_process_identity': True}
        x = C.CDLL('libX11.so.6')
        signatures = {
            'XOpenDisplay': ([C.c_char_p], P), 'XCloseDisplay': ([P], I),
            'XConnectionNumber': ([P], I), 'XDefaultRootWindow': ([P], U),
            'XSetErrorHandler': ([P], P),
            'XInternAtom': ([P, C.c_char_p, I], U), 'XFree': ([P], I), 'XSync': ([P, I], I),
            'XQueryTree': ([P, U, C.POINTER(U), C.POINTER(U), C.POINTER(C.POINTER(U)), C.POINTER(C.c_uint)], I),
            'XGetWindowAttributes': ([P, U, C.POINTER(Attributes)], I),
            'XTranslateCoordinates': ([P, U, U, I, I, C.POINTER(I), C.POINTER(I), C.POINTER(U)], I),
            'XMoveWindow': ([P, U, I, I], I),
            'XGetWindowProperty': ([P, U, U, C.c_long, C.c_long, I, U, C.POINTER(U), C.POINTER(I),
                                   C.POINTER(U), C.POINTER(U), C.POINTER(P)], I),
            'XGetImage': ([P, U, I, I, C.c_uint, C.c_uint, U, I], C.POINTER(XImage)),
            'XDestroyImage': ([C.POINTER(XImage)], I),
        }
        for name, (arguments, returned) in signatures.items():
            fn = getattr(x, name)
            fn.argtypes, fn.restype = arguments, returned

        def on_xerror(connection, event_pointer):
            event = event_pointer.contents
            row = {'error_code': event.error_code, 'request_code': event.request_code,
                   'minor_code': event.minor_code, 'resource_id': event.resourceid,
                   'serial': event.serial, 'operation': stage['operation'],
                   'connection_matches': bool(connection == event.display)}
            if len(result['Xlib_errors']) < 16:
                result['Xlib_errors'].append(row)
            else:
                result['Xlib_error_count_exceeded'] = True
            return 0

        error_handler = XErrorHandler(on_xerror)  # Retain callback through XCloseDisplay.
        previous_error_handler = x.XSetErrorHandler(C.cast(error_handler, P))
        error_handler_installed = True
        stage['operation'] = 'XOpenDisplay'
        display = x.XOpenDisplay(selected['DISPLAY'].encode())
        assert display, 'XOpenDisplay failed'
        with socket.socket(fileno=os.dup(x.XConnectionNumber(display))) as connection:
            server_pid, server_uid, server_gid = struct.unpack('3i', connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        assert server_uid == server_gid == 1000
        server = process(server_pid)
        assert Path(server['argv'][0]).name == 'Xvfb' and server['uid'] == 1000
        assert selected['DISPLAY'].split('.')[0] in server['argv']
        assert '1080x2424x24' in server['argv'] and '-nolisten' in server['argv'] and 'tcp' in server['argv']
        sa = ancestry(server_pid, wrapper['pid'])
        assert sa[-1] == wrapper, 'X server must have QEMU same exact xvfb-run owner'
        sw = watch(server_pid)
        watches.append(sw)
        result.update(qemu=qemu, QEMU_ancestry_to_container_init=qa, xvfb_owner=wrapper,
                      X11_server_peer={'pid': server_pid, 'uid': server_uid, 'gid': server_gid,
                                       'process': server, 'ancestry_to_xvfb_owner': sa})

        def check_xerrors():
            if result['Xlib_errors'] or result.get('Xlib_error_count_exceeded'):
                raise RuntimeError('actual structured Xlib error retained; no retry or fallback')

        def synchronize(operation):
            stage['operation'] = operation
            x.XSync(display, 0)
            check_xerrors()

        def attrs(window):
            value = Attributes()
            stage['operation'] = 'XGetWindowAttributes window=' + str(window)
            assert x.XGetWindowAttributes(display, window, C.byref(value)), 'window attributes unavailable'
            check_xerrors()
            return value

        def root_position(window, root):
            rx, ry, child = I(), I(), U()
            stage['operation'] = 'XTranslateCoordinates window=' + str(window)
            assert x.XTranslateCoordinates(display, window, root, 0, 0, C.byref(rx), C.byref(ry), C.byref(child)), 'root coordinate translation failed'
            check_xerrors()
            return {'x': rx.value, 'y': ry.value, 'child_at_origin': child.value}

        def prop(window, name):
            stage['operation'] = 'XInternAtom ' + name
            atom = x.XInternAtom(display, name.encode(), 1)
            if not atom:
                return None
            kind, bits, count, after, data = U(), I(), U(), U(), P()
            stage['operation'] = 'XGetWindowProperty ' + name + ' window=' + str(window)
            status = x.XGetWindowProperty(display, window, atom, 0, 256, 0, 0,
                                          C.byref(kind), C.byref(bits), C.byref(count), C.byref(after), C.byref(data))
            try:
                check_xerrors()
                assert status == 0 and after.value == 0, 'bounded complete X property required'
                if name == '_NET_WM_PID':
                    if kind.value == 6 and bits.value == 32 and count.value == 1 and data:
                        return C.cast(data, C.POINTER(U))[0]
                    return None
                if bits.value == 8 and count.value <= 1024 and data:
                    return C.string_at(data, count.value).decode('utf-8', errors='replace').split('\0')
                return None
            finally:
                if data:
                    x.XFree(data)

        root = x.XDefaultRootWindow(display)
        ra = attrs(root)
        assert (ra.width, ra.height, ra.depth) == (W, H, 24), 'actual X server whole screen geometry/depth differs'
        pending, visited, selected_windows = [(root, 0)], set(), []
        while pending:
            window, depth = pending.pop()
            assert window not in visited and depth <= 16 and len(visited) < 4096
            visited.add(window)
            wa = attrs(window)
            if prop(window, '_NET_WM_PID') == qemu['pid'] and wa.map_state == 2 and wa.window_class == 1:
                classes = prop(window, 'WM_CLASS') or []
                if any('qemu' in value.lower() for value in classes):
                    selected_windows.append((window, wa, classes))
            tree_root, parent, children, count = U(), U(), C.POINTER(U)(), C.c_uint()
            stage['operation'] = 'XQueryTree window=' + str(window)
            assert x.XQueryTree(display, window, C.byref(tree_root), C.byref(parent), C.byref(children), C.byref(count))
            try:
                check_xerrors()
                assert count.value <= 4096 - len(visited)
                pending.extend((children[i], depth + 1) for i in range(count.value))
            finally:
                if children:
                    x.XFree(children)
        assert len(selected_windows) == 1, 'exactly one owned mapped SDL window required'
        window, wa, classes = selected_windows[0]
        before_position = root_position(window, root)
        geometry = {'window_id': window, 'width': wa.width, 'height': wa.height,
                    'border_width': wa.border_width, 'depth': wa.depth, 'map_state': wa.map_state,
                    'parent_relative_position': {'x': wa.x, 'y': wa.y}, 'root_position': before_position,
                    'root': {'id': root, 'width': ra.width, 'height': ra.height, 'depth': ra.depth},
                    'WM_CLASS': classes, 'owner_PID_property': qemu['pid']}
        result['geometry_before_placement' if args.place_only else 'geometry_before_XGetImage'] = geometry
        exclusive(Path(str(prefix) + '.before.json'), (json.dumps(result, indent=2) + '\n').encode())
        assert (wa.width, wa.height, wa.border_width, wa.depth, wa.root) == (W, H, 0, 24, root), 'actual owned native1080x2424 SDL viewport required; no resize permitted'
        if args.place_only:
            # Caller orders this fixed frontend step before the first Source adapter load.
            alive(qemu, qw)
            alive(wrapper, ww)
            alive(server, sw)
            assert prop(window, '_NET_WM_PID') == qemu['pid']
            synchronize('before fixed pre-Source host origin placement')
            result['placement'] = {'target_root_origin': {'x': 0, 'y': 0},
                                   'native_viewport': {'width': W, 'height': H},
                                   'XMoveWindow_request_sent': False,
                                   'caller_pre_Source_constructor_order_required': True}
            stage['operation'] = 'fixed pre-Source XMoveWindow origin0,0'
            result['host_frontend_origin_placement_used'] = True
            move_status = x.XMoveWindow(display, window, 0, 0)
            result['placement']['XMoveWindow_request_sent'] = True
            result['placement']['XMoveWindow_status'] = move_status
            synchronize('after fixed pre-Source host origin placement')
            after = attrs(window)
            after_position = root_position(window, root)
            result['geometry_after_placement'] = {'root_position': after_position,
                                                   'width': after.width, 'height': after.height,
                                                   'border_width': after.border_width, 'depth': after.depth,
                                                   'map_state': after.map_state, 'root_id': after.root}
            exclusive(Path(str(prefix) + '.after.json'), (json.dumps(result, indent=2) + '\n').encode())
            assert (after.width, after.height, after.border_width, after.depth, after.map_state, after.root) == (W, H, 0, 24, 2, root), 'placement may change only origin'
            assert (after_position['x'], after_position['y']) == (0, 0), 'actual fixed root origin placement required'
            assert prop(window, '_NET_WM_PID') == qemu['pid']
            alive(qemu, qw)
            alive(wrapper, ww)
            alive(server, sw)
            result.update(ok=True, process_deathwatch_before_after=True,
                          placement_scope='host frontend constructor origin only; no guest input, model or source mutation',
                          PNG_or_RGB_created=False)
        else:
            assert (before_position['x'], before_position['y']) == (0, 0), 'whole SDL window must start at root0,0; no offscreen capture'
            assert 0 <= before_position['x'] and before_position['x'] + wa.width <= ra.width
            assert 0 <= before_position['y'] and before_position['y'] + wa.height <= ra.height
            # Entire mapped client window, never a root-screen crop or transformed export.
            alive(qemu, qw)
            alive(wrapper, ww)
            alive(server, sw)
            synchronize('before whole-window XGetImage')
            stage['operation'] = 'whole-window XGetImage'
            image = x.XGetImage(display, window, 0, 0, W, H, U(-1).value, 2)
            synchronize('after whole-window XGetImage')
            assert image, 'whole-window XGetImage failed'
            im = image.contents
            assert (im.width, im.height, im.xoffset, im.format, im.depth) == (W, H, 0, 2, 24)
            assert (im.red_mask, im.green_mask, im.blue_mask) == (0xff0000, 0xff00, 0xff)
            assert im.bits_per_pixel in (24, 32) and im.byte_order in (0, 1)
            step = im.bits_per_pixel // 8
            assert W * step <= im.bytes_per_line <= W * step + 1024
            pixels = C.string_at(im.data, im.bytes_per_line * H)
            result['actual_pixels_captured'] = True
            rgb = bytearray(W * H * 3)
            for row in range(H):
                data = pixels[row * im.bytes_per_line:row * im.bytes_per_line + W * step]
                if im.byte_order == 0:
                    red, green, blue = data[2::step], data[1::step], data[0::step]
                elif step == 4:
                    red, green, blue = data[1::step], data[2::step], data[3::step]
                else:
                    red, green, blue = data[0::step], data[1::step], data[2::step]
                offset = row * W * 3
                rgb[offset:offset + W * 3:3] = red
                rgb[offset + 1:offset + W * 3:3] = green
                rgb[offset + 2:offset + W * 3:3] = blue
            synchronize('after RGB extraction')
            after = attrs(window)
            after_position = root_position(window, root)
            result['geometry_after_XGetImage'] = {'root_position': after_position,
                                                  'width': after.width, 'height': after.height,
                                                  'border_width': after.border_width, 'depth': after.depth,
                                                  'map_state': after.map_state}
            assert (after.width, after.height, after.border_width, after.depth, after.map_state) == (W, H, 0, 24, 2)
            assert (after_position['x'], after_position['y']) == (0, 0)
            assert prop(window, '_NET_WM_PID') == qemu['pid']
            alive(qemu, qw)
            alive(wrapper, ww)
            alive(server, sw)
            encoded = png(rgb)
            exclusive(Path(str(prefix) + '.rgb'), rgb)
            exclusive(Path(str(prefix) + '.png'), encoded)
            result.update(ok=True, window={'id': window, 'owner_PID_property': qemu['pid'], 'WM_CLASS': classes,
                                          'width': W, 'height': H, 'depth': wa.depth, 'border_width': wa.border_width,
                                          'map_state': wa.map_state, 'x': wa.x, 'y': wa.y},
                          XImage={'width': W, 'height': H, 'bits_per_pixel': im.bits_per_pixel,
                                  'byte_order': im.byte_order, 'stride': im.bytes_per_line,
                                  'masks': [im.red_mask, im.green_mask, im.blue_mask]},
                          capture_rectangle={'x': 0, 'y': 0, 'width': W, 'height': H, 'whole_client_window': True},
                          rgb_bytes=len(rgb), rgb_sha256=digest(rgb), png_bytes=len(encoded), png_sha256=digest(encoded),
                          independent_PNG_decode_required=True, process_deathwatch_before_after=True)
    except BaseException as error:
        result['error'] = type(error).__name__ + ': ' + str(error)
    finally:
        if image:
            x.XDestroyImage(image)
        if display:
            stage['operation'] = 'XCloseDisplay'
            x.XCloseDisplay(display)
        if error_handler_installed:
            x.XSetErrorHandler(previous_error_handler)
        if result['Xlib_errors'] or result.get('Xlib_error_count_exceeded'):
            result['ok'] = False
            result.setdefault('error', 'RuntimeError: actual structured Xlib error retained; no retry or fallback')
        for fd, _ in watches:
            os.close(fd)
        result['completed_monotonic_ns'] = time.monotonic_ns()
        exclusive(Path(str(prefix) + '.reply.json'), (json.dumps(result, indent=2) + '\n').encode())
        signal.alarm(0)
    print(json.dumps({'ok': result['ok'], 'method': result['method'], 'error': result.get('error')}))
    return 0 if result['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
