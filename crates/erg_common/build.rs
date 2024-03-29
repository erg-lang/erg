use std::process::Command;

mod datetime;

fn main() -> std::io::Result<()> {
    // recording the build date and the git hash
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .expect("failed to get the git hash");
    let git_hash_short = String::from_utf8_lossy(&output.stdout);
    let now = datetime::now();
    #[allow(deprecated)]
    let erg_path = std::env::home_dir().unwrap_or_default().join(".erg");
    println!("cargo:rustc-env=GIT_HASH_SHORT={git_hash_short}");
    println!("cargo:rustc-env=BUILD_DATE={now}");
    println!("cargo:rustc-env=CARGO_ERG_PATH={}", erg_path.display());
    let case_sensitive = if cfg!(windows) {
        false
    } else if cfg!(target_os = "macos") {
        let command = Command::new("diskutil")
            .args(["info", "/"])
            .output()
            .expect("failed to get the file system type");
        let output = String::from_utf8_lossy(&command.stdout);
        output.contains("Case-sensitive")
    } else {
        true
    };
    println!("cargo:rustc-env=CASE_SENSITIVE={}", case_sensitive);
    Ok(())
}
