//! Isolated composition using the ordinary production actor and plain real Owner.
use std::{io,path::Path};
fn main()->io::Result<()> {
 let a:Vec<_>=std::env::args_os().skip(1).collect();
 if a.first().is_some_and(|v|v=="--storage-worker"){return pokelinux_lifecycle::storage::worker(&a[1..]);}
 if a.len()!=5 || rustix::process::getuid().as_raw()!=0 || std::env::var("NATIVE_LIFECYCLE_ISOLATED").as_deref()!=Ok("1"){return Err(io::ErrorKind::InvalidInput.into());}
 let mut audio=pokelinux_lifecycle::audio::Owner::connect(Path::new(&a[4]),11040)?;
 pokelinux_lifecycle::service::run_with_services(Path::new(&a[0]),Path::new(&a[1]),Path::new(&a[2]),Path::new(&a[3]),None,None,None,None,None,Some(&mut audio),None)
}
