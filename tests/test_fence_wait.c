/* SPDX-License-Identifier: MIT */
#include "fence_wait.h"
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>
static volatile sig_atomic_t signals;
static void tick(int n) { (void)n; ++signals; }
static double now(void) {struct timespec t;assert(clock_gettime(CLOCK_MONOTONIC,&t)==0);return t.tv_sec+t.tv_nsec/1e9;}
int main(void) {
 int p[2];assert(pipe(p)==0);
 assert(pixel_wait_fence(-1,1)==-1&&errno==EBADF);
 assert(pixel_wait_fence(p[0],-1)==-1&&errno==EINVAL);
 struct sigaction sa={0};sa.sa_handler=tick;sigemptyset(&sa.sa_mask);assert(sigaction(SIGALRM,&sa,NULL)==0);
 struct itimerval timer={{0,2000},{0,2000}};assert(setitimer(ITIMER_REAL,&timer,NULL)==0);
 double start=now();assert(pixel_wait_fence(p[0],40)==-1&&errno==ETIMEDOUT);double elapsed=now()-start;
 timer.it_interval.tv_usec=timer.it_value.tv_usec=0;assert(setitimer(ITIMER_REAL,&timer,NULL)==0);
 assert(signals>0&&elapsed>=0.03&&elapsed<0.5);
 assert(write(p[1],"x",1)==1);assert(pixel_wait_fence(p[0],0)==0);
 char c;assert(read(p[0],&c,1)==1);close(p[1]);assert(pixel_wait_fence(p[0],5)==-1&&errno==EIO);close(p[0]);
 puts("fence wait: timeout survives repeated signals; ready, closed and invalid fds passed");
}
