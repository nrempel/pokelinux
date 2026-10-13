use pokelinux_deployment::{Inventory, Profile, validate};
use serde::de::DeserializeOwned;
use std::{env, fs, io::Read, path::Path};

fn load<T: DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    const MAX_MANIFEST: u64 = 1024 * 1024;
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_MANIFEST + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_MANIFEST {
        return Err("manifest exceeds 1 MiB".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: pokelinux-deployment PROFILE.json INVENTORY.json STAGING_ROOT");
        std::process::exit(2);
    }
    let result = (|| -> Result<_, Box<dyn std::error::Error>> {
        let profile: Profile = load(&args[1])?;
        let inventory: Inventory = load(&args[2])?;
        Ok(validate(&profile, &inventory, Path::new(&args[3])))
    })();
    match result {
        Ok(report) => {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
            if !report.deployment_admitted {
                std::process::exit(1);
            }
        }
        Err(_) => {
            eprintln!("manifest input rejected");
            std::process::exit(2);
        }
    }
}
