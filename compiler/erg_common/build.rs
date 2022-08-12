use std::process::Command;
// use std::io::Write;

mod datetime;

fn main() -> std::io::Result<()> {
    // recording the build date and the git hash
    let output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .expect("failed to get the git hash");
    let git_hash_short = String::from_utf8(output.stdout).unwrap();
    let now = datetime::now();
    println!("cargo:rustc-env=GIT_HASH_SHORT={git_hash_short}");
    println!("cargo:rustc-env=BUILD_DATE={now}");
    Ok(())
}
