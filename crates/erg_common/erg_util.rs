use std::process::Command;

pub fn get_erg_version(erg_command: &str) -> Option<String> {
    let out = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(erg_command)
            .arg("--version")
            .output()
            .ok()?
    } else {
        let exec_command = format!("{erg_command} --version");
        Command::new("sh")
            .arg("-c")
            .arg(exec_command)
            .output()
            .ok()?
    };
    // e.g. Erg 0.1.0, Erg 0.1.2-nightly.2
    let s_version = String::from_utf8(out.stdout).ok()?;
    let mut iter = s_version.split(' ').skip(1);
    Some(iter.next()?.trim_end().to_string())
}

pub fn env_erg_version() -> Option<String> {
    get_erg_version("erg")
}

pub const BUILTIN_ERG_MODS: [&str; 3] = ["consts", "consts/physics", "semver"];
