package org.omarchy.bridge;
import java.util.*;
/** Main-looper observation fence. It does not claim ordering of unseen Telecom events. */
final class CallRevision {
 private long value=1;
 private Map<?,?> observed=Collections.emptyMap();
 long value(){return value;}
 void changed(){value++;}
 long observe(Map<?,?> current){
  if(!observed.equals(current)){observed=new HashMap<>(current);changed();}
  return value;
 }
}
