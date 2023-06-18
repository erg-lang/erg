#[cfg(all(unix, any(feature = "debug", feature = "backtrace")))]
pub use backtrace_on_stack_overflow;
use std::thread::{self, JoinHandle};

#[macro_export]
macro_rules! enable_overflow_stacktrace {
    () => {
        #[cfg(all(unix, any(feature = "debug", feature = "backtrace")))]
        unsafe {
            $crate::spawn::backtrace_on_stack_overflow::enable()
        };
    };
}

/// Execute a function in a new thread on Windows, otherwise just run it.
///
/// Windows has a smaller default stack size than other OSs, which may cause a stack overflow, especially in the parsing process.
pub fn exec_new_thread<F, T>(run: F, name: &str) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    enable_overflow_stacktrace!();
    if cfg!(windows) || cfg!(feature = "large_thread") {
        const STACK_SIZE: usize = 4 * 1024 * 1024;
        let child = thread::Builder::new()
            .name(name.to_string())
            .stack_size(STACK_SIZE)
            .spawn(run)
            .unwrap();
        // Wait for thread to join
        child.join().unwrap_or_else(|err| {
            eprintln!("Thread panicked: {err:?}");
            std::process::exit(1);
        })
    } else {
        run()
    }
}

pub fn spawn_new_thread<F, T>(run: F, name: &str) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    enable_overflow_stacktrace!();
    const STACK_SIZE: usize = 4 * 1024 * 1024;
    thread::Builder::new()
        .name(name.to_string())
        .stack_size(STACK_SIZE)
        .spawn(run)
        .unwrap()
}
