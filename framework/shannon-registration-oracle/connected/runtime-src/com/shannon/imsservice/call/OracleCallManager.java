package com.shannon.imsservice.call;

import com.shannon.imsservice.conn.ApnNetType;
import org.omarchy.audit.RegistrationConnectedOracle;

/** Authored external-call cut. Allocated with Unsafe; no original constructor runs. */
public final class OracleCallManager extends ImsCallManager {
    private OracleCallManager(int slot) {
        super(slot);
        throw new AssertionError("oracle call-manager constructor forbidden");
    }
    @Override public void updateAllCalls(ApnNetType apn) {
        RegistrationConnectedOracle.effect("calls.updateAllCalls", apn);
    }
    @Override public void updateHandoverStateForCall(int from, int to, boolean connected, ApnNetType apn) {
        RegistrationConnectedOracle.effect("calls.updateHandoverStateForCall", from, to, connected, apn);
    }
}
