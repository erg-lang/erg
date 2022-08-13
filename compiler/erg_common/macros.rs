#[macro_export]
macro_rules! impl_display_for_single_struct {
    ($Name: ident, $name: tt $(. $attr: tt)*) => {
        impl std::fmt::Display for $Name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.$name $(. $attr)*)
            }
        }
    };
    ($Name: ident) => {
        impl std::fmt::Display for $Name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    }
}

#[macro_export]
macro_rules! impl_display_from_debug {
    ($Name: ident) => {
        impl std::fmt::Display for $Name {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{self:#?}")
            }
        }
    };
}

#[macro_export]
macro_rules! impl_display_for_enum {
    ($Enum: ident; $($Variant: ident $(,)?)*) => {
        impl std::fmt::Display for $Enum {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $($Enum::$Variant(v) => write!(f, "{}", v),)*
                }
            }
        }
    }
}

#[macro_export]
macro_rules! impl_display_for_enum_with_variant {
    ($Enum: ident; $($Variant: ident $(,)?)*) => {
        impl std::fmt::Display for $Enum {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $($Enum::$Variant(v) => write!(f, "{} {}", stringify!($Variant), v),)*
                }
            }
        }
    }
}

/// More languages will be added ...
/// マクロはパラメータを展開しないので、format!のロスがなくなる
#[macro_export]
macro_rules! switch_lang {
    ($en: expr, $jp: expr $(,)*) => {{
        if cfg!(feature = "japanese") {
            $jp
        } else {
            $en
        }
    }};
}

/// 2重のunwrapまでサポート
/// :は制限を回避するためのdummy token
/// ```
/// let i: IntObj = enum_unwrap!(obj, Obj::Int);
/// let i: i32 = enum_unwrap!(obj, Obj::Int:(IntObj:(_)));
/// ```
#[macro_export]
macro_rules! enum_unwrap {
    ($ex: expr, $Enum: path $(,)*) => {{
        if let $Enum(res) = $ex { res } else { $crate::switch_unreachable!() }
    }};
    ($ex: expr, $Enum: path :( $Cons: path :(_) ) $(,)*) => {{
        if let $Enum($Cons(res)) = $ex { res } else { $crate::switch_unreachable!() }
    }};
    // X::A{a, b}
    ($ex: expr, $Enum: path {$($fields: ident $(,)*)*}) => {{
        if let $Enum{$($fields,)*} = $ex { ($($fields,)*) } else { $crate::switch_unreachable!() }
    }};
}

/// ```rust
/// assert fmt_option!(Some(1)) == "1"
/// assert fmt_option!(None) == ""
/// assert fmt_option!(None, else 1) == "1"
/// assert fmt_option!(Some(1), post: ",") == "1,"
/// assert fmt_option!("[", Some(1), "]") == "[1]"
/// ```
#[macro_export]
macro_rules! fmt_option {
    ($ex: expr $(,)*) => {
        if let Some(x) = $ex {
            format!("{}", x)
        } else {
            "".to_string()
        }
    };
    ($ex: expr $(,)*, else $els: expr $(,)*) => {
        if let Some(x) = $ex {
            format!("{}", x)
        } else {
            $els.to_string()
        }
    };
    (pre $prefix: expr, $ex: expr $(,)*) => {
        if let Some(x) = $ex {
            format!("{}{}", $prefix, x)
        } else {
            "".to_string()
        }
    };
    ($ex: expr, post $postfix: expr $(,)*) => {
        if let Some(x) = $ex {
            format!("{}{}", x, $postfix)
        } else {
            "".to_string()
        }
    };
    ($prefix: expr, $ex: expr, $postfix: expr $(,)*) => {
        if let Some(x) = $ex {
            format!("{}{}{}", $prefix, x, $postfix)
        } else {
            "".to_string()
        }
    };
}

#[macro_export]
macro_rules! switch_unreachable {
    () => {{
        if cfg!(debug_assertions) {
            unreachable!()
        } else {
            unsafe { std::hint::unreachable_unchecked() }
        }
    }};
}

#[macro_export]
macro_rules! assume_unreachable {
    () => {{
        unsafe { std::hint::unreachable_unchecked() }
    }};
}

/// indicates the current invoked function.
#[macro_export]
macro_rules! fn_name_full {
    () => {{
        const fn dummy() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(dummy); // "~::dummy"
        &name[..name.len() - 7] // remove "::dummy"
    }};
}

