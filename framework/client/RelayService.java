package org.omarchy.probe;
import android.app.Service;
import android.content.Intent;
import android.os.*;
/** Adversarial test fixture: use a Binder handle obtained by an authorized caller. */
public final class RelayService extends Service {
 @Override public IBinder onBind(Intent i){return new Binder(){
  @Override protected boolean onTransact(int code,Parcel data,Parcel reply,int flags) throws RemoteException {
   IBinder victim=data.readStrongBinder();
   Parcel request=Parcel.obtain(),response=Parcel.obtain();
   String outcome="UNEXPECTED_SUCCESS";
   try{request.writeInterfaceToken("org.omarchy.bridge.v1");request.writeInt(1);victim.transact(1,request,response,0);response.readException();}
   catch(SecurityException e){outcome="SecurityException";}
   finally{request.recycle();response.recycle();}
   reply.writeNoException();reply.writeString(outcome);return true;
  }
 };}
}
