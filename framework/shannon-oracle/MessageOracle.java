package org.omarchy.audit;
/** Calls only original message constructors, setters and getBytes; never send. */
public final class MessageOracle {
 public static void main(String[] args) throws Exception {
  Class<?> channel=Class.forName("com.shannon.imsservice.network.NetworkAdaptor$AdaptorChannel");
  String[][] messages={
   {"RilReqStackStartReq","setSimStatus","setStackType"},
   {"RilReqSetImsRegistrationState","setRegistrationState","setRegisteredRat"}
  };
  int[] values={Integer.MIN_VALUE,-1,0,1,127,128,255,256,Integer.MAX_VALUE};
  for(String[] spec:messages){
   Class<?> type=Class.forName("com.shannon.imsservice.network.adaptor.data."+spec[0]);
   for(int a:values)for(int b:values){
    Object message=type.getConstructor(channel).newInstance(new Object[]{null});
    type.getMethod(spec[1],int.class).invoke(message,a);
    type.getMethod(spec[2],int.class).invoke(message,b);
    byte[] bytes=(byte[])type.getMethod("getBytes").invoke(message);
    StringBuilder hex=new StringBuilder();
    for(byte value:bytes)hex.append(String.format(java.util.Locale.ROOT,"%02x",value&255));
    System.out.println(spec[0]+"\t"+a+"\t"+b+"\t"+hex);
   }
  }
  // SolicitedMessage owns a HandlerThread; this disposable oracle process ends.
  System.exit(0);
 }
}
