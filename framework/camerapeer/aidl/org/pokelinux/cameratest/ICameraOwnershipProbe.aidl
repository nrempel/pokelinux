package org.pokelinux.cameratest;
import android.hardware.camera.device.ICameraDeviceCallback;
import android.hardware.camera.device.CaptureRequest;
import android.hardware.camera.device.BufferCache;
/** Original synthetic controls; data and callback interface are exact frozen camera V4. */
@VintfStability
interface ICameraOwnershipProbe {
    void setCallback(in ICameraDeviceCallback callback);
    int processCaptureRequest(in CaptureRequest[] requests, in BufferCache[] cachesToRemove, int mode);
    void returnResult(int frameNumber, int behavior);
    void signalFence(int frameNumber);
    boolean held();
    void releaseHold();
    void releaseAll();
    int fdCount();
    void die();
}
