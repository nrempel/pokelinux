#define _GNU_SOURCE
/* Actual prepared-component Wayland client: XDG toplevel, SHM damage, real keyboard and session-lock requests. */
#include <wayland-client.h>
#include "xdg-shell-client.h"
#include "virtual-pointer-client.h"
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <time.h>
#include <unistd.h>
#define U(x) ((void)(x))
static struct wl_display *display;
static struct wl_compositor *compositor;
static struct wl_shm *shm;
static struct wl_output *output;
static struct wl_seat *seat;
static struct wl_keyboard *keyboard;
static struct xdg_wm_base *wm;
static struct xdg_surface *xdg;
static struct xdg_toplevel *toplevel;
static struct wl_surface *surface;
static struct zwlr_virtual_pointer_manager_v1 *virtual_pointer_manager;
static struct zwlr_virtual_pointer_v1 *virtual_pointer;
static uint64_t until;
static unsigned buffers,width=411,height=923,mapped;
static struct wl_buffer *owned_buffers[8];
static volatile sig_atomic_t stopping;
/* Private assigned-SHM/padded-cursor boundary actor; never the frozen Source actor. */
static unsigned snapshot_profile;
static struct wl_pointer *pointer;
static struct wl_surface *cursor_surface;
static uint32_t pointer_enter_serial;
static unsigned pointer_inside;
static unsigned assigned_sync_stage, cursor_sync_stage, pointer_sync_stage, assigned_requested;
struct retained_mapping { uint32_t *pixels; size_t bytes; int fd; unsigned released; };
static struct retained_mapping retained[8];
static uint32_t profile_pixel(unsigned x,unsigned y,unsigned w,unsigned h,unsigned cursor){
 static const uint32_t colors[4]={0xff142c4cU,0xff386084U,0xff74a0bcU,0xffb0c8e0U};
 const unsigned quadrant=(y>=h/2?2U:0U)+(x>=w/2?1U:0U);
 if(cursor)return 0xff000000U|(((17U+7U*x+3U*y)&255U)<<16)|(((29U+5U*x+11U*y)&255U)<<8)|((43U+13U*x+17U*y)&255U);
 return colors[quadrant];
}
static uint64_t ns(void){struct timespec t;if(clock_gettime(CLOCK_BOOTTIME,&t))_exit(90);return (uint64_t)t.tv_sec*1000000000ULL+(uint64_t)t.tv_nsec;}
static void event(const char *name,unsigned value){printf("{\"event\":\"%s\",\"at_ns\":%llu,\"pid\":%d,\"uid\":%u,\"value\":%u}\n",name,(unsigned long long)ns(),getpid(),getuid(),value);fflush(stdout);}
static void stop_signal(int sig){U(sig);stopping=1;}
static void released(void *data,struct wl_buffer *b){
 U(b);const unsigned index=(unsigned)(uintptr_t)data;
 if(snapshot_profile&&index>0&&index<=8)retained[index-1].released=1;
 event("buffer_release",index);
}
static const struct wl_buffer_listener buffer_listener={.release=released};
static void submit(struct wl_surface *target,unsigned w,unsigned h,unsigned role){
 if(!target||!w||!h||w>16384||h>16384||(uint64_t)w*h*4>64*1024*1024||buffers>=8)_exit(91);
 const unsigned stride=w*4+(snapshot_profile&&role==2?16:0);
 const size_t bytes=(size_t)stride*h;int fd=memfd_create("owned-shm-consumer",MFD_CLOEXEC);if(fd<0||ftruncate(fd,(off_t)bytes))_exit(92);
 uint32_t *p=mmap(NULL,bytes,PROT_READ|PROT_WRITE,MAP_SHARED,fd,0);if(p==MAP_FAILED)_exit(93);
 memset(p,0xa5,bytes);
 for(unsigned y=0;y<h;y++)for(unsigned x=0;x<w;x++){
  const uint32_t pixel=profile_pixel(x,y,w,h,role==2);memcpy((uint8_t*)p+(size_t)y*stride+(size_t)x*4,&pixel,4);
 }
 struct wl_shm_pool *pool=wl_shm_create_pool(shm,fd,(int)bytes);struct wl_buffer *buffer=wl_shm_pool_create_buffer(pool,0,(int)w,(int)h,(int)stride,role==2?WL_SHM_FORMAT_ARGB8888:WL_SHM_FORMAT_XRGB8888);
 owned_buffers[buffers++]=buffer;wl_buffer_add_listener(buffer,&buffer_listener,(void*)(uintptr_t)buffers);
 wl_shm_pool_destroy(pool);
 if(snapshot_profile)retained[buffers-1]=(struct retained_mapping){p,bytes,fd,0};
 else {munmap(p,bytes);close(fd);}
 wl_surface_attach(target,buffer,0,0);wl_surface_damage_buffer(target,0,0,(int)w,(int)h);wl_surface_commit(target);event(snapshot_profile&&role==2?"profile_cursor_buffer_committed":role?"lock_buffer_committed":"window_buffer_committed",buffers);
}
static void ping(void *data,struct xdg_wm_base *base,uint32_t serial){U(data);xdg_wm_base_pong(base,serial);}
static const struct xdg_wm_base_listener wm_listener={.ping=ping};
static void configured(void *data,struct xdg_surface *s,uint32_t serial){U(data);xdg_surface_ack_configure(s,serial);submit(surface,width,height,0);mapped=1;event("xdg_configure_received",serial);}
static const struct xdg_surface_listener xdg_listener={.configure=configured};
static void top_configure(void *data,struct xdg_toplevel *top,int32_t w,int32_t h,struct wl_array *states){U(data);U(top);U(states);if(w>0)width=(unsigned)w;if(h>0)height=(unsigned)h;}
static void top_close(void *data,struct xdg_toplevel *top){U(data);U(top);stopping=1;}
static const struct xdg_toplevel_listener top_listener={.configure=top_configure,.close=top_close};
static void keymap(void *data,struct wl_keyboard *k,uint32_t format,int32_t fd,uint32_t size){U(data);U(k);U(format);U(size);close(fd);}
static void enter(void *data,struct wl_keyboard *k,uint32_t serial,struct wl_surface *s,struct wl_array *keys){U(data);U(k);U(serial);U(keys);event("keyboard_enter",s==surface?1:0);}
static void leave(void *data,struct wl_keyboard *k,uint32_t serial,struct wl_surface *s){U(data);U(k);U(serial);U(s);event("keyboard_leave",0);}
static void key(void *data,struct wl_keyboard *k,uint32_t serial,uint32_t time,uint32_t code,uint32_t state){U(data);U(k);U(serial);U(time);U(code);U(state);}
static void modifiers(void *data,struct wl_keyboard *k,uint32_t serial,uint32_t a,uint32_t b,uint32_t c,uint32_t group){U(data);U(k);U(serial);U(a);U(b);U(c);U(group);}
static void repeat(void *data,struct wl_keyboard *k,int32_t rate,int32_t delay){U(data);U(k);U(rate);U(delay);}
static const struct wl_keyboard_listener keyboard_listener={.keymap=keymap,.enter=enter,.leave=leave,.key=key,.modifiers=modifiers,.repeat_info=repeat};
static void pointer_enter(void *data,struct wl_pointer *p,uint32_t serial,struct wl_surface *s,wl_fixed_t x,wl_fixed_t y){U(data);U(p);U(x);U(y);pointer_inside=s==surface;pointer_enter_serial=serial;event("profile_pointer_enter_serial",serial);event("profile_pointer_enter",pointer_inside);}
static void pointer_leave(void *data,struct wl_pointer *p,uint32_t serial,struct wl_surface *s){U(data);U(p);U(serial);U(s);pointer_inside=0;event("profile_pointer_leave",0);}
static void pointer_motion(void *data,struct wl_pointer *p,uint32_t t,wl_fixed_t x,wl_fixed_t y){U(data);U(p);U(t);U(x);U(y);}
static void pointer_button(void *data,struct wl_pointer *p,uint32_t s,uint32_t t,uint32_t b,uint32_t state){U(data);U(p);U(s);U(t);U(b);U(state);}
static void pointer_axis(void *data,struct wl_pointer *p,uint32_t t,uint32_t axis,wl_fixed_t value){U(data);U(p);U(t);U(axis);U(value);}
static void pointer_frame(void *data,struct wl_pointer *p){U(data);U(p);}
static void pointer_axis_source(void *data,struct wl_pointer *p,uint32_t source){U(data);U(p);U(source);}
static void pointer_axis_stop(void *data,struct wl_pointer *p,uint32_t t,uint32_t axis){U(data);U(p);U(t);U(axis);}
static void pointer_axis_discrete(void *data,struct wl_pointer *p,uint32_t axis,int32_t discrete){U(data);U(p);U(axis);U(discrete);}
static const struct wl_pointer_listener pointer_listener={.enter=pointer_enter,.leave=pointer_leave,.motion=pointer_motion,.button=pointer_button,.axis=pointer_axis,.frame=pointer_frame,.axis_source=pointer_axis_source,.axis_stop=pointer_axis_stop,.axis_discrete=pointer_axis_discrete};
static void capabilities(void *data,struct wl_seat *s,uint32_t caps){
 U(data);event("profile_seat_capabilities",caps);if((caps&WL_SEAT_CAPABILITY_KEYBOARD)&&!keyboard){keyboard=wl_seat_get_keyboard(s);wl_keyboard_add_listener(keyboard,&keyboard_listener,NULL);}
 if(snapshot_profile&&(caps&WL_SEAT_CAPABILITY_POINTER)&&!pointer){pointer=wl_seat_get_pointer(s);wl_pointer_add_listener(pointer,&pointer_listener,NULL);}
}
static void seat_name(void *data,struct wl_seat *s,const char *name){U(data);U(s);U(name);}
static const struct wl_seat_listener seat_listener={.capabilities=capabilities,.name=seat_name};
static void global(void *data,struct wl_registry *reg,uint32_t name,const char *iface,uint32_t version){U(data);
 if(!strcmp(iface,"wl_compositor")&&version>=4)compositor=wl_registry_bind(reg,name,&wl_compositor_interface,4);
 else if(!strcmp(iface,"wl_shm"))shm=wl_registry_bind(reg,name,&wl_shm_interface,1);
 else if(!strcmp(iface,"wl_output")&&!output)output=wl_registry_bind(reg,name,&wl_output_interface,1);
 else if(!strcmp(iface,"wl_seat")&&!seat){seat=wl_registry_bind(reg,name,&wl_seat_interface,version>=7?7:version);wl_seat_add_listener(seat,&seat_listener,NULL);}
 else if(!strcmp(iface,xdg_wm_base_interface.name)){wm=wl_registry_bind(reg,name,&xdg_wm_base_interface,1);xdg_wm_base_add_listener(wm,&wm_listener,NULL);}
 else if(!strcmp(iface,zwlr_virtual_pointer_manager_v1_interface.name)&&version>=2)virtual_pointer_manager=wl_registry_bind(reg,name,&zwlr_virtual_pointer_manager_v1_interface,2);
}
static void removed(void *data,struct wl_registry *reg,uint32_t name){U(data);U(reg);U(name);}
static const struct wl_registry_listener registry_listener={.global=global,.global_remove=removed};
static int flush(void){for(;;){if(wl_display_flush(display)>=0)return 0;if(errno!=EAGAIN||ns()>=until)return -1;struct pollfd fd={wl_display_get_fd(display),POLLOUT,0};if(poll(&fd,1,5)<0&&errno!=EINTR)return -1;}}
static void boundary_frame_done(void *data,struct wl_callback *callback,uint32_t time){
 U(data);U(time);wl_callback_destroy(callback);event("profile_assigned_frame_done",wl_proxy_get_id((struct wl_proxy*)surface));
}
static const struct wl_callback_listener boundary_frame_listener={.done=boundary_frame_done};
static void boundary_sync_done(void *data,struct wl_callback *callback,uint32_t serial){
 U(serial);wl_callback_destroy(callback);const unsigned stage=(unsigned)(uintptr_t)data;
 if(stage==1){assigned_sync_stage=1;event("profile_assigned_synced",wl_proxy_get_id((struct wl_proxy*)surface));}
 else if(stage==2){cursor_sync_stage=1;event("profile_cursor_synced",wl_proxy_get_id((struct wl_proxy*)cursor_surface));}
 else {pointer_sync_stage=1;event("profile_pointer_motion_synced",wl_proxy_get_id((struct wl_proxy*)virtual_pointer));}
}
static const struct wl_callback_listener boundary_sync_listener={.done=boundary_sync_done};
static int boundary_sync(unsigned stage){
 struct wl_callback *callback=wl_display_sync(display);
 if(!callback||wl_callback_add_listener(callback,&boundary_sync_listener,(void*)(uintptr_t)stage)<0||flush())return -1;
 if(stage==3)event("profile_pointer_motion_flushed",wl_proxy_get_id((struct wl_proxy*)virtual_pointer));
 else event(stage==1?"profile_assigned_flushed":"profile_cursor_flushed",wl_proxy_get_id((struct wl_proxy*)(stage==1?surface:cursor_surface)));
 return 0;
}
int main(int argc,char **argv){
 if(argc==4&&strcmp(argv[3],"--consumer-profile")==0)snapshot_profile=1;
 else if(argc!=3){return 64;}
 char *end=NULL;until=strtoull(argv[2],&end,10);
 if(!end||*end||until<=ns()||strlen(argv[1])>64){return 64;}
 signal(SIGTERM,stop_signal);signal(SIGINT,stop_signal);signal(SIGPIPE,SIG_IGN);
 if(fcntl(STDIN_FILENO,F_SETFL,O_NONBLOCK)<0)return 65;
 display=wl_display_connect(NULL);if(!display)return 66;struct wl_registry *reg=wl_display_get_registry(display);wl_registry_add_listener(reg,&registry_listener,NULL);
 if(wl_display_roundtrip(display)<0||!compositor||!shm||!wm||!seat||!output||!virtual_pointer_manager)return 67;
 virtual_pointer=zwlr_virtual_pointer_manager_v1_create_virtual_pointer_with_output(virtual_pointer_manager,seat,output);
 if(!virtual_pointer||wl_display_roundtrip(display)<0||!pointer)return 67;
 event("profile_virtual_pointer_created",wl_proxy_get_id((struct wl_proxy*)virtual_pointer));
 surface=wl_compositor_create_surface(compositor);xdg=xdg_wm_base_get_xdg_surface(wm,surface);xdg_surface_add_listener(xdg,&xdg_listener,NULL);
 toplevel=xdg_surface_get_toplevel(xdg);xdg_toplevel_add_listener(toplevel,&top_listener,NULL);xdg_toplevel_set_app_id(toplevel,argv[1]);xdg_toplevel_set_title(toplevel,argv[1]);wl_surface_commit(surface);if(flush())return 68;
 event("registry_ready",0);unsigned commands=0;
 while(!stopping&&ns()<until){
  while(wl_display_prepare_read(display)!=0)if(wl_display_dispatch_pending(display)<0)goto closed;
  if(flush()){wl_display_cancel_read(display);break;}
  struct pollfd fds[2]={{wl_display_get_fd(display),POLLIN,0},{STDIN_FILENO,POLLIN,0}};int ready=poll(fds,2,5);
  if(ready>0&&(fds[0].revents&POLLIN)){if(wl_display_read_events(display)<0||wl_display_dispatch_pending(display)<0)break;}else wl_display_cancel_read(display);
  if(ready<0&&errno!=EINTR)break;
  if(fds[1].revents&POLLIN){char op;const ssize_t n=read(STDIN_FILENO,&op,1);if(n==1){if(++commands>16)break;
   if(snapshot_profile&&op=='P'&&mapped&&pointer&&virtual_pointer&&!pointer_sync_stage){
    zwlr_virtual_pointer_v1_motion_absolute(virtual_pointer,(uint32_t)(ns()/1000000ULL),0,0,2,2);
    zwlr_virtual_pointer_v1_frame(virtual_pointer);
    zwlr_virtual_pointer_v1_motion_absolute(virtual_pointer,(uint32_t)(ns()/1000000ULL),1,1,2,2);
    zwlr_virtual_pointer_v1_frame(virtual_pointer);
    if(boundary_sync(3))break;
   }
   else if(snapshot_profile&&op=='A'&&mapped&&!assigned_requested){
    assigned_requested=1;
    struct wl_callback *frame=wl_surface_frame(surface);
    if(!frame||wl_callback_add_listener(frame,&boundary_frame_listener,NULL)<0)break;
    submit(surface,width,height,0);
    event("profile_assigned_buffer_committed",buffers);
    if(boundary_sync(1))break;
   }
   else if(snapshot_profile&&op=='C'&&mapped&&pointer&&pointer_inside&&assigned_sync_stage&&pointer_sync_stage&&!cursor_surface){
    cursor_surface=wl_compositor_create_surface(compositor);wl_pointer_set_cursor(pointer,pointer_enter_serial,cursor_surface,0,0);submit(cursor_surface,32,32,2);
    event("profile_cursor_request_queued",pointer_enter_serial);
    if(boundary_sync(2))break;
   }
   else if(op=='Q'&&assigned_sync_stage&&cursor_sync_stage){event("client_exit_requested",commands);stopping=1;}
   else {event("invalid_test_command",commands);break;}
  }}
  if(mapped&&commands==0){event("prepared_window_live",0);commands=1;}
 }
closed:
 // Disconnect is the genuine Wayland client death boundary; do not wait for a blocked server roundtrip.
 wl_display_disconnect(display);
 if(snapshot_profile)for(unsigned i=0;i<buffers;i++)if(retained[i].pixels){munmap(retained[i].pixels,retained[i].bytes);close(retained[i].fd);retained[i].pixels=NULL;}
 event("client_disconnected",0);return stopping?0:69;
}
