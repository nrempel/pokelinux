#define _GNU_SOURCE
/* Actual prepared-component Wayland client: XDG toplevel, SHM damage, real keyboard and session-lock requests. */
#include <wayland-client.h>
#include "xdg-shell-client.h"
#include "session-lock-client.h"
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
static struct wl_surface *surface,*lock_surface;
static struct ext_session_lock_manager_v1 *locks;
static struct ext_session_lock_v1 *lock;
static struct ext_session_lock_surface_v1 *lock_role;
static uint64_t until;
static unsigned buffers,width=411,height=923,mapped,lock_requested;
static struct wl_buffer *owned_buffers[8];
static volatile sig_atomic_t stopping;
static uint64_t ns(void){struct timespec t;if(clock_gettime(CLOCK_BOOTTIME,&t))_exit(90);return (uint64_t)t.tv_sec*1000000000ULL+(uint64_t)t.tv_nsec;}
static void event(const char *name,unsigned value){printf("{\"event\":\"%s\",\"at_ns\":%llu,\"pid\":%d,\"uid\":%u,\"value\":%u}\n",name,(unsigned long long)ns(),getpid(),getuid(),value);fflush(stdout);}
static void stop_signal(int sig){U(sig);stopping=1;}
static void released(void *data,struct wl_buffer *b){U(b);event("buffer_release",(unsigned)(uintptr_t)data);}
static const struct wl_buffer_listener buffer_listener={.release=released};
static void submit(struct wl_surface *target,unsigned w,unsigned h,unsigned role){
 if(!target||!w||!h||w>16384||h>16384||(uint64_t)w*h*4>64*1024*1024||buffers>=8)_exit(91);
 const size_t bytes=(size_t)w*h*4;int fd=memfd_create("owned-render-stall-shm",MFD_CLOEXEC);if(fd<0||ftruncate(fd,(off_t)bytes))_exit(92);
 uint32_t *p=mmap(NULL,bytes,PROT_READ|PROT_WRITE,MAP_SHARED,fd,0);if(p==MAP_FAILED)_exit(93);
 for(size_t i=0;i<bytes/4;i++)p[i]=role?0xff102030U:buffers%2?0xff405060U:0xff708090U;
 struct wl_shm_pool *pool=wl_shm_create_pool(shm,fd,(int)bytes);struct wl_buffer *buffer=wl_shm_pool_create_buffer(pool,0,(int)w,(int)h,(int)w*4,WL_SHM_FORMAT_XRGB8888);
 owned_buffers[buffers++]=buffer;wl_buffer_add_listener(buffer,&buffer_listener,(void*)(uintptr_t)buffers);
 wl_shm_pool_destroy(pool);munmap(p,bytes);close(fd);
 wl_surface_attach(target,buffer,0,0);wl_surface_damage_buffer(target,0,0,(int)w,(int)h);wl_surface_commit(target);event(role?"lock_buffer_committed":"window_buffer_committed",buffers);
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
static void capabilities(void *data,struct wl_seat *s,uint32_t caps){U(data);if((caps&WL_SEAT_CAPABILITY_KEYBOARD)&&!keyboard){keyboard=wl_seat_get_keyboard(s);wl_keyboard_add_listener(keyboard,&keyboard_listener,NULL);}}
static void seat_name(void *data,struct wl_seat *s,const char *name){U(data);U(s);U(name);}
static const struct wl_seat_listener seat_listener={.capabilities=capabilities,.name=seat_name};
static void global(void *data,struct wl_registry *reg,uint32_t name,const char *iface,uint32_t version){U(data);
 if(!strcmp(iface,"wl_compositor")&&version>=4)compositor=wl_registry_bind(reg,name,&wl_compositor_interface,4);
 else if(!strcmp(iface,"wl_shm"))shm=wl_registry_bind(reg,name,&wl_shm_interface,1);
 else if(!strcmp(iface,"wl_output")&&!output)output=wl_registry_bind(reg,name,&wl_output_interface,1);
 else if(!strcmp(iface,"wl_seat")&&!seat){seat=wl_registry_bind(reg,name,&wl_seat_interface,version>=7?7:version);wl_seat_add_listener(seat,&seat_listener,NULL);}
 else if(!strcmp(iface,xdg_wm_base_interface.name)){wm=wl_registry_bind(reg,name,&xdg_wm_base_interface,1);xdg_wm_base_add_listener(wm,&wm_listener,NULL);}
 else if(!strcmp(iface,ext_session_lock_manager_v1_interface.name))locks=wl_registry_bind(reg,name,&ext_session_lock_manager_v1_interface,1);
}
static void removed(void *data,struct wl_registry *reg,uint32_t name){U(data);U(reg);U(name);}
static const struct wl_registry_listener registry_listener={.global=global,.global_remove=removed};
static void locked(void *data,struct ext_session_lock_v1 *object){U(data);U(object);event("protocol_locked_event",0);}
static void finished(void *data,struct ext_session_lock_v1 *object){U(data);U(object);event("protocol_finished_event",0);}
static const struct ext_session_lock_v1_listener lock_listener={.locked=locked,.finished=finished};
static void lock_configure(void *data,struct ext_session_lock_surface_v1 *object,uint32_t serial,uint32_t w,uint32_t h){U(data);ext_session_lock_surface_v1_ack_configure(object,serial);submit(lock_surface,w,h,1);event("lock_configure_received",serial);}
static const struct ext_session_lock_surface_v1_listener lock_surface_listener={.configure=lock_configure};
static int flush(void){for(;;){if(wl_display_flush(display)>=0)return 0;if(errno!=EAGAIN||ns()>=until)return -1;struct pollfd fd={wl_display_get_fd(display),POLLOUT,0};if(poll(&fd,1,5)<0&&errno!=EINTR)return -1;}}
int main(int argc,char **argv){
 if(argc!=3){return 64;}
 char *end=NULL;until=strtoull(argv[2],&end,10);
 if(!end||*end||until<=ns()||strlen(argv[1])>64){return 64;}
 signal(SIGTERM,stop_signal);signal(SIGINT,stop_signal);signal(SIGPIPE,SIG_IGN);
 if(fcntl(STDIN_FILENO,F_SETFL,O_NONBLOCK)<0)return 65;
 display=wl_display_connect(NULL);if(!display)return 66;struct wl_registry *reg=wl_display_get_registry(display);wl_registry_add_listener(reg,&registry_listener,NULL);
 if(wl_display_roundtrip(display)<0||!compositor||!shm||!wm||!locks||!output)return 67;
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
   if(op=='D'&&mapped){submit(surface,width,height,0);if(flush())break;event("damage_request_flushed",commands);}
   else if(op=='L'&&!lock_requested){lock_requested=1;lock=ext_session_lock_manager_v1_lock(locks);ext_session_lock_v1_add_listener(lock,&lock_listener,NULL);lock_surface=wl_compositor_create_surface(compositor);lock_role=ext_session_lock_v1_get_lock_surface(lock,lock_surface,output);ext_session_lock_surface_v1_add_listener(lock_role,&lock_surface_listener,NULL);if(flush())break;event("native_lock_request_flushed",commands);}
   else if(op=='Q'){event("client_exit_requested",commands);stopping=1;}
   else {event("invalid_test_command",commands);break;}
  }}
  if(mapped&&commands==0){event("prepared_window_live",0);commands=1;}
 }
closed:
 // Disconnect is the genuine Wayland client death boundary; do not wait for a blocked server roundtrip.
 wl_display_disconnect(display);event("client_disconnected",0);return stopping?0:69;
}
