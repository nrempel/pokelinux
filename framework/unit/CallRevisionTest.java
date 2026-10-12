package org.omarchy.bridge;
import java.util.*;
public final class CallRevisionTest {
 static void require(boolean value){if(!value)throw new AssertionError();}
 public static void main(String[] args){
  CallRevision revision=new CallRevision();Map<Integer,String> observed=new HashMap<>();observed.put(1,"ringing:standalone");
  long first=revision.observe(observed);require(revision.observe(observed)==first);
  observed.put(1,"ringing:conference");require(revision.observe(observed)>first);
  long beforeLocal=revision.value();revision.changed();require(revision.value()>beforeLocal);
  long beforeCallback=revision.value();revision.changed();require(revision.value()>beforeCallback);
  long beforeRemoval=revision.value();observed.clear();require(revision.observe(observed)>beforeRemoval);
  System.out.println("CallRevision: live changes and local dispatch invalidate snapshots");
 }
}
