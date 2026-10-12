package org.omarchy.bridge;
import java.io.IOException;
final class DialAttemptTest {
 static final String A="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",B="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
 static class Store implements DialAttempt.Store {
  String value="";boolean fail;
  public String load(){return value;}
  public void save(String token)throws IOException{if(fail)throw new IOException("storage unavailable");value=token;}
 }
 public static void main(String[] args)throws Exception{
  Store store=new Store();DialAttempt attempt=new DialAttempt(store);
  if(attempt.pending())throw new AssertionError("Initially pending");
  store.fail=true;
  try{attempt.reserve(A);throw new AssertionError("Storage failure admitted call");}catch(IOException expected){}
  if(!attempt.pending())throw new AssertionError("Ambiguous storage failure lost reservation");
  store.fail=false;attempt.terminal(A);attempt.reserve(A);
  if(!new DialAttempt(store).pending())throw new AssertionError("Restart forgot unresolved call");
  if(attempt.reserve(B))throw new AssertionError("Second call admitted before observation");
  attempt.terminal(B);if(!attempt.pending())throw new AssertionError("Unrelated call cleared pending");
  store.fail=true;
  try{attempt.terminal(A);throw new AssertionError("Clear failure hidden");}catch(IOException expected){}
  if(!attempt.pending())throw new AssertionError("Unpersisted clear enabled calls");
  store.fail=false;attempt.terminal(A);
  if(attempt.pending()||new DialAttempt(store).pending())throw new AssertionError("Terminal correlation did not resolve");
  if(!attempt.reserve(B))throw new AssertionError("Resolved call prevented new request");
  store.value="corrupt";
  try{new DialAttempt(store);throw new AssertionError("Malformed journal accepted");}catch(IOException expected){}
  System.out.println("DialAttempt: storage failure, restart, duplicate reservation and exact terminal correlation passed");
 }
}
