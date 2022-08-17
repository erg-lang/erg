use std::process::Command;

/// returns the current datetime as String
pub fn now() -> String {
    let output = if cfg!(windows) {
        Command::new("cmd")
            .args(&["/C", "echo %date% %time%"])
            .output()
            .expect("failed to execute a process to get current time")
    } else {
        Command::new("date")
            .args(&["+%Y/%m/%d %T"])
            .output()
            .expect("failed to execute process to get current time")
    };
    String::from_utf8_lossy(&output.stdout[..])
        .trim_end()
        .to_string()
}
