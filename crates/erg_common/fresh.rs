use crate::shared::Shared;

use once_cell::sync::Lazy;

pub static VAR_ID: Lazy<Shared<usize>> = Lazy::new(|| Shared::new(0));

pub fn fresh_varname() -> String {
    *VAR_ID.borrow_mut() += 1;
    let i = *VAR_ID.borrow();
    format!("%v{i}")
}

pub fn fresh_param_name() -> String {
    *VAR_ID.borrow_mut() += 1;
    let i = *VAR_ID.borrow();
    format!("%p{i}")
}
