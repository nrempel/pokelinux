package org.omarchy.bridge;
import android.content.*;
/** Explicit immutable notification action, not exported. */
public final class StopReceiver extends BroadcastReceiver {
 @Override public void onReceive(Context context,Intent intent){GatewayService.disable(context);}
}
