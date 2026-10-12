package org.omarchy.bridge;
import java.util.*;

/** Main-looper owned current state. Episode IDs never authorize a call action. */
final class RingingRegistry {
 static final class Snapshot {
  final String generation,sourceState;
  final long revision;
  final Map<Integer,String> ringing;
  Snapshot(String generation,String sourceState,long revision,Map<Integer,String> ringing){
   this.generation=generation;this.sourceState=sourceState;this.revision=revision;
   this.ringing=Collections.unmodifiableMap(new TreeMap<>(ringing));
  }
 }
 private Snapshot current;
 // A queued state callback can carry an earlier departure while current Call
 // details already show the next ringing period. Retire that old episode first.
 void leftRinging(String generation,int id){
  if(current==null||!current.generation.equals(generation)||!current.ringing.containsKey(id))return;
  Set<Integer> remaining=new HashSet<>(current.ringing.keySet());remaining.remove(id);
  observe(generation,current.sourceState,remaining);
 }
 Snapshot observe(String generation,String sourceState,Set<Integer> ringing){
  if(generation==null||!generation.matches("[0-9a-f]{32}")
    ||!Arrays.asList("observed","detached","unavailable").contains(sourceState)
    ||ringing==null||ringing.size()>64
    ||(!"observed".equals(sourceState)&&!ringing.isEmpty()))throw new IllegalArgumentException("Invalid ringing state");
  for(Integer id:ringing)if(id==null||id<1)throw new IllegalArgumentException("Invalid call identity");
  boolean sameGeneration=current!=null&&generation.equals(current.generation);
  if(sameGeneration&&sourceState.equals(current.sourceState)&&ringing.equals(current.ringing.keySet()))return current;
  long revision=sameGeneration?current.revision+1:1;
  if(revision>9007199254740991L)throw new IllegalStateException("Ringing revision exhausted");
  Map<Integer,String> episodes=new TreeMap<>();
  for(Integer id:ringing){
   String episode=sameGeneration?current.ringing.get(id):null;
   if(episode==null)episode=UUID.randomUUID().toString().replace("-", "");
   episodes.put(id,episode);
  }
  current=new Snapshot(generation,sourceState,revision,episodes);
  return current;
 }
}
