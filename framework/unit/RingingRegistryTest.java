package org.omarchy.bridge;
import java.util.*;
public final class RingingRegistryTest {
 static void require(boolean value){if(!value)throw new AssertionError();}
 static Set<Integer> ids(Integer... values){return new HashSet<>(Arrays.asList(values));}
 static void rejected(Runnable action){try{action.run();throw new AssertionError("accepted invalid state");}catch(IllegalArgumentException expected){}}
 public static void main(String[] args){
  RingingRegistry registry=new RingingRegistry();
  String generation="11111111111111111111111111111111";
  RingingRegistry.Snapshot first=registry.observe(generation,"observed",ids(7,8));
  String episode=first.ringing.get(7);
  require(episode.matches("[0-9a-f]{32}"));require(!episode.equals(first.ringing.get(8)));
  require(registry.observe(generation,"observed",ids(8,7)).revision==first.revision);
  RingingRegistry.Snapshot removed=registry.observe(generation,"observed",ids(8));
  require(removed.revision>first.revision);require(first.ringing.containsKey(7));
  RingingRegistry.Snapshot again=registry.observe(generation,"observed",ids(7,8));
  require(!again.ringing.get(7).equals(episode));require(again.ringing.get(8).equals(first.ringing.get(8)));
  try{first.ringing.clear();throw new AssertionError("mutable snapshot");}catch(UnsupportedOperationException expected){}
  RingingRegistry.Snapshot detached=registry.observe(generation,"detached",ids());
  require(detached.ringing.isEmpty());require(detached.revision>again.revision);
  RingingRegistry.Snapshot unavailable=registry.observe(generation,"unavailable",ids());
  require(unavailable.revision>detached.revision);
  RingingRegistry.Snapshot rebound=registry.observe("22222222222222222222222222222222","observed",ids(7));
  require(!rebound.ringing.get(7).equals(again.ringing.get(7)));
  rejected(()->registry.observe(generation,"detached",ids(7)));
  rejected(()->registry.observe(generation,"unavailable",ids(7)));
  rejected(()->registry.observe(generation,"unknown",ids()));
  rejected(()->registry.observe("bad","observed",ids()));
  rejected(()->registry.observe(generation,"observed",ids(0)));
  rejected(()->registry.observe(generation,"observed",ids(-1)));
  Set<Integer> tooMany=new HashSet<>();for(int i=1;i<=65;i++)tooMany.add(i);
  rejected(()->registry.observe(generation,"observed",tooMany));
  require(registry.observe(rebound.generation,"observed",ids(7)).revision==rebound.revision);
  RingingRegistry queued=new RingingRegistry();
  RingingRegistry.Snapshot before=queued.observe(generation,"observed",ids(7,8));
  // Android's latest details already say RINGING again while an earlier ACTIVE
  // callback is being delivered. Preserve the departure carried by that event.
  queued.leftRinging(generation,7);
  RingingRegistry.Snapshot after=queued.observe(generation,"observed",ids(7,8));
  require(!after.ringing.get(7).equals(before.ringing.get(7)));
  require(after.ringing.get(8).equals(before.ringing.get(8)));
  require(after.revision>before.revision);
  queued.leftRinging("22222222222222222222222222222222",7);
  queued.leftRinging(generation,99);
  require(queued.observe(generation,"observed",ids(7,8)).revision==after.revision);
  System.out.println("RingingRegistry: stable snapshots, new episodes, source changes, bounds and immutable results");
 }
}
