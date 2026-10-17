#!/usr/bin/env python3
"""Independent source-only real SEQPACKET consumer. No production Python/Rust imports."""
import argparse, array, fcntl, json, os, select, socket, struct, sys, time

MAX_REPLY = 16384
MAX_OUTPUT = 16384
SO_PEERPIDFD = 77  # Linux UAPI; unsupported kernel is setup failure, no PID fallback.

def now():
    return time.clock_gettime_ns(time.CLOCK_BOOTTIME)

def before(until):
    if now() >= until:
        raise TimeoutError('original absolute operator cutoff')

def wait(fd, write, until):
    before(until)
    read, writable, error = select.select([] if write else [fd], [fd] if write else [], [fd],
        min((until-now())/1e9, .02))
    if error:
        raise OSError('socket poll error')
    before(until)

def emit(value, code):
    data=(json.dumps(value,separators=(',',':'),sort_keys=True)+'\n').encode()
    if len(data)>MAX_OUTPUT:
        raise ValueError('bounded helper output')
    os.set_blocking(1,False)
    if os.write(1,data)!=len(data):
        raise OSError('whole helper PIPE write required')
    return code

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--socket',required=True)
    p.add_argument('--server-pid',required=True,type=int)
    p.add_argument('--until-ns',required=True,type=int)
    g=p.add_mutually_exclusive_group(required=True)
    g.add_argument('--admit',help='one exact command JSON object, not a config pathname')
    g.add_argument('--stop',action='store_true')
    a=p.parse_args(); before(a.until_ns)
    if a.stop:
        request=b'\x02'
    else:
        body=json.loads(a.admit)
        expected={'schema','incarnation','attempt','serial','request','slot','payload_sha256','deadline_ns'}
        if set(body)!=expected or body['schema']!=1 or body['attempt']!=1 or body['serial']!=1 or body['request']!=105 or body['slot']!=1:
            raise ValueError('fixed operator command fields')
        if len(body['incarnation'])!=32 or len(body['payload_sha256'])!=64:
            raise ValueError('exact identity/digest width')
        bytes.fromhex(body['incarnation']); bytes.fromhex(body['payload_sha256'])
        request=b'\x03'+json.dumps(body,separators=(',',':'),sort_keys=True).encode()
        if len(request)>512:
            raise ValueError('operator command bound')
    s=None; peerfd=None
    try:
        while True:
            before(a.until_ns)
            candidate=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET|socket.SOCK_CLOEXEC|socket.SOCK_NONBLOCK)
            candidate.setsockopt(socket.SOL_SOCKET,socket.SO_PASSCRED,1)
            try:
                candidate.connect(a.socket); s=candidate; break
            except BlockingIOError:
                candidate.close(); time.sleep(.001)
            except BaseException:
                candidate.close(); raise
        peer=struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))
        if peer!=(a.server_pid,1500,1500):
            raise ValueError('actual server PID/UID/GID does not match bound Main')
        peerfd=s.getsockopt(socket.SOL_SOCKET,SO_PEERPIDFD)
        if peerfd<0 or not fcntl.fcntl(peerfd,fcntl.F_GETFD)&fcntl.FD_CLOEXEC:
            raise ValueError('owned socket-origin CLOEXEC pidfd required')
        if select.select([peerfd],[],[],0)[0]:
            raise ValueError('server already exited')
        creds=struct.pack('3i',os.getpid(),os.geteuid(),os.getegid())
        while True:
            before(a.until_ns)
            try:
                n=s.sendmsg([request],[(socket.SOL_SOCKET,socket.SCM_CREDENTIALS,creds)],socket.MSG_DONTWAIT|socket.MSG_NOSIGNAL)
                if n!=len(request):
                    raise ValueError('short SEQPACKET command')
                break
            except (BlockingIOError,InterruptedError):
                wait(s,True,a.until_ns)
            except (BrokenPipeError,ConnectionResetError):
                if a.stop:raise
                return emit({'classification':'refused-or-missing-command','sender_uid':os.geteuid(),
                    'server_pid':a.server_pid,'unknown_command':True},101)
        while True:
            before(a.until_ns)
            try:
                data,anc,flags,_=s.recvmsg(MAX_REPLY,socket.CMSG_SPACE(12)+socket.CMSG_SPACE(4),
                    socket.MSG_DONTWAIT|socket.MSG_CMSG_CLOEXEC)
                break
            except (BlockingIOError,InterruptedError):
                wait(s,False,a.until_ns)
            except ConnectionResetError:
                data=b'';anc=[];flags=0;break
        if not data:
            # Current code closes unknown03. This is structural missing capability;
            # envelope still must prove healthy setup and zero105/normal closure.
            return emit({'classification':'refused-or-missing-command','sender_uid':os.geteuid(),
                'server_pid':a.server_pid,'unknown_command':not a.stop},101)
        if flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC):
            raise ValueError('truncated reply')
        got=[]
        for level,kind,value in anc:
            if level==socket.SOL_SOCKET and kind==socket.SCM_CREDENTIALS and len(value)==12:
                got.append(struct.unpack('3i',value))
            else:
                if level==socket.SOL_SOCKET and kind==socket.SCM_RIGHTS:
                    fds=array.array('i');fds.frombytes(value[:len(value)//4*4])
                    for fd in fds:os.close(fd)
                raise ValueError('unexpected reply ancillary data')
        if got!=[peer]:
            raise ValueError('exact connected server packet credentials required')
        if a.stop:
            if len(data)<12 or data[:4]!=b'RADS' or data[5:8]!=b'\x02\x00\x00' or struct.unpack('<I',data[8:12])[0]!=len(data):
                raise ValueError('StopAck envelope')
            if data[4]==1 and len(data)==12:
                return emit({'classification':'baseline-stop-ack-no-history'},0)
            if data[4]!=2:
                raise ValueError('current StopAck version2 required')
            reply=json.loads(data[12:])
            return emit({'classification':'stop-snapshot','snapshot':reply},0)
        if len(data)>1024:
            raise ValueError('operator reply bound')
        reply=json.loads(data)
        if set(reply)!={'schema','admitted','reason','incarnation','attempt','request_revision'} or reply['schema']!=1 or type(reply['admitted']) is not bool:
            raise ValueError('typed operator reply shape')
        return emit({'classification':'operator-reply','reply':reply},0)
    finally:
        if peerfd is not None:os.close(peerfd)
        if s is not None:s.close()

if __name__=='__main__':
    try:sys.exit(main())
    except Exception as e:
        # Setup/protocol failure is separate from structural unknown-command RED.
        try:emit({'classification':'setup-or-protocol-failure','error':str(e)},70)
        finally:sys.exit(70)
