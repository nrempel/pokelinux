package org.omarchy.audit;

import java.lang.reflect.*;
import java.nio.*;
import java.nio.charset.StandardCharsets;
import org.json.JSONObject;

/** Authored synthetic parser probes. Never invokes onMessage, send or a service. */
public final class RegistrationIndicationOracle {
    private static Class<?> type;
    private static Constructor<?> constructor;
    private static Method update;
    private static int cases;
    private static final String[] fields = {"mRegState","mFeature","mFeatureBitmask","mStatusCode","mUriLength","mReasonLength","mFeatureBitmaskExt","mSipDelegateCapaBitmask","mFeatureExt","mUriReasonString"};
    private static String hex(byte[] data) {
        StringBuilder s=new StringBuilder();
        for(byte b:data)s.append(String.format(java.util.Locale.ROOT,"%02x",b&255));
        return s.toString();
    }
    private static byte[] packet(int state,int feature,int mask,int status,int uri,int reason,byte[] body,byte[] tail) {
        ByteBuffer b=ByteBuffer.allocate(12+body.length+tail.length).order(ByteOrder.LITTLE_ENDIAN);
        b.put((byte)state).put((byte)feature).putInt(mask).putShort((short)status).putShort((short)uri).putShort((short)reason).put(body).put(tail);
        return b.array();
    }
    private static JSONObject snapshot(Object instance) throws Exception {
        JSONObject result=new JSONObject();
        for(String name:fields) {
            Field field=type.getDeclaredField(name);field.setAccessible(true);
            Object value=field.get(instance);
            result.put(name,value==null?JSONObject.NULL:value);
        }
        return result;
    }
    private static Object fresh(boolean seeded) throws Exception {
        Object object=constructor.newInstance(new Object[]{null});
        if(seeded) {
            for(String name:fields) {
                Field field=type.getDeclaredField(name);field.setAccessible(true);
                if(field.getType()==int.class)field.setInt(object,-1234567);
                else if(field.getType()==long.class)field.setLong(object,-123456789012345L);
                else if(field.getType()==String.class)field.set(object,"sentinel");
            }
        }
        return object;
    }
    private static void run(String id,byte[] data,int length,boolean seeded) throws Exception {
        JSONObject out=new JSONObject();out.put("id",id);out.put("input",hex(data));out.put("length",length);out.put("seeded",seeded);
        Object instance=fresh(seeded);out.put("before",snapshot(instance));
        try {update.invoke(instance,data,length);out.put("exception",JSONObject.NULL);}
        catch(InvocationTargetException e) {out.put("exception",e.getCause().getClass().getName());}
        out.put("after",snapshot(instance));System.out.println(out.toString());cases++;
    }
    public static void main(String[] args) throws Exception {
        type=Class.forName("com.shannon.imsservice.network.adaptor.data.RilIndRegistration",false,RegistrationIndicationOracle.class.getClassLoader());
        if(args.length==1 && args[0].equals("metadata")) {
            JSONObject meta=new JSONObject();meta.put("class",type.getName());meta.put("base",type.getSuperclass().getName());
            StringBuilder constructors=new StringBuilder();for(Constructor<?> c:type.getSuperclass().getDeclaredConstructors())constructors.append(c.toString()).append("\n");
            meta.put("base_constructors",constructors.toString());System.out.println(meta.toString());return;
        }
        System.err.println("ORACLE_STEP: channel-class");
        Class<?> channel=Class.forName("com.shannon.imsservice.network.NetworkAdaptor$AdaptorChannel");
        System.err.println("ORACLE_STEP: constructor-metadata");
        constructor=type.getConstructor(channel);update=type.getMethod("update",byte[].class,int.class);
        System.err.println("ORACLE_STEP: constants-read");
        Object constantOwner=fresh(false);
        JSONObject constants=new JSONObject();constants.put("record","constants");
        JSONObject staticFields=new JSONObject();
        for(String name:new String[]{"RAT_LTE","RAT_LTEPLUS","RAT_UNKNOWN","URI_LENGTH_MAX","REASON_LENGTH_MAX"}) {
            Field f=type.getDeclaredField(name);f.setAccessible(true);staticFields.put(name,Modifier.isStatic(f.getModifiers()));constants.put(name,f.getInt(constantOwner));
        }
        constants.put("static_fields",staticFields);
        constants.put("default_charset",java.nio.charset.Charset.defaultCharset().name());System.out.println(constants.toString());
        byte[] full=packet(255,128,Integer.MIN_VALUE,65535,4,3,new byte[]{'s','i','p',0,'o','k',0},new byte[]{1,2,3,4,5,6,7,8,9,10,11,12,13,14});
        for(int n=0;n<=full.length;n++) {run("prefix-"+n,full,n,false);run("seeded-prefix-"+n,full,n,true);}
        for(int tail=0;tail<=17;tail++) {
            byte[] t=new byte[tail];for(int j=0;j<tail;j++)t[j]=(byte)(j+1);
            run("tail-"+tail,packet(1,2,0x12345678,200,0,0,new byte[0],t),12+tail,false);
        }
        int[] lengths={0,1,2,255,256,257,999,1000,1001,32767,32768,65535};
        for(int n:lengths) {
            for(boolean uri:new boolean[]{true,false}) {
                int retained=uri?Math.min(n,1000):Math.min(n,256);
                byte[] body=new byte[retained];java.util.Arrays.fill(body,(byte)'x');
                run((uri?"uri-":"reason-")+n,packet(127,255,-1,32768,uri?n:0,uri?0:n,body,new byte[0]),12+body.length,false);
            }
        }
        byte[][] texts={new byte[]{65,0,66},new byte[]{(byte)0xc3,(byte)0xa9},new byte[]{(byte)0xff},new byte[]{(byte)0xc0},new byte[]{(byte)0xf0,(byte)0x9f,(byte)0x98,(byte)0x80}};
        for(int i=0;i<texts.length;i++)run("text-"+i,packet(0,0,0,0,texts[i].length,0,texts[i],new byte[0]),12+texts[i].length,false);
        for(int n:new int[]{-1,0,1,full.length,full.length+1})run("range-"+n,full,n,false);
        run("oversized-4097",new byte[4097],4097,false);
        System.err.println("Original registration indication synthetic cases: "+cases);System.exit(0);
    }
}
