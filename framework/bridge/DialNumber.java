package org.omarchy.bridge;
/** Protocol-level telephone number syntax. Carrier routing remains Android's job. */
final class DialNumber {
 static boolean valid(String value){return value!=null&&value.matches("\\+?[0-9]{3,32}");}
}
