package org.omarchy.bridge;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.*;
/** One request's atomic admission boundary. Peer closure is not a cancellation acknowledgment. */
final class Admission {
 private final AtomicInteger state=new AtomicInteger();
 private final LongSupplier elapsed,wall;
 private final BooleanSupplier closed;
 private final long deadline,expires;
 Admission(LongSupplier elapsed,LongSupplier wall,BooleanSupplier closed,long deadline,long expires){this.elapsed=elapsed;this.wall=wall;this.closed=closed;this.deadline=deadline;this.expires=expires;}
 private boolean valid(){return !closed.getAsBoolean()&&elapsed.getAsLong()<deadline&&wall.getAsLong()<expires;}
 boolean commit(){if(!valid()||!state.compareAndSet(0,1))return false;if(!valid()){state.set(2);return false;}return true;}
 void cancel(){state.compareAndSet(0,2);}
}
