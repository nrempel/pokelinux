package org.pokelinux.suspendtest;
import android.system.suspend.ISystemSuspend;
import android.system.suspend.IWakeLock;
import org.pokelinux.suspendtest.ISuspendControl;
interface ISuspendBoundaryProbe {
  void install(in ISystemSuspend service, in ISuspendControl control);
  ISystemSuspend service();
  ISuspendControl control();
  void retain(in IWakeLock wake);
  IWakeLock borrow();
  void dropRetained();
  int foreignAcquire();
  ISystemSuspend ndkService();
  long[] ndkSnapshot();
}