#[macro_export]
macro_rules! fn_name {
    () => {{
        const fn dummy() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(dummy).rsplit("::").nth(1).unwrap();
        &name[..]
    }};
}

// do not break lines (line!() is used)
#[macro_export]
macro_rules! caused_by {
    () => {{
        let fn_name = $crate::fn_name!();
        &format!("{fn_name} at line {}", line!())
    }};
}

/// 一度文字列をパースするので
/// アドレスの比較をしたい場合はaddr_eq!またはAddrEq traitを使うこと
#[macro_export]
macro_rules! addr {
    ($obj: expr) => {{
        let s = format!("{:p}", &$obj);
        let s = s.trim_start_matches("0x");
        u64::from_str_radix(&s, 16).unwrap()
    }};
}

#[macro_export]
macro_rules! addr_eq {
    ($l: expr, $r: expr $(,)*) => {{
        &$l as *const _ == &$r as *const _
    }};
}

#[macro_export]
macro_rules! power_assert {
    ($l: expr, $op: tt, $r: expr $(,)*) => {
        if !($l $op $r) {
            let s_l = stringify!($l);
            let s_r = stringify!($r);
            let s_op = stringify!($op);
            panic!(
                "assertion failed: `{s_l} {s_op} {s_r}` (`{s_l}` = {:#?}, `{s_r}` = {:#?})",
                $l, $r,
            )
        }
    };
    ($cond: expr) => {
        if !$cond {
            let s_cond = stringify!($cond);
            panic!("assertion failed: `{s_cond}` == {:#?}", $cond)
        }
    };
}

#[macro_export]
macro_rules! debug_power_assert {
    ($l: expr, $op: tt, $r: expr) => {
        if cfg!(debug_assertions) {
            erg_common::power_assert!($l, $op, $r)
        }
    };
    ($ex: expr) => {
        if cfg!(debug_assertions) {
            erg_common::power_assert!($ex)
        }
    };
}

#[macro_export]
macro_rules! debug_enum_assert {
    ($ex: expr, $Enum: ident :: $Variant: ident $(,)*) => {
        debug_assert!(common::enum_is!($ex, $Enum::$Variant));
    };
    ($ex: expr, $Enum: ident :: $Variant: ident, $Enum2: ident :: $Variant2: ident $(,)*) => {{
        debug_assert!(common::enum_is!($ex, $Enum::$Variant, $Enum2::$Variant2));
    }};
    ($ex: expr, $TupleCons: ident, $Enum: ident :: $Variant: ident $(,)*) => {{
        debug_assert!(common::enum_is!($ex, $TupleCons, $Enum::$Variant));
    }};
}

#[macro_export]
macro_rules! log {
    (f $output: ident, $($arg: tt)*) => {
        if cfg!(feature = "debug") { write!($output, "{}:{}: ", file!(), line!()).unwrap();
            write!($output, $($arg)*).unwrap();
            $output.flush().unwrap();
        }
    };
    ($($arg: tt)*) => {
        if cfg!(feature = "debug") { print!("{}:{}: ", file!(), line!()); println!($($arg)*); }
    };
}

#[macro_export]
macro_rules! log_with_time {
    (f $output: ident, $($arg: tt)*) => {
        if cfg!(feature = "debug") {
            write!($output, "{}: ", $crate::datetime::now()).unwrap();
            write!($output, $($arg)*).unwrap();
            $output.flush().unwrap();
        }
    };
    ($($arg: tt)*) => {
        if cfg!(feature = "debug") {
            print!("{}: ", $crate::datetime::now());
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! fmt_dbg {
    ($arg: expr $(,)*) => {
        if cfg!(feature = "debug") { print!("{}:{}:\n", file!(), line!());
            print!("{} = ", stringify!($arg));
            println!("{}", $arg);
        }
    };
    ($head: expr, $($arg: expr,)+) => {
        if cfg!(feature = "debug") { print!("{}:{}:\n", file!(), line!());
            print!("{} = ", stringify!($head));
            println!("{}", $head);
            $crate::fmt_dbg!(rec $($arg,)+);
        }
    };
    (rec $arg: expr,) => {
        if cfg!(feature = "debug") {
            print!("{} = ", stringify!($arg));
            println!("{}", $arg);
        }
    };
    (rec $head: expr, $($arg: expr,)+) => {
        if cfg!(feature = "debug") {
            print!("{} = ", stringify!($head));
            println!("{}", $head);
            $crate::fmt_dbg!(rec $($arg,)+);
        }
    };
}
