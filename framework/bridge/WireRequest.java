package org.omarchy.bridge;

import android.util.JsonReader;
import android.util.JsonToken;
import java.io.*;
import java.nio.charset.*;
import java.util.*;

/** One strict JSON object. Field order is not part of the protocol. */
final class WireRequest {
 final String id,op,generation,action,offer,number;
 final int callId,state;
 final long revision;
 final String afterGeneration;
 final long afterRevision;
 final int waitMs;
 private WireRequest(Map<String,String> fields)throws IOException {
  if(!"1".equals(fields.get("v")))throw new IOException("Version");
  id=identity(fields.get("id"));op=fields.get("op");
  boolean watch="watch".equals(op);
  waitMs=watch?(int)bounded(fields.get("wait_ms"),0,25000):0;
  if(watch&&fields.containsKey("after_generation")!=fields.containsKey("after_revision"))throw new IOException("Cursor pair");
  afterGeneration=watch&&fields.containsKey("after_generation")?identity(fields.get("after_generation")):null;
  afterRevision=afterGeneration==null?0:bounded(fields.get("after_revision"),1,9007199254740991L);
  Set<String> expected=new HashSet<>(Arrays.asList("v","id","op"));
  if("status".equals(op)||"controls".equals(op)||"dial_status".equals(op)){
   generation=action=offer=number=null;callId=state=0;revision=0;
  }else if(watch){
   expected.add("wait_ms");
   if(afterGeneration!=null)expected.addAll(Arrays.asList("after_generation","after_revision"));
   generation=action=offer=number=null;callId=state=0;revision=0;
  }else if("control".equals(op)){
   expected.addAll(Arrays.asList("generation","call_id","expected_revision","expected_state","action"));
   generation=identity(fields.get("generation"));
   callId=(int)bounded(fields.get("call_id"),1,2147483647L);
   revision=bounded(fields.get("expected_revision"),1,9007199254740991L);
   state=(int)bounded(fields.get("expected_state"),0,20);
   action=fields.get("action");
   if(!Arrays.asList("answer","reject","disconnect").contains(action))throw new IOException("Action");
   offer=number=null;
  }else if("dial".equals(op)){
   expected.addAll(Arrays.asList("generation","offer","number"));
   generation=identity(fields.get("generation"));offer=identity(fields.get("offer"));
   number=fields.get("number");
   if(number==null||!number.matches("[+]?[0-9]{3,32}"))throw new IOException("Number");
   action=null;callId=state=0;revision=0;
  }else throw new IOException("Operation");
  if(!fields.keySet().equals(expected))throw new IOException("Fields");
 }
 private static String identity(String value)throws IOException {
  if(value==null||!value.matches("[0-9a-f]{32}"))throw new IOException("Identity");
  return value;
 }
 private static long bounded(String value,long minimum,long maximum)throws IOException {
  if(value==null||!value.matches("0|[1-9][0-9]*"))throw new IOException("Integer");
  try{long number=Long.parseLong(value);if(number<minimum||number>maximum)throw new IOException("Bounds");return number;}
  catch(NumberFormatException e){throw new IOException("Integer bounds");}
 }
 static WireRequest parse(byte[] bytes)throws IOException {
  if(bytes.length<1||bytes.length>4096)throw new IOException("Frame size");
  Set<String> numeric=new HashSet<>(Arrays.asList("v","call_id","expected_revision","expected_state","wait_ms","after_revision"));
  Set<String> strings=new HashSet<>(Arrays.asList("id","op","generation","action","offer","number","after_generation"));
  Map<String,String> fields=new HashMap<>();
  CharsetDecoder decoder=StandardCharsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT).onUnmappableCharacter(CodingErrorAction.REPORT);
  try(JsonReader reader=new JsonReader(new InputStreamReader(new ByteArrayInputStream(bytes),decoder))){
   reader.setLenient(false);reader.beginObject();
   while(reader.hasNext()){
    String name=reader.nextName();
    if(fields.containsKey(name))throw new IOException("Duplicate field");
    boolean isNumber=numeric.contains(name);
    if(!isNumber&&!strings.contains(name))throw new IOException("Unknown field");
    if(reader.peek()!=(isNumber?JsonToken.NUMBER:JsonToken.STRING))throw new IOException("Field type");
    String value=reader.nextString();
    if(isNumber&&!value.matches("0|[1-9][0-9]*"))throw new IOException("Integer");
    fields.put(name,value);
   }
   reader.endObject();
   if(reader.peek()!=JsonToken.END_DOCUMENT)throw new IOException("Trailing data");
  }catch(IllegalStateException e){throw new IOException("Request structure");}
  return new WireRequest(fields);
 }
}
