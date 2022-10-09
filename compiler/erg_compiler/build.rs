#![allow(deprecated)]

use std::env;
use std::fs;
use std::path;

fn main() -> std::io::Result<()> {
    // Create a ".erg" directory
    let erg_path = env::home_dir()
        .expect("failed to get the location of the home dir")
        .to_str()
        .expect("invalid encoding of the home dir name")
        .to_string()
        + "/.erg";
    if !path::Path::new(&erg_path).exists() {
        fs::create_dir(&erg_path)?;
        fs::create_dir(format!("{erg_path}/std"))?;
    }
    println!("cargo:rustc-env=ERG_PATH={erg_path}");
    println!("cargo:rustc-env=ERG_STD_PATH={erg_path}/std");
    // create a std library in ".erg"
    for res in fs::read_dir("std")? {
        let entry = res?;
        let path = entry.path();
        let filename = path
            .file_name()
            .expect("this is not a file")
            .to_str()
            .unwrap();
        fs::copy(&path, format!("{erg_path}/std/{filename}"))?;
    }
    Ok(())
}
