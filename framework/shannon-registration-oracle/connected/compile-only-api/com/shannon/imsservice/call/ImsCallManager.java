package com.shannon.imsservice.call;

import com.shannon.imsservice.conn.ApnNetType;

/** Compile-only recovered API declarations. NEVER include in DEX/runtime classpath. */
public class ImsCallManager {
    ImsCallManager(int slot) { throw new UnsupportedOperationException("compile-only API"); }
    public void updateAllCalls(ApnNetType apn) { throw new UnsupportedOperationException("compile-only API"); }
    public void updateHandoverStateForCall(int from, int to, boolean connected, ApnNetType apn) {
        throw new UnsupportedOperationException("compile-only API");
    }
}
