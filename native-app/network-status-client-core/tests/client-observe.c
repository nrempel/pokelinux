#define _GNU_SOURCE
#include "pokelinux_network_status_client.h"
#include <errno.h>
#include <assert.h>
#include <inttypes.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>
_Static_assert(sizeof(PrNetworkStatusViewV1) == 24, "view size");
_Static_assert(offsetof(PrNetworkStatusViewV1, revision) == 16, "revision offset");
static uint64_t now_ns(void) {
    struct timespec t;
    if (clock_gettime(CLOCK_BOOTTIME, &t) || t.tv_sec < 0 || t.tv_nsec < 0 || t.tv_nsec >= 1000000000) exit(70);
    return (uint64_t)t.tv_sec * UINT64_C(1000000000) + (uint64_t)t.tv_nsec;
}
int main(void) {
    if (getuid()!=11050 || geteuid()!=11050 || getgid()!=11050 || getegid()!=11050) return 70;
    FILE *status=fopen("/proc/self/status","r"); if (!status) return 70;
    char line[1024]; int capless=0;
    while (fgets(line,sizeof line,status)) if (!strncmp(line,"CapEff:",7)) capless=strtoull(line+7,NULL,16)==0;
    fclose(status); if (!capless) return 70;
    struct stat ns; if (stat("/proc/thread-self/ns/net",&ns)) return 70;
    PrNetworkStatusClient *client=NULL;
    if (pr_network_status_client_new_v1(&client)!=PR_NETWORK_STATUS_OK || !client) return 71;
    PrNetworkStatusViewV1 v={0};
    if (pr_network_status_client_view_v1(client,&v)!=PR_NETWORK_STATUS_OK) return 71;
    int initial=(v.available==0 && v.local_interfaces==-1 && v.address_records==-1 && v.revision==0);
    uint64_t start=now_ns(), until=start+UINT64_C(2000000000),ticks=0,max_tick_ns=0;
    int matched=0,api_ok=1;
    while (now_ns()<until) {
        uint64_t before=now_ns();
        if (pr_network_status_client_tick_v1(client)!=PR_NETWORK_STATUS_OK || pr_network_status_client_view_v1(client,&v)!=PR_NETWORK_STATUS_OK) { api_ok=0; break; }
        uint64_t after=now_ns(); if (after-before>max_tick_ns) max_tick_ns=after-before; ++ticks;
        if (v.available==1 && v.local_interfaces==0 && v.address_records==0 && v.revision>0 && after<until) { matched=1; break; }
        struct timespec pause={0,2000000}; while (nanosleep(&pause,&pause)<0 && errno==EINTR) {}
    }
    pr_network_status_client_free_v1(client); pr_network_status_client_free_v1(NULL);
    char body[2048];
    int length=snprintf(body,sizeof body,"{\"schema\":1,\"consumer_uid\":11050,\"capabilities\":0,\"namespace_dev\":%" PRIuMAX ",\"namespace_ino\":%" PRIuMAX ",\"initial_unavailable_valid\":%s,\"API_calls_OK\":%s,\"healthy_empty_observed\":%s,\"available\":%u,\"local_interfaces\":%d,\"address_records\":%d,\"revision\":%" PRIu64 ",\"ticks\":%" PRIu64 ",\"maximum_tick_ns\":%" PRIu64 ",\"original_until_ns\":%" PRIu64 ",\"completed_ns\":%" PRIu64 "}\n",(uintmax_t)ns.st_dev,(uintmax_t)ns.st_ino,initial?"true":"false",api_ok?"true":"false",matched?"true":"false",v.available,v.local_interfaces,v.address_records,v.revision,ticks,max_tick_ns,until,now_ns());
    if (length<=0 || (size_t)length>=sizeof body) return 72;
    FILE *record=fopen("/out/client-observation.part","wx"); if (!record) return 72;
    if (fchmod(fileno(record),0644)) return 72;
    if (fwrite(body,1,(size_t)length,record)!=(size_t)length || fclose(record)) return 72;
    if (rename("/out/client-observation.part","/out/client-observation.json")) return 72;
    fputs(body,stdout); fflush(stdout);
    /* Observer writes this marker only after actual daemon Main0 + PIPE EOF. */
    uint64_t close_until=start+UINT64_C(5000000000);
    while (access("/out/daemon-stopped.json",F_OK)!=0 && now_ns()<close_until) {
        struct timespec pause={0,2000000}; nanosleep(&pause,NULL);
    }
    assert(access("/out/daemon-stopped.json",F_OK)==0 && "daemon cleanup must precede client assertion");
    assert(initial && api_ok && matched && "actual Rust client must observe healthy empty 0/0");
    return 0;
}
