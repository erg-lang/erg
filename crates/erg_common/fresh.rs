use std::sync::atomic::AtomicUsize;

static VAR_ID: AtomicUsize = AtomicUsize::new(0);

pub fn fresh_varname() -> String {
    VAR_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let i = VAR_ID.load(std::sync::atomic::Ordering::SeqCst);
    format!("%v{i}")
}

pub fn fresh_param_name() -> String {
    VAR_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let i = VAR_ID.load(std::sync::atomic::Ordering::SeqCst);
    format!("%p{i}")
}
