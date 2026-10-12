package org.omarchy.audit;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.lang.reflect.Method;

/** Actual framework SMS-DELIVER parsing of synthetic bytes; no SMS API/service. */
public final class SmsReceiveOracle {
    private static Method parser;
    private static byte[] unhex(String value) {
        if (value.equals("-")) return new byte[0];
        byte[] result = new byte[value.length()/2];
        for (int i=0;i<result.length;i++) result[i]=(byte)Integer.parseInt(value.substring(i*2,i*2+2),16);
        return result;
    }
    private static String hex(byte[] data) {
        if (data.length==0) return "-";
        StringBuilder result=new StringBuilder();
        for (byte value:data) result.append(String.format("%02x",value&255));
        return result.toString();
    }
    private static byte[] deliver(byte[] submit, byte[] stamp, boolean replyPath) {
        int addressBytes=((submit[2]&255)+1)/2;
        ByteArrayOutputStream data=new ByteArrayOutputStream();
        data.write(0); // No SMSC prefix address.
        data.write((submit[0]&0x40) | (replyPath ? 0x80 : 0));
        data.write(submit,2,2+addressBytes); // Same synthetic originating address.
        data.write(submit,4+addressBytes,2); // PID and DCS.
        data.write(stamp,0,stamp.length);
        data.write(submit,6+addressBytes,submit.length-(6+addressBytes)); // UDL and packed data.
        return data.toByteArray();
    }
    private static void sample(String name,byte[] pdu) throws Exception {
        Object parsed=parser.invoke(null,(Object)pdu);
        if (parsed==null) throw new AssertionError("original parser refused fixture "+name);
        Class<?> type=parsed.getClass();
        String sender=(String)type.getMethod("getOriginatingAddress").invoke(parsed);
        String body=(String)type.getMethod("getMessageBody").invoke(parsed);
        if (sender==null || body==null) throw new AssertionError("original text unavailable "+name);
        long timestamp=((Number)type.getMethod("getTimestampMillis").invoke(parsed)).longValue();
        boolean reply=(Boolean)type.getMethod("isReplyPathPresent").invoke(parsed);
        Object udh=type.getMethod("getUserDataHeader").invoke(parsed);
        Object concat=udh==null ? null : udh.getClass().getField("concatRef").get(udh);
        String concatenation="-";
        if (concat!=null) {
            Class<?> c=concat.getClass();
            concatenation=((Boolean)c.getField("isEightBits").get(concat) ? "8" : "16")+":"
                +c.getField("refNumber").getInt(concat)+":"+c.getField("msgCount").getInt(concat)+":"+c.getField("seqNumber").getInt(concat);
        }
        System.out.println("sms-deliver\t"+name+"\t"+hex(pdu)+"\t"+hex(sender.getBytes(StandardCharsets.UTF_8))
            +"\t"+hex(body.getBytes(StandardCharsets.UTF_8))+"\t"+timestamp+"\t"+concatenation+"\t"+(reply?1:0)+"\tPASS");
    }
    public static void main(String[] args) throws Exception {
        parser=Class.forName("com.android.internal.telephony.gsm.SmsMessage").getMethod("createFromPdu",byte[].class);
        byte[] stamp=new byte[]{0x62,0x01,0x21,0x31,0x41,0x51,0};
        byte[] hello=null;
        for (String line:Files.readAllLines(Paths.get(args[0]),StandardCharsets.UTF_8)) {
            String[] fields=line.split("\t");
            if (!fields[0].equals("sms-submit")) continue;
            byte[] submit=unhex(fields[6]);
            sample(fields[1],deliver(submit,stamp,false));
            if (fields[1].equals("hello")) hello=submit;
        }
        if (hello==null) throw new AssertionError("hello input missing");
        Method encoder=Class.forName("com.android.internal.telephony.gsm.SmsMessage").getMethod("getSubmitPdu",String.class,String.class,String.class,boolean.class,byte[].class,int.class,int.class,int.class);
        for (String[] input:new String[][]{{"concat-gsm-eight","AB^"},{"concat-unicode-eight","你好"}}) {
            Object encoded=encoder.invoke(null,null,"+15550100",input[1],false,new byte[]{0,3,0x7f,2,1},1,0,0);
            sample(input[0],deliver((byte[])encoded.getClass().getField("encodedMessage").get(encoded),stamp,false));
        }
        Object bank=encoder.invoke(null,null,"+15550100","Bank",false,null,1,0,0);
        byte[] encodedBank=(byte[])bank.getClass().getField("encodedMessage").get(bank);
        byte[] standard=deliver(hello,stamp,false);
        ByteArrayOutputStream alpha=new ByteArrayOutputStream();
        alpha.write(0);alpha.write(0);alpha.write(7);alpha.write(0xd0);
        alpha.write(encodedBank,11,encodedBank.length-11);alpha.write(standard,8,standard.length-8);
        sample("alphanumeric-sender",alpha.toByteArray());
        stamp[6]=(byte)0x80;sample("timezone-positive",deliver(hello,stamp,false));
        stamp[6]=(byte)0x88;sample("timezone-negative",deliver(hello,stamp,false));
        stamp[6]=0;stamp[0]=(byte)0x99;sample("year-1999-reply-path",deliver(hello,stamp,true));
        System.exit(0);
    }
}
