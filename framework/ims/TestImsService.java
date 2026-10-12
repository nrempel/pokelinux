package org.omarchy.imsfixture;
import android.telephony.ims.ImsService;
import android.telephony.ims.feature.*;
import android.telephony.ims.stub.*;
import android.util.Log;
/** Contract fixture only. No modem, registration, emergency service, or media implementation. */
public final class TestImsService extends ImsService {
 private static final java.util.concurrent.Executor EXECUTOR=new java.util.concurrent.Executor(){public void execute(Runnable action){action.run();}};
 @Override public void onCreate(){super.onCreate();Log.i("OmarchyIms","SERVICE_CREATE");}
 @Override public java.util.concurrent.Executor getExecutor(){return EXECUTOR;}
 @Override public ImsFeatureConfiguration querySupportedImsFeatures(){
  Log.i("OmarchyIms","QUERY_FEATURES");
  return new ImsFeatureConfiguration.Builder().addFeature(0,ImsFeature.FEATURE_MMTEL).build();
 }
 @Override public void readyForFeatureCreation(){Log.i("OmarchyIms","READY_FOR_FEATURE_CREATION");}
 @Override public MmTelFeature createMmTelFeatureForSubscription(int slot,int sub){
  Log.i("OmarchyIms","CREATE_MMTEL slot="+slot+" sub="+sub);
  MmTelFeature feature=new MmTelFeature(EXECUTOR){
   @Override public void onFeatureReady(){Log.i("OmarchyIms","FEATURE_READY_CALLBACK");}
   @Override public void onFeatureRemoved(){Log.i("OmarchyIms","FEATURE_REMOVED_CALLBACK");}
  };
  feature.setFeatureState(ImsFeature.STATE_READY);
  return feature;
 }
 @Override public ImsConfigImplBase getConfigForSubscription(int slot,int sub){return new ImsConfigImplBase(EXECUTOR);}
 @Override public ImsRegistrationImplBase getRegistrationForSubscription(int slot,int sub){return new ImsRegistrationImplBase(EXECUTOR);}
 @Override public void onDestroy(){Log.i("OmarchyIms","SERVICE_DESTROY");super.onDestroy();}
}
