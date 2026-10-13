//! Authored fixture controls; no callback registration or modem commands.
use native_messages_incoming_binder_fixture::*;
use rsbinder::{FromIBinder, ProcessState};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let process = ProcessState::init("/dev/binder", 2)?;
    ProcessState::start_thread_pool();
    let b = lookup_probe(process)?;
    if !b.is_remote() || b.descriptor() != DESCRIPTOR {
        return Err("wrong probe identity".into());
    }
    let p: rsbinder::Strong<dyn IIncomingMessagesProbe> = FromIBinder::try_from(b)?;
    let n = |i: usize| -> Result<i32, Box<dyn std::error::Error>> {
        Ok(a.get(i).ok_or("missing number")?.parse()?)
    };
    match a.first().map(String::as_str) {
 Some("info") if a.len()==1=>{
  let mut endpoints=Vec::new(); for slot in 0..2 {let e=p.endpoint(slot)?;
   endpoints.push(serde_json::json!({"slot":slot,"version":e.getInterfaceVersion()?,"hash":e.getInterfaceHash()?,"counters":p.counters(slot)?}));}
  println!("{}",serde_json::json!({"generation":p.generation()?,"endpoints":endpoints}));
 }
 Some("counters") if a.len()==2=>println!("{}",serde_json::to_string(&p.counters(n(1)?)?)?),
 Some("barrier") if a.len()==5=>{p.barrier(n(1)?,n(2)?,n(3)?,n(4)?)?;println!("OK");}
 Some("burst") if a.len()==6=>{
  let h=&a[3];if h.len()>1026 || !h.len().is_multiple_of(2) || !h.bytes().all(|b|b.is_ascii_hexdigit()){return Err("bounded hexadecimal required".into());}
  let raw=(0..h.len()).step_by(2).map(|i|u8::from_str_radix(&h[i..i+2],16)).collect::<Result<Vec<_>,_>>()?;
  let count=n(5)?;if !(1..=64).contains(&count){return Err("bounded burst required".into());}
  let mut submitted=0;for _ in 0..count {if p.incoming(n(1)?,n(2)?,&raw,n(4)?).is_err(){break;}submitted+=1;}
  println!("{}",serde_json::json!({"submitted":submitted}));
 }
 Some("incoming") if a.len()==5=>{
  let h=&a[3]; if h.len()>1026||!h.len().is_multiple_of(2)||!h.bytes().all(|b|b.is_ascii_hexdigit()){return Err("bounded synthetic hexadecimal required".into());}
  let raw=(0..h.len()).step_by(2).map(|i|u8::from_str_radix(&h[i..i+2],16)).collect::<Result<Vec<_>,_>>()?;
  p.incoming(n(1)?,n(2)?,&raw,n(4)?)?;println!("SUBMITTED");
 }
 Some("die") if a.len()==1=>{let _=p.die();}
 _=>return Err("usage: info|counters slot|barrier slot registrations callbacks timeoutMs|incoming slot kind hex type|die".into()),
 }
    Ok(())
}
