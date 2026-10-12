package org.pokelinux.smstest;
import android.hardware.radio.messaging.IRadioMessaging;
@VintfStability
interface ISmsBoundaryProbe {
    IRadioMessaging endpoint(int slot);
    void barrier(int slot, int registrations, int sends);
    void acknowledgeRequest(int slot, int serial);
    void finalResponse(int slot, int serial, int mode, boolean previous);
    int sendCount(int slot);
    int capturedSerial(int slot);
    String capturedPdu(int slot);
    String capturedSmsc(int slot);
    void die();
}
