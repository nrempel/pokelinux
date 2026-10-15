package com.shannon.imsservice.conn;

import org.omarchy.audit.RegistrationConnectedOracle;

/** Authored peripheral cut. Allocated with Unsafe; this constructor must never run. */
public final class OracleConnectivityMgr extends ImsConnectivityMgr {
    private OracleConnectivityMgr(int slot) {
        super(slot);
        throw new AssertionError("oracle owner constructor forbidden");
    }
    @Override boolean isRcsEnabled() {
        RegistrationConnectedOracle.effect("owner.isRcsEnabled");
        return false;
    }
    @Override boolean triggerRegistering(ImsTriggerReason reason) {
        RegistrationConnectedOracle.effect("owner.triggerRegistering", reason);
        return false;
    }
}
