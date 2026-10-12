package org.pokelinux.suspendtest;
interface ISuspendControl {
  long[] snapshot();
  String callerSid();
  void holdRelease(boolean hold);
  void arm(in IBinder authority);
  int attempt();
  void die();
}
