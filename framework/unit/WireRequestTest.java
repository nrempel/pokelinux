package org.omarchy.bridge;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
public final class WireRequestTest {
 static final String BASE="\"v\":1,\"id\":\"11111111111111111111111111111111\",\"op\":\"watch\"";
 static WireRequest parse(String text)throws IOException{return WireRequest.parse(text.getBytes(StandardCharsets.UTF_8));}
 static void reject(String text)throws IOException{try{parse(text);throw new AssertionError("accepted invalid request: "+text);}catch(IOException expected){}}
 static void require(boolean value){if(!value)throw new AssertionError();}
 public static void main(String[] args)throws Exception{
  WireRequest first=parse("{"+BASE+",\"wait_ms\":25000}");
  require(first.op.equals("watch"));
  // Read fields only after the behavioral red test accepts the new operation.
  require(first.waitMs==25000&&first.afterGeneration==null&&first.afterRevision==0);
  WireRequest next=parse("{"+BASE+",\"wait_ms\":0,\"after_generation\":\"22222222222222222222222222222222\",\"after_revision\":9007199254740991}");
  require(next.waitMs==0&&next.afterRevision==9007199254740991L);
  for(String suffix:new String[]{"",",\"wait_ms\":-1",",\"wait_ms\":25001",",\"wait_ms\":1.0",",\"wait_ms\":1e3",",\"wait_ms\":\"0\"",",\"wait_ms\":true",",\"wait_ms\":null",",\"wait_ms\":0,\"wait_ms\":1",",\"wait_ms\":0,\"after_revision\":1",",\"wait_ms\":0,\"after_generation\":\"22222222222222222222222222222222\"",",\"wait_ms\":0,\"action\":\"answer\""})reject("{"+BASE+suffix+"}");
  for(String revision:new String[]{"0","9007199254740992","-1","1.5","\"1\""})reject("{"+BASE+",\"wait_ms\":0,\"after_generation\":\"22222222222222222222222222222222\",\"after_revision\":"+revision+"}");
  reject("{"+BASE+",\"wait_ms\":0} {}");reject("{"+BASE+",\"wait_ms\":0,}");
  reject("{"+BASE.replace("watch","status")+",\"wait_ms\":0}");
  require(parse("{"+BASE.replace("watch","status")+"}").op.equals("status"));
  System.out.println("WireRequest: bounded watch, paired cursor, strict JSON and command-field rejection passed");
 }
}
