"""Bounded JSON framing for one local request/response per Unix connection.

The caller owns the socket and must close it after the exchange, including on
failure. This codec grants no authority and never retries a request.
"""
import json
import math
import time

MAX_FRAME = 16384  # Includes the terminating newline.


class FrameError(ValueError):
    pass


def _object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise FrameError('Duplicate JSON field')
        result[key] = value
    return result


def _constant(value):
    raise FrameError('Non-finite JSON number')


def _float(value):
    number = float(value)
    if not math.isfinite(number):
        raise FrameError('Non-finite JSON number')
    return number


def receive(sock):
    """Read one object within both a size limit and an absolute deadline."""
    original_timeout = sock.gettimeout()
    deadline = time.monotonic() + min(original_timeout if original_timeout is not None else 2, 2)
    data = bytearray()
    try:
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError('Local frame deadline')
            sock.settimeout(remaining)
            chunk = sock.recv(min(4096, MAX_FRAME + 1 - len(data)))
            if not chunk:
                raise FrameError('Incomplete JSON frame')
            data.extend(chunk)
            if len(data) > MAX_FRAME:
                raise FrameError('JSON frame too large')
            if b'\n' in chunk:
                if data.index(b'\n') != len(data) - 1:
                    raise FrameError('One frame per connection required')
                break
        try:
            value = json.loads(data.decode('utf-8'), object_pairs_hook=_object,
                               parse_constant=_constant, parse_float=_float)
        except (UnicodeError, ValueError, RecursionError) as error:
            raise FrameError('Invalid JSON frame') from error
        if not isinstance(value, dict):
            raise FrameError('JSON object required')
        return value
    finally:
        sock.settimeout(original_timeout)


def send(sock, value):
    """Serialize and bound the whole frame before writing any bytes."""
    if not isinstance(value, dict):
        raise FrameError('JSON object required')
    try:
        data = (json.dumps(value, allow_nan=False, separators=(',', ':')) + '\n').encode('utf-8')
    except (TypeError, ValueError, UnicodeError, RecursionError) as error:
        raise FrameError('Invalid JSON value') from error
    if len(data) > MAX_FRAME:
        raise FrameError('JSON frame too large')
    sock.sendall(data)
