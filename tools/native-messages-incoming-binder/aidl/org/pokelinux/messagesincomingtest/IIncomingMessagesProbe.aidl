package org.pokelinux.messagesincomingtest;
import android.hardware.radio.messaging.IRadioMessaging;
@VintfStability
interface IIncomingMessagesProbe {
    IRadioMessaging endpoint(int slot);
    String generation();
    void barrier(int slot, int registrations, int callbacks, int timeoutMillis);
    void incoming(int slot, int kind, in byte[] pdu, int indicationType);
    int[] counters(int slot);
    void die();
}
