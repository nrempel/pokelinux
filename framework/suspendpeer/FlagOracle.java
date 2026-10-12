package org.pokelinux.suspendpeer;
/** Read-only installed original-framework bootstrap flag oracle. */
public final class FlagOracle {
    public static void main(String[] args) throws Exception {
        Class<?> activityManager=Class.forName("android.app.ActivityManager");
        java.lang.reflect.Field flag=activityManager.getDeclaredField("INSTR_FLAG_NO_RESTART");
        flag.setAccessible(true);
        System.out.println("no-restart\t"+flag.getInt(null));
    }
}
