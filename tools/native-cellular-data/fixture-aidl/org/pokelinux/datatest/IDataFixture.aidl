package org.pokelinux.datatest;

import org.pokelinux.datatest.CaptureSetup;

// Separate generated fixture interface; ordinary production builds omit it.
@VintfStability
interface IDataFixture {
    android.hardware.radio.data.IRadioData endpoint();
    String generation();
    CaptureSetup capture();
    // Counts are exact; sticky duplicates/refusals fail the barrier.
    // untilBoottimeNanos is an unchanged caller deadline, at most 3s away.
    void barrier(int attachments, int setups, int responses, int indications,
                 long untilBoottimeNanos);
    // Only ordinary SOLICITED is accepted. No ACK_EXP obligation is synthesized.
    void setupResponse(in android.hardware.radio.RadioResponseInfo info,
                       in android.hardware.radio.data.SetupDataCallResult result);
    // Always emits ordinary UNSOLICITED, only after explicit fixture instruction.
    void listChanged(in android.hardware.radio.data.SetupDataCallResult[] calls);
    void die();
}
