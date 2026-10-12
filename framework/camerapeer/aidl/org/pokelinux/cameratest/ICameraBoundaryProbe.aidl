package org.pokelinux.cameratest;
import android.hardware.common.NativeHandle;
/** Synthetic infrastructure; no camera HAL or factory ABI identity. */
@VintfStability
interface ICameraBoundaryProbe {
    int submit(in NativeHandle buffer, int mode);
    NativeHandle returnedHandle();
    void release();
    int fdCount();
}
