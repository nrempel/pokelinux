"""Selected SDL window observation plus one explicit owned native-window setup mode."""
import argparse,ctypes as C,json,os,select,time
from pathlib import Path
P=C.c_void_p;U=C.c_ulong;I=C.c_int
class Attributes(C.Structure):
 _fields_=[('x',I),('y',I),('width',I),('height',I),('border_width',I),('depth',I),('visual',P),('root',U),('window_class',I),('bit_gravity',I),('win_gravity',I),('backing_store',I),('backing_planes',U),('backing_pixel',U),('save_under',I),('colormap',U),('map_installed',I),('map_state',I),('all_event_masks',C.c_long),('your_event_mask',C.c_long),('do_not_propagate_mask',C.c_long),('override_redirect',I),('screen',P)]
def now():return time.clock_gettime_ns(time.CLOCK_BOOTTIME)
def identity(pid):
 p=Path('/proc')/str(pid);s=(p/'stat').read_text().rsplit(')',1)[1].split();return{'PID':pid,'UID':p.stat().st_uid,'PPID':int(s[1]),'start_ticks':int(s[19])}
a=argparse.ArgumentParser();a.add_argument('--prepare-native-once',action='store_true');a.add_argument('--qemu-pid',type=int,required=True);a.add_argument('--xvfb-pid',type=int,required=True);a.add_argument('--parent-pid',type=int,required=True);a.add_argument('--until-host-ns',type=int,required=True);args=a.parse_args();assert os.getppid()==args.parent_pid
rows=[identity(v)for v in [args.qemu_pid,args.xvfb_pid,args.parent_pid]];fds=[os.pidfd_open(v['PID'],os.O_NONBLOCK)for v in rows];display=None

def check():
 assert now()<args.until_host_ns and not select.select(fds,[],[],0)[0]
 assert rows==[identity(v['PID'])for v in rows]and all(v['UID']==os.getuid()==1000 for v in rows)
