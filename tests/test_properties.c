/* SPDX-License-Identifier: MIT */
#define _GNU_SOURCE
#include "pixel_runtime.h"
#include <assert.h>
#include <errno.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#ifdef NDEBUG
#undef NDEBUG
#include <assert.h>
#endif
static int (*get)(const char *,char *);
static const void *(*find)(const char *);
static void (*callback)(const void *,void (*)(void*,const char*,const char*,unsigned),void*);
static void check(void *cookie,const char *name,const char *value,unsigned serial){
 assert(cookie==(void*)42);assert(!strcmp(name,"ro.long"));assert(strlen(value)==160);assert(serial);
}
static void *thread(void *arg){(void)arg;for(int i=0;i<500;i++){char v[92];assert(get("ro.test",v)==6&&!strcmp(v,"vendor"));}return NULL;}
int main(void){
 char root[]="/tmp/pixel-properties-XXXXXX",path[512];assert(mkdtemp(root));
 assert(pixel_runtime_start("relative")==-1);
 assert(pixel_runtime_start(root)==-1); /* Missing inputs must not publish partial state. */
 snprintf(path,sizeof(path),"%s/system",root);assert(mkdir(path,0700)==0);
 snprintf(path,sizeof(path),"%s/vendor",root);assert(mkdir(path,0700)==0);
 snprintf(path,sizeof(path),"%s/system/build.prop",root);FILE *f=fopen(path,"w");assert(f);fputs("# snapshot\nro.test=system\nro.empty=\n",f);fclose(f);
 snprintf(path,sizeof(path),"%s/vendor/build.prop",root);f=fopen(path,"w");assert(f);fputs("ro.test=vendor\nro.long=",f);for(int i=0;i<160;i++)fputc('x',f);fputc('\n',f);fclose(f);
 assert(pixel_runtime_start(root)==0);
 get=pixel_runtime_hook("__system_property_get",NULL);find=pixel_runtime_hook("__system_property_find",NULL);callback=pixel_runtime_hook("__system_property_read_callback",NULL);assert(get&&find&&callback);
 char value[92];assert(get("missing",value)==0&&value[0]==0);assert(find("missing")==NULL);
 assert(find("ro.empty")!=NULL);callback(find("ro.long"),check,(void*)42);
 assert(get("ro.long",value)==0&&errno==EOVERFLOW&&value[0]==0);
 int (*set)(const char*,const char*)=pixel_runtime_hook("__system_property_set",NULL);assert(set&&set("ro.test","bad")==-1&&errno==EROFS);
 pthread_t ts[4];for(int i=0;i<4;i++)assert(pthread_create(&ts[i],NULL,thread,NULL)==0);for(int i=0;i<4;i++)assert(pthread_join(ts[i],NULL)==0);
 assert(pixel_runtime_hook("unknown_symbol",NULL)==NULL);
 assert(pixel_runtime_start(root)==-1&&errno==EALREADY);
 snprintf(path,sizeof(path),"%s/vendor/build.prop",root);unlink(path);snprintf(path,sizeof(path),"%s/system/build.prop",root);unlink(path);snprintf(path,sizeof(path),"%s/vendor",root);rmdir(path);snprintf(path,sizeof(path),"%s/system",root);rmdir(path);rmdir(root);
 puts("immutable property snapshot: missing input rollback, overrides, long values, read-only API, threads passed");
}
