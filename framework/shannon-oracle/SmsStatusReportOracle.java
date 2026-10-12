package org.omarchy.audit;

import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;

/** Pure original AOSP parsers on bounded synthetic reports; no SMS service call. */
public final class SmsStatusReportOracle {
    private static Method parser;
    private static Constructor<?> timestampParser;
    private static Method timestamp;
    private static Field position;
    private static Field coding;
    private static byte[] unhex(String value) {
        if ((value.length() & 1) != 0 || value.length() > 1024) throw new IllegalArgumentException("hex bound");
        byte[] result = new byte[value.length()/2];
        for (int i=0;i<result.length;i++) {
            int low=Character.digit(value.charAt(i*2),16);
            int high=Character.digit(value.charAt(i*2+1),16);
            if (low<0 || high<0) throw new IllegalArgumentException("hex digit");
            result[i]=(byte)((low<<4)|high);
        }
        return result;
    }
    private static String text(String value) {
        if (value == null) return "-";
        StringBuilder result=new StringBuilder();
        for (byte data:value.getBytes(StandardCharsets.UTF_8)) result.append(String.format("%02x",data&255));
        return result.toString();
    }
    private static String discharge(byte[] pdu) throws Exception {
        // Full SmsMessage consumes and discards TP-DT. Execute the same original
        // timestamp decoder at the synthetic recipient's end, independently.
        int address=1+(pdu[0]&255)+2;
        int offset=address+2+((pdu[address]&255)+1)/2+7;
        Object p=timestampParser.newInstance((Object)pdu);
        position.setInt(p,offset);
        try { return String.valueOf(timestamp.invoke(p)); }
        catch (InvocationTargetException error) {
            if (error.getCause() instanceof IndexOutOfBoundsException) return "-";
            throw error;
        }
    }
    private static void sample(String[] input) throws Exception {
        String[] out=new String[16];
        out[0]="sms-status-report";out[1]=input[0];out[2]=input[1];out[3]=input[2];
        for(int i=4;i<15;i++) out[i]="-";
        byte[] pdu=unhex(input[1]);
        Object parsed=parser.invoke(null,(Object)pdu);
        out[4]=parsed==null ? "null" : "parsed";
        if (parsed!=null) {
            Class<?> c=parsed.getClass();
            out[5]=text((String)c.getMethod("getServiceCenterAddress").invoke(parsed));
            out[6]=String.valueOf(c.getField("mMessageRef").getInt(parsed));
            out[7]=text((String)c.getMethod("getRecipientAddress").invoke(parsed));
            out[8]=String.valueOf(c.getMethod("getTimestampMillis").invoke(parsed));
            out[9]=discharge(pdu);
            out[10]=String.valueOf(c.getMethod("getStatus").invoke(parsed));
            out[11]=String.valueOf(c.getMethod("getProtocolIdentifier").invoke(parsed));
            out[12]=String.valueOf(coding.getInt(parsed));
            out[13]=text((String)c.getMethod("getMessageBody").invoke(parsed));
            out[14]=(Boolean)c.getMethod("isStatusReportMessage").invoke(parsed) ? "1" : "0";
        }
        if (input[2].equals("compare") && (!out[4].equals("parsed") || !out[14].equals("1")))
            throw new AssertionError("original refused qualified report "+input[0]);
        out[15]="PASS";
        System.out.println(String.join("\t",out));
    }
    public static void main(String[] args) throws Exception {
        Class<?> message=Class.forName("com.android.internal.telephony.gsm.SmsMessage");
        parser=message.getMethod("createFromPdu",byte[].class);
        coding=message.getDeclaredField("mDataCodingScheme");coding.setAccessible(true);
        Class<?> p=Class.forName("com.android.internal.telephony.gsm.SmsMessage$PduParser");
        timestampParser=p.getDeclaredConstructor(byte[].class);timestampParser.setAccessible(true);
        position=p.getDeclaredField("mCur");position.setAccessible(true);
        timestamp=p.getDeclaredMethod("getSCTimestampMillis");timestamp.setAccessible(true);
        for (String line:Files.readAllLines(Paths.get(args[0]),StandardCharsets.UTF_8)) {
            String[] input=line.split("\t",-1);
            if (input.length!=3) throw new AssertionError("synthetic input shape");
            sample(input);
        }
        System.exit(0);
    }
}
