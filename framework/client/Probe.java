package org.omarchy.probe;
import android.app.Instrumentation;
import android.content.*;
import android.os.*;
import java.util.concurrent.*;

/** Test-only client. Runs in its own application UID, never the bridge UID. */
public final class Probe extends Instrumentation {
    private Bundle args;
    @Override public void onCreate(Bundle arguments) { args=arguments; start(); }
    @Override public void onStart() {
        Bundle result=new Bundle();
        Context ctx=getTargetContext();
        CountDownLatch ready=new CountDownLatch(1);
        IBinder[] remote=new IBinder[1];
        ServiceConnection connection=new ServiceConnection() {
            public void onServiceConnected(ComponentName n,IBinder b){remote[0]=b;ready.countDown();}
            public void onServiceDisconnected(ComponentName n){remote[0]=null;}
            public void onNullBinding(ComponentName n){ready.countDown();}
        };
        boolean bound=false;
        try {
            String component=args.getString("component","org.omarchy.bridge/org.omarchy.bridge.BridgeService");
            Intent intent=new Intent().setComponent(ComponentName.unflattenFromString(component));
            bound=ctx.bindService(intent,connection,Context.BIND_AUTO_CREATE);
            if (!bound) throw new IllegalStateException("SERVICE_NOT_FOUND");
            if (!ready.await(10,TimeUnit.SECONDS)||remote[0]==null) throw new IllegalStateException("BIND_TIMEOUT");
            if ("true".equals(args.getString("relay"))) {
                CountDownLatch relayReady=new CountDownLatch(1);IBinder[] relay=new IBinder[1];
                ServiceConnection relayConnection=new ServiceConnection(){
                    public void onServiceConnected(ComponentName n,IBinder b){relay[0]=b;relayReady.countDown();}
                    public void onServiceDisconnected(ComponentName n){}
                };
                Intent ri=new Intent().setComponent(new ComponentName("org.omarchy.untrusted","org.omarchy.probe.RelayService"));
                if(!ctx.bindService(ri,relayConnection,Context.BIND_AUTO_CREATE))throw new IllegalStateException("RELAY_MISSING");
                try {
                    if(!relayReady.await(10,TimeUnit.SECONDS))throw new IllegalStateException("RELAY_TIMEOUT");
                    Parcel in=Parcel.obtain(),out=Parcel.obtain();
                    try{in.writeStrongBinder(remote[0]);relay[0].transact(1,in,out,0);out.readException();result.putString("value",out.readString());result.putString("outcome","RELAY");}
                    finally{in.recycle();out.recycle();}
                }finally{ctx.unbindService(relayConnection);}
                return;
            }
            Parcel request=Parcel.obtain(),reply=Parcel.obtain();
            try {
                request.writeInterfaceToken(args.getString("token","org.omarchy.bridge.v1"));
                request.writeInt(Integer.parseInt(args.getString("version","1")));
                int operation=Integer.parseInt(args.getString("operation","1"));
                if (!remote[0].transact(operation,request,reply,0)) throw new IllegalArgumentException("UNKNOWN_OPERATION");
                reply.readException();
                result.putString("value",reply.readString());
                result.putString("outcome","OK");
            } finally {request.recycle();reply.recycle();}
        } catch (Exception e) {
            result.putString("outcome",e.getClass().getSimpleName());
            result.putString("detail",e.getMessage());
        } finally {
            if(bound)ctx.unbindService(connection);
            result.putString("client_uid",Integer.toString(android.os.Process.myUid()));
            finish(0,result);
        }
    }
}
