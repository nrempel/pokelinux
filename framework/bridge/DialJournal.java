package org.omarchy.bridge;
import android.content.Context;
import android.util.AtomicFile;
import java.io.*;
import java.nio.charset.StandardCharsets;
/** Stores only the random unresolved-attempt token, never a telephone number. */
final class DialJournal implements DialAttempt.Store {
 private final AtomicFile file;
 DialJournal(Context context){file=new AtomicFile(new File(context.getNoBackupFilesDir(),"outgoing-attempt"));}
 public String load()throws IOException{
  try(InputStream input=file.openRead()){
   byte[] value=new byte[33];int length=0,count;
   while(length<value.length&&(count=input.read(value,length,value.length-length))!=-1)length+=count;
   if(length>32)throw new IOException("Outgoing journal too large");
   return new String(value,0,length,StandardCharsets.US_ASCII);
  }catch(FileNotFoundException e){
   if(file.getBaseFile().exists())throw e;
   return "";
  }
 }
 public void save(String token)throws IOException{
  if(token==null||(!token.isEmpty()&&!token.matches("[0-9a-f]{32}")))throw new IOException("Invalid outgoing journal token");
  FileOutputStream output=null;
  try{
   output=file.startWrite();output.write(token.getBytes(StandardCharsets.US_ASCII));output.getFD().sync();
   file.finishWrite(output);output=null;
   if(!load().equals(token))throw new IOException("Outgoing journal commit failed");
  }catch(IOException e){if(output!=null)file.failWrite(output);throw e;}
 }
}
