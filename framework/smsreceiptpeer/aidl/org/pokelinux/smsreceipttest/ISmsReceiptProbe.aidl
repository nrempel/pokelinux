package org.pokelinux.smsreceipttest;
import android.hardware.radio.messaging.IRadioMessaging;
@VintfStability
interface ISmsReceiptProbe {
    IRadioMessaging endpoint(int slot);
    void barrier(int slot, int registrations, int acks);
    void incoming(int slot, int kind, in byte[] pdu, int mode, boolean previous);
    void intermediate(int slot, int serial);
    void finalResponse(int slot, int serial, int mode, boolean previous);
    int ackCount(int slot);
    int capturedSerial(int slot);
    boolean capturedSuccess(int slot);
    int capturedCause(int slot);
    void die();
}
