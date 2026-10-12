package org.omarchy.bridge;
import java.io.IOException;
/** Main-thread outgoing reservation. Idle observations and timeouts never clear it. */
final class DialAttempt {
 interface Store {String load()throws IOException;void save(String token)throws IOException;}
 private final Store store;
 private String token;
 DialAttempt(Store store)throws IOException{
  this.store=store;token=store.load();
  if(token==null||(!token.isEmpty()&&!token.matches("[0-9a-f]{32}")))throw new IOException("Invalid outgoing journal");
 }
 boolean pending(){return !token.isEmpty();}
 boolean reserve(String next)throws IOException{
  if(next==null||!next.matches("[0-9a-f]{32}"))throw new IllegalArgumentException("Invalid attempt token");
  if(pending())return false;
  token=next; // A write can fail after changing disk; keep a conservative in-memory fence.
  store.save(next);return true;
 }
 void terminal(String observedToken)throws IOException{
  if(pending()&&token.equals(observedToken)){store.save("");token="";}
 }
}
