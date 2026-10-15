package com.shannon.imsservice.conn;

/** Compile-only recovered API declarations. NEVER include this class or its nested enum in DEX. */
public class ImsConnectivityMgr {
    public enum ImsTriggerReason { }
    public ImsConnectivityMgr(int slot) { throw new UnsupportedOperationException("compile-only API"); }
    boolean isRcsEnabled() { throw new UnsupportedOperationException("compile-only API"); }
    boolean triggerRegistering(ImsTriggerReason reason) { throw new UnsupportedOperationException("compile-only API"); }
}