try:
 x=C.CDLL('libX11.so.6')
 specs={'XOpenDisplay':([C.c_char_p],P),'XCloseDisplay':([P],I),'XDefaultRootWindow':([P],U),'XGetWindowAttributes':([P,U,C.POINTER(Attributes)],I),'XInternAtom':([P,C.c_char_p,I],U),'XGetWindowProperty':([P,U,U,C.c_long,C.c_long,I,U,C.POINTER(U),C.POINTER(I),C.POINTER(U),C.POINTER(U),C.POINTER(P)],I),'XQueryTree':([P,U,C.POINTER(U),C.POINTER(U),C.POINTER(C.POINTER(U)),C.POINTER(C.c_uint)],I),'XFree':([P],I),'XTranslateCoordinates':([P,U,U,I,I,C.POINTER(I),C.POINTER(I),C.POINTER(U)],I),'XGetInputFocus':([P,C.POINTER(U),C.POINTER(I)],I),'XGetSelectionOwner':([P,U],U),'XMoveResizeWindow':([P,U,I,I,C.c_uint,C.c_uint],I),'XSync':([P,I],I),'XQueryPointer':([P,U,C.POINTER(U),C.POINTER(U),C.POINTER(I),C.POINTER(I),C.POINTER(I),C.POINTER(I),C.POINTER(C.c_uint)],I)}
 for n,(inputs,output)in specs.items():getattr(x,n).argtypes=inputs;getattr(x,n).restype=output
 check();display=x.XOpenDisplay(b':97');assert display;check();root=x.XDefaultRootWindow(display)
 def attr(w):
  check();v=Attributes();assert x.XGetWindowAttributes(display,w,C.byref(v));check();return v
 parents={}
 def children(w):
  check();r=U();p=U();data=C.POINTER(U)();count=C.c_uint();assert x.XQueryTree(display,w,C.byref(r),C.byref(p),C.byref(data),C.byref(count));assert count.value<=4096
  try:result=[data[i]for i in range(count.value)]
  finally:
   if data:x.XFree(data)
  parents[w]=p.value;check();return result
 def owner(w):
  atom=x.XInternAtom(display,b'_NET_WM_PID',1)
  if not atom:return None
  typ=U();fmt=I();count=U();after=U();data=P();assert x.XGetWindowProperty(display,w,atom,0,1,0,0,C.byref(typ),C.byref(fmt),C.byref(count),C.byref(after),C.byref(data))==0
  try:return C.cast(data,C.POINTER(U))[0]if data and fmt.value==32 and count.value==1 and after.value==0 else None
  finally:
   if data:x.XFree(data)
 rootattr=attr(root);assert(rootattr.width,rootattr.height,rootattr.depth)==(1080,2424,24)
 todo=[root];seen=set();windows=[];tree={}
 while todo:
  w=todo.pop();assert w not in seen and len(seen)<4096;seen.add(w);wa=attr(w)
  if owner(w)==args.qemu_pid and wa.map_state==2 and wa.window_class==1:windows.append((w,wa))
  kids=children(w);tree[w]=kids;todo.extend(kids)
 assert len(windows)==1;window,wa=windows[0];rx=I();ry=I();child=U();assert x.XTranslateCoordinates(display,window,root,0,0,C.byref(rx),C.byref(ry),C.byref(child));check()
 setup=None
 if args.prepare_native_once:
  check();wm_atom=x.XInternAtom(display,b'WM_S0',0);wm_owner=x.XGetSelectionOwner(display,wm_atom);check();assert wm_owner==0,'explicit setup requires the private Xvfb display without a window manager'
  assert parents[window]==root and owner(window)==args.qemu_pid and wa.map_state==2
  before=(wa.width,wa.height,wa.border_width,wa.depth,rx.value,ry.value);check()
  x.XMoveResizeWindow(display,window,0,0,1080,2424);x.XSync(display,0);check()
  wa=attr(window);assert owner(window)==args.qemu_pid and wa.map_state==2;assert x.XTranslateCoordinates(display,window,root,0,0,C.byref(rx),C.byref(ry),C.byref(child));check()
  setup={'request_count':1,'before_tuple':before,'after_tuple':(wa.width,wa.height,wa.border_width,wa.depth,rx.value,ry.value),'WM_S0_selection_owner':wm_owner,'same_retained_QEMU_window_XID':window,'XSync_completed':True,'capture_or_image_scaling_selected':False,'input_injection_selected':False}
 observed_viewport=(wa.width,wa.height,wa.border_width,wa.depth,rx.value,ry.value);expected_viewport=(1080,2424,0,24,0,0)
 if observed_viewport!=expected_viewport:print(json.dumps({'viewer_viewport_failure':{'field_order':['width','height','border','depth','root_x','root_y'],'observed':observed_viewport,'expected':expected_viewport,'window_XID':window,'retained_QEMU_PID':args.qemu_pid,'read_only_observation':not args.prepare_native_once,'placement_or_resize_selected':bool(args.prepare_native_once),'explicit_native_window_setup':setup}},separators=(',',':')),flush=True)
 assert(wa.width,wa.height,wa.border_width,wa.depth,rx.value,ry.value)==(1080,2424,0,24,0,0),'selected SDL viewport must already be native and at origin; no placement/resize'
 focus=U();revert=I();x.XGetInputFocus(display,C.byref(focus),C.byref(revert));r=U();under=U();px=I();py=I();wx=I();wy=I();mask=C.c_uint();assert x.XQueryPointer(display,root,C.byref(r),C.byref(under),C.byref(px),C.byref(py),C.byref(wx),C.byref(wy),C.byref(mask));check()
 descendants={window};pending=list(tree.get(window,[]))
 while pending:
  w=pending.pop();descendants.add(w);pending.extend(tree.get(w,[]))
 focused=focus.value in descendants or(focus.value==1 and 0<=px.value<1080 and 0<=py.value<2424 and under.value in descendants)
 value={'viewport':{'x':rx.value,'y':ry.value,'width':wa.width,'height':wa.height},'window_XID':window,'owner_PID':args.qemu_pid,'retained_processes':rows,'input_focus_XID':focus.value,'keyboard_focus_valid':focused,'pointer_root':{'x':px.value,'y':py.value,'child_XID':under.value},'read_only_X11_observation':True,'placement_or_resize_or_input_selected':False}
 if setup is not None:value.update(read_only_X11_observation=False,placement_or_resize_or_input_selected=True,explicit_native_window_setup=setup)
 print(json.dumps(value),flush=True)
finally:
 if display:x.XCloseDisplay(display)
 for fd in fds:os.close(fd)
