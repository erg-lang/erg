#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Triple<T, E> {
    Ok(T),
    Err(E),
    None,
}

impl<T, E> Triple<T, E> {
    pub fn none_then(self, err: E) -> Result<T, E> {
        match self {
            Triple::None => Err(err),
            Triple::Ok(ok) => Ok(ok),
            Triple::Err(err) => Err(err),
        }
    }

    pub fn none_or_result(self, f: impl FnOnce() -> E) -> Result<T, E> {
        match self {
            Triple::None => Err(f()),
            Triple::Ok(ok) => Ok(ok),
            Triple::Err(err) => Err(err),
        }
    }

    pub fn none_or_else(self, f: impl FnOnce() -> Triple<T, E>) -> Triple<T, E> {
        match self {
            Triple::None => f(),
            Triple::Ok(ok) => Triple::Ok(ok),
            Triple::Err(err) => Triple::Err(err),
        }
    }

    #[track_caller]
    pub fn unwrap_to_result(self) -> Result<T, E> {
        match self {
            Triple::None => panic!("unwrapping Triple::None"),
            Triple::Ok(ok) => Ok(ok),
            Triple::Err(err) => Err(err),
        }
    }

    pub fn ok(self) -> Option<T> {
        match self {
            Triple::None => None,
            Triple::Ok(ok) => Some(ok),
            Triple::Err(_) => None,
        }
    }

    pub fn or_else(self, f: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
        match self {
            Triple::None => f(),
            Triple::Ok(ok) => Ok(ok),
            Triple::Err(err) => Err(err),
        }
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Triple::None => default,
            Triple::Ok(ok) => ok,
            Triple::Err(_) => default,
        }
    }

    #[track_caller]
    pub fn unwrap_err(self) -> E {
        match self {
            Triple::None => panic!("unwrapping Triple::None"),
            Triple::Ok(_) => panic!("unwrapping Triple::Ok"),
            Triple::Err(err) => err,
        }
    }

    pub fn map_ok<T2>(self, f: impl FnOnce(T) -> T2) -> Triple<T2, E> {
        match self {
            Triple::None => Triple::None,
            Triple::Ok(ok) => Triple::Ok(f(ok)),
            Triple::Err(err) => Triple::Err(err),
        }
    }

    pub fn map_ok_or<T2>(self, default: T2, f: impl FnOnce(T) -> T2) -> T2 {
        match self {
            Triple::None => default,
            Triple::Ok(ok) => f(ok),
            Triple::Err(_err) => default,
        }
    }

    pub fn map_err<E2>(self, f: impl FnOnce(E) -> E2) -> Triple<T, E2> {
        match self {
            Triple::None => Triple::None,
            Triple::Ok(ok) => Triple::Ok(ok),
            Triple::Err(err) => Triple::Err(f(err)),
        }
    }
}

impl<T> Triple<T, T> {
    pub fn either(self) -> Option<T> {
        match self {
            Triple::None => None,
            Triple::Ok(a) | Triple::Err(a) => Some(a),
        }
    }
}

impl<T, E: std::error::Error> Triple<T, E> {
    #[track_caller]
    pub fn unwrap(self) -> T {
        match self {
            Triple::None => panic!("unwrapping Triple::None"),
            Triple::Ok(ok) => ok,
            Triple::Err(err) => panic!("unwrapping Triple::Err({err})"),
        }
    }
}
