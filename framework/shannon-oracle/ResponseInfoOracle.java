package org.omarchy.audit;

import android.os.Parcel;
import android.os.Parcelable;

/** Records the original vendor Parcelable serializer, without any radio service. */
public final class ResponseInfoOracle {
    public static void main(String[] args) throws Exception {
        Class<?> type = Class.forName("vendor.samsung_slsi.telephony.hardware.radioExternal.RadioExternalResponseInfo");
        String[] fields = {"slotId", "serial", "rilcMsgId", "length", "error"};
        int[] values = {0, 1, -1, Integer.MIN_VALUE, Integer.MAX_VALUE, 123456, -16777216};
        for (int row = 0; row < values.length; row++) {
            Parcelable info = (Parcelable) type.getConstructor().newInstance();
            StringBuilder output = new StringBuilder("responseInfo");
            for (int i = 0; i < fields.length; i++) {
                int value = values[(row + i) % values.length];
                type.getField(fields[i]).setInt(info, value);
                output.append('\t').append(value);
            }
            Parcel parcel = Parcel.obtain();
            try {
                parcel.writeTypedObject(info, 0);
                byte[] bytes = parcel.marshall();
                if (bytes.length != 28) throw new AssertionError("unexpected metadata size");
                output.append('\t');
                for (byte value : bytes) output.append(String.format(java.util.Locale.ROOT, "%02x", value & 255));
                System.out.println(output);
            } finally {
                parcel.recycle();
            }
        }
    }
}
