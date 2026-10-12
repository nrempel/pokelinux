package org.omarchy.audit;

import java.lang.reflect.Constructor;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import org.json.JSONObject;

/** Authored synthetic calls to an original factory reader. No services or modem calls. */
public final class ReaderOracle {
    private static Class<?> reader;
    private static Constructor<?> range;
    private static Method remaining;
    private static int cases;

    private static byte[] bytes(String hex) {
        byte[] out = new byte[hex.length() / 2];
        for (int i = 0; i < out.length; i++) out[i] = (byte)Integer.parseInt(hex.substring(2*i, 2*i+2), 16);
        return out;
    }
    private static String hex(byte[] bytes) {
        StringBuilder out = new StringBuilder();
        for (byte b : bytes) out.append(String.format(java.util.Locale.ROOT, "%02x", b & 255));
        return out.toString();
    }
    private static void run(String name, String method, byte[] input, int offset, int length, int argument) throws Exception {
        JSONObject out = new JSONObject();
        out.put("id", name); out.put("method", method); out.put("input", hex(input));
        out.put("offset", offset); out.put("length", length); out.put("argument", argument);
        Object instance = null;
        try {
            instance = range.newInstance(input, offset, length);
            Object value = null;
            if (!method.equals("construct")) {
                if (method.equals("getBytes") || method.equals("skipBytes")) value = reader.getMethod(method, int.class).invoke(instance, argument);
                else value = reader.getMethod(method).invoke(instance);
            }
            out.put("kind", value instanceof byte[] ? "bytes" : value instanceof String ? "string" : value instanceof Number ? "integer" : "void");
            out.put("value", value instanceof byte[] ? hex((byte[])value) : value == null ? JSONObject.NULL : value);
        } catch (InvocationTargetException failure) {
            out.put("kind", "error"); out.put("value", failure.getCause().getClass().getName());
        }
        if (instance != null) out.put("remaining", remaining.invoke(instance));
        System.out.println(out.toString()); cases++;
    }
    private static void scalar(String method, int width) throws Exception {
        String[] vectors = {"0000000000000000", "0102030405060708", "ffffffffffffffff", "0000000000000080", "0000008000000000", "ff7fffffffffffff", "80fffefffdfcfbfa"};
        for (int i = 0; i < vectors.length; i++) {
            byte[] value = bytes(vectors[i]);
            run(method+"-vector-"+i, method, value, 0, value.length, 0);
            run(method+"-offset-"+i, method, value, 1, value.length-1, 0);
        }
        for (int count=0;count<width;count++) run(method+"-short-"+count,method,bytes("0102030405060708"),0,count,0);
    }
    private static void text(String method, boolean wide) throws Exception {
        String[] vectors = {"", "00", "41", "4142", "414200", "410042", "c3a900", "e282ac00", "f09f988000", "ff00", "c000", "f080808000", "4142434445"};
        for (int i=0;i<vectors.length;i++) {
            byte[] data=bytes(vectors[i]);
            for (int delta=-1;delta<=1;delta++) {
                int size=Math.max(0,data.length+delta);
                byte[] input=new byte[(wide?2:1)+data.length];
                input[0]=(byte)size; if(wide) input[1]=(byte)(size>>>8);
                System.arraycopy(data,0,input,wide?2:1,data.length);
                run(method+"-string-"+i+"-"+delta,method,input,0,input.length,0);
            }
        }
        int[] lengths=wide ? new int[]{127,128,255,256,32767,32768,65535} : new int[]{127,128,255};
        for(int size:lengths) {
            byte[] input=new byte[(wide?2:1)+size];input[0]=(byte)size;if(wide)input[1]=(byte)(size>>>8);
            java.util.Arrays.fill(input,wide?2:1,input.length,(byte)'x');input[input.length-1]=0;
            run(method+"-length-"+size,method,input,0,input.length,0);
        }
        for(int n=0;n<(wide?2:1);n++)run(method+"-missing-prefix-"+n,method,new byte[n],0,n,0);
    }
    public static void main(String[] args) throws Exception {
        reader=Class.forName("com.shannon.imsservice.network.oem.io.DataReader");
        range=reader.getConstructor(byte[].class,int.class,int.class);remaining=reader.getMethod("getLeftSize");
        scalar("getByte",1); scalar("getByteAsInt",1); scalar("getShort",2);
        scalar("getShortAsInt",2);scalar("getInt",4);scalar("getLong",8);
        for(int size:new int[]{-1,0,1,2,3,4,8,16})run("bytes-"+size,"getBytes",bytes("0102ff"),0,3,size);
        for(int size:new int[]{-1,0,1,2,3,4,8,16,Integer.MIN_VALUE,Integer.MAX_VALUE})run("skip-"+size,"skipBytes",bytes("0102ff"),0,3,size);
        for(int offset:new int[]{-1,0,1,3,4,Integer.MAX_VALUE})for(int length:new int[]{-1,0,1,3,4,Integer.MAX_VALUE})run("construct-"+offset+"-"+length,"construct",bytes("010203"),offset,length,0);
        text("getCpFormatByteLenString",false);text("getCpFormatShortLenString",true);
        System.err.println("Original pure IMS reader cases: "+cases);
    }
}
