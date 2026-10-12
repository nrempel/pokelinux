package org.pokelinux.sensortest;
/** Own app-facing permission fixture. Caller identity comes from Binder, never this payload. */
@VintfStability
interface ISensorClient {
    long subscribe(int claimedUid, int kind);
}
