package org.omarchy.audit;
import java.lang.reflect.Method;
/** Executes the original factory APK's serializer. No service or modem calls. */
public final class CodecOracle {
 public static void main(String[] args) throws Exception {
  Class<?> type=Class.forName("com.shannon.imsservice.network.oem.io.DataWriter");
  int[] values={Integer.MIN_VALUE,-65537,-32769,-256,-129,-1,0,1,127,128,255,256,32767,65535,Integer.MAX_VALUE};
  for(String name:new String[]{"writeByte","writeShort","writeInt"}) {
   Method write=type.getMethod(name,int.class);
   for(int value:values) {
    Object writer=type.getConstructor().newInstance();
    write.invoke(writer,value);
    byte[] bytes=(byte[])type.getMethod("toByteArray").invoke(writer);
    StringBuilder hex=new StringBuilder();
    for(byte b:bytes)hex.append(String.format(java.util.Locale.ROOT,"%02x",b&255));
    System.out.println(name+"\t"+value+"\t"+hex);
   }
  }
 }
}
