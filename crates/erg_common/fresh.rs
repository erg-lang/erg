use crate::shared::Shared;

thread_local! {
    static VAR_ID: Shared<usize> = Shared::new(0);
}

pub fn fresh_varname() -> String {
    VAR_ID.with(|id| {
        *id.borrow_mut() += 1;
        let i = *id.borrow();
        format!("%v{i}")
    })
}

pub fn fresh_param_name() -> String {
    VAR_ID.with(|id| {
        *id.borrow_mut() += 1;
        let i = *id.borrow();
        format!("%p{i}")
    })
}
