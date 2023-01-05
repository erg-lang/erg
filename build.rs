use erg_common::python_util::{env_magic_number, env_python_version};

fn main() -> std::io::Result<()> {
    let version = env_python_version();
    if version.major != 3 {
        panic!("Python 3 is required");
    }
    println!(
        "cargo:rustc-env=PYTHON_VERSION_MINOR={}",
        version.minor.unwrap_or(11)
    );
    println!(
        "cargo:rustc-env=PYTHON_VERSION_MICRO={}",
        version.micro.unwrap_or(0)
    );
    let magic_number = env_magic_number();
    println!("cargo:rustc-env=PYTHON_MAGIC_NUMBER={}", magic_number);
    Ok(())
}
