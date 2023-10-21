#[cfg(all(unix, feature = "backtrace"))]
pub use backtrace_on_stack_overflow;
use std::thread::{self, JoinHandle};

const STACK_SIZE: usize = if cfg!(feature = "large_thread") {
    8 * 1024 * 1024
} else {
    4 * 1024 * 1024
};

#[macro_export]
macro_rules! enable_overflow_stacktrace {
    () => {
        #[cfg(all(unix, feature = "backtrace"))]
        unsafe {
            $crate::spawn::backtrace_on_stack_overflow::enable()
        };
    };
}

/// Execute the function in a new thread.
/// The default stack size is 4MB, and with the `large_thread` flag, the stack size is 8MB.
pub fn exec_new_thread<F, T>(run: F, name: &str) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    enable_overflow_stacktrace!();
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
}

pub fn spawn_new_thread<F, T>(run: F, name: &str) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    enable_overflow_stacktrace!();
    thread::Builder::new()
        .name(name.to_string())
        .stack_size(STACK_SIZE)
        .spawn(run)
        .unwrap()
}

pub fn safe_yield() {
    std::thread::yield_now();
    std::thread::sleep(std::time::Duration::from_millis(10));
}
