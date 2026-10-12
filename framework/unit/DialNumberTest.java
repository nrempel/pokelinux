package org.omarchy.bridge;
final class DialNumberTest {
 public static void main(String[] args){
  String[] valid={"5550100","+12505550100","123","11111111111111111111111111111111"};
  for(String value:valid)if(!DialNumber.valid(value))throw new AssertionError("Valid number rejected");
  String[] invalid={"0","12","+1","+12","","+","++123","tel:5550100","sip:a@example.test","*21*5550100#","5550100,123","5550100;123","5550100\n","555 0100","１２３","١٢٣","111111111111111111111111111111111",null};
  for(String value:invalid)if(DialNumber.valid(value))throw new AssertionError("Invalid number accepted");
  System.out.println("DialNumber: canonical destinations and non-call syntax boundaries passed");
 }
}
