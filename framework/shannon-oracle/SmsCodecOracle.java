package org.omarchy.audit;

import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;

/** Actual AOSP encoder execution only: no service discovery or SMS submission. */
public final class SmsCodecOracle {
    private static String hex(byte[] data) {
        if (data.length == 0) return "-";
        StringBuilder output = new StringBuilder();
        for (byte value : data) output.append(String.format("%02x", value & 255));
        return output.toString();
    }
    private static String repeat(String value, int count) {
        StringBuilder result = new StringBuilder();
        for (int i = 0; i < count; i++) result.append(value);
        return result.toString();
    }
    private static byte[] concat(int reference, int total, int sequence) {
        return new byte[]{8, 4, (byte)(reference >> 8), (byte)reference, (byte)total, (byte)sequence};
    }
    private static Method encoder;
    private static void sample(String name, String recipient, String text, boolean report, byte[] header) throws Exception {
        // Encoding 1 requests the default GSM alphabet; the original encoder
        // falls back to its UTF-16 implementation if the text is not representable.
        sample(name, recipient, text, report, header, 1);
    }
    private static void sample(String name, String recipient, String text, boolean report, byte[] header, int encoding) throws Exception {
        Object pdu = encoder.invoke(null, null, recipient, text, report, header, encoding, 0, 0);
        if (pdu == null) throw new AssertionError("original encoder refused fixture: " + name);
        byte[] message = (byte[])pdu.getClass().getField("encodedMessage").get(pdu);
        System.out.println("sms-submit\t" + name + "\t" + recipient + "\t" + (report ? 1 : 0)
            + "\t" + (header == null ? "-" : hex(header)) + "\t" + hex(text.getBytes(StandardCharsets.UTF_8))
            + "\t" + hex(message) + "\tPASS");
    }
    public static void main(String[] args) throws Exception {
        Class<?> original = Class.forName("com.android.internal.telephony.gsm.SmsMessage");
        encoder = original.getMethod("getSubmitPdu", String.class, String.class, String.class,
            boolean.class, byte[].class, int.class, int.class, int.class);
        String phone = "+15550100";
        sample("empty", phone, "", false, null);
        sample("hello", phone, "hello", false, null);
        sample("report", phone, "hello", true, null);
        sample("alphabet", phone, "@£$¥èéùìòÇØøÅåΔ_ΦΓΛΩΠΨΣΘΞÆæßÉ¤¡ÄÖÑÜ§¿äöñüà", false, null);
        sample("extension", phone, "^{}\\[~]|€", false, null);
        sample("controls", phone, "\n\r\f", false, null);
        sample("tab", phone, "\t", false, null);
        sample("chinese", phone, "你好世界", false, null);
        sample("cyrillic", phone, "Привет", false, null);
        sample("emoji", phone, "hello 😀", false, null);
        sample("combining", phone, "e\u0301", false, null);
        sample("arabic", phone, "مرحبا", false, null);
        sample("odd-international", "+1555011", "hello", false, null);
        sample("odd-national", "5550101", "hello", false, null);
        sample("gsm-limit", phone, repeat("A", 160), false, null);
        sample("unicode-limit", phone, repeat("中", 70), false, null);
        sample("concat-gsm-first", phone, repeat("A", 152), false, concat(0x1234, 2, 1));
        sample("concat-gsm-second", phone, repeat("A", 9), false, concat(0x1234, 2, 2));
        sample("concat-unicode-first", phone, repeat("中", 66), false, concat(0x1234, 2, 1));
        sample("concat-unicode-second", phone, repeat("中", 5), false, concat(0x1234, 2, 2));
        sample("concat-emoji", phone, repeat("😀", 33), false, concat(0x1234, 2, 1));
        sample("concat-before-escape", phone, repeat("A", 151), false, concat(0x1234, 2, 1));
        sample("concat-escape", phone, "^", false, concat(0x1234, 2, 2));
        sample("concat-zero", phone, "a", false, concat(0, 2, 1));
        sample("concat-last", phone, "z", false, concat(0xffff, 255, 255));
        sample("concat-report", phone, "😀", true, concat(0x1234, 2, 2));
        sample("concat-forced-unicode-ascii", phone, repeat("A", 66), false, concat(0x1234, 2, 1), 3);
        System.exit(0);
    }
}
