package org.omarchy.bridge;
import java.io.IOException;
import java.util.*;
import java.util.function.LongSupplier;
/** Main-thread outgoing ledger, separate from incoming controls. Never retries Telecom. */
final class DialPolicy {
 interface Phone {
  String unavailable();Object account();long revision();String classify(String number);
  boolean commitAllowed();void dispatch(String number,Object account,String attempt);
 }
 private static final class Offer {
  final long revision,expires;final Object account;
  Offer(long revision,long expires,Object account){this.revision=revision;this.expires=expires;this.account=account;}
 }
 private static final class Entry {
  final String payload;String result="outcome_unknown:dispatch_error";
  Entry(String payload){this.payload=payload;}
 }
 private final String generation;
 private final int capacity,offerCapacity;
 private final DialAttempt attempt;
 private final LongSupplier clock;
 private final Map<String,Offer> offers=new HashMap<>();
 private final Map<String,Entry> ledger=new HashMap<>();
 DialPolicy(String generation,int capacity,int offerCapacity,DialAttempt attempt,LongSupplier clock){
  this.generation=generation;this.capacity=capacity;this.offerCapacity=offerCapacity;this.attempt=attempt;this.clock=clock;
 }
 boolean pending(){return attempt.pending();}
 void terminal(String token)throws IOException{attempt.terminal(token);}
 String offer(Phone phone){
  long now=clock.getAsLong();
  for(Iterator<Offer> entries=offers.values().iterator();entries.hasNext();)if(now>=entries.next().expires)entries.remove();
  if(pending()||phone.unavailable()!=null||offers.size()>=offerCapacity)return "";
  Object account=phone.account();if(account==null)return "";
  String token=UUID.randomUUID().toString().replace("-","");
  offers.put(token,new Offer(phone.revision(),now+5000,account));return token;
 }
 private String check(Phone phone,Offer offer,String number){
  String reason=phone.unavailable();if(reason!=null)return reason;
  if(offer==null)return "stale_offer";
  if(phone.revision()!=offer.revision)return "stale_revision";
  if(!offer.account.equals(phone.account()))return "account_changed";
  reason=phone.classify(number);if(reason!=null)return reason;
  return clock.getAsLong()>=offer.expires?"deadline":null;
 }
 String execute(Phone phone,String id,String expectedGeneration,String token,String number){
  if(id==null||!id.matches("[0-9a-f]{32}")||token==null||!token.matches("[0-9a-f]{32}")||!DialNumber.valid(number))return "not_invoked:invalid_request";
  if(!generation.equals(expectedGeneration))return "not_invoked:stale_generation";
  String payload=expectedGeneration+":"+token+":"+number;
  Entry previous=ledger.get(id);
  if(previous!=null)return previous.payload.equals(payload)?previous.result:"not_invoked:replay_conflict";
  if(ledger.size()>=capacity)return "not_invoked:capacity";
  Entry entry=new Entry(payload);ledger.put(id,entry);
  String marker=null;boolean entered=false;
  try{
   if(pending()){entry.result="not_invoked:pending";return entry.result;}
   Offer offer=offers.get(token);String reason=check(phone,offer,number);
   if(reason!=null){entry.result="not_invoked:"+reason;return entry.result;}
   marker=UUID.randomUUID().toString().replace("-","");
   if(!attempt.reserve(marker)){entry.result="not_invoked:pending";return entry.result;}
   reason=check(phone,offer,number); // Persistence may consume the offer lifetime or change authority.
   if(reason==null&&(!phone.commitAllowed()||clock.getAsLong()>=offer.expires))reason="deadline";
   if(reason!=null){entry.result="not_invoked:"+reason;attempt.terminal(marker);return entry.result;}
   entered=true;phone.dispatch(number,offer.account,marker);entry.result="invocation_attempted:returned";
  }catch(IOException e){
   entry.result="not_invoked:storage_error";
   if(marker!=null)try{attempt.terminal(marker);}catch(IOException ignored){}
  }catch(RuntimeException e){
   entry.result=entered?"outcome_unknown:dispatch_error":"not_invoked:unavailable";
   if(!entered&&marker!=null)try{attempt.terminal(marker);}catch(IOException ignored){}
  }
  return entry.result;
 }
}
