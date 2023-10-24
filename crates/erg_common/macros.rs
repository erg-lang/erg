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
macro_rules! impl_u8_enum {
    ($Enum: ident; $($Variant: ident $(,)?)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum $Enum {
            $($Variant,)*
        }

        $crate::impl_display_from_debug!($Enum);

        impl TryFrom<u8> for $Enum {
            type Error = ();
            fn try_from(byte: u8) -> Result<Self, Self::Error> {
                match byte {
                    $($val => Ok($Enum::$Variant),)*
                    _ => Err(()),
                }
            }
        }

        impl From<$Enum> for u8 {
            fn from(op: $Enum) -> u8 {
                op as u8
            }
        }
    };
    ($Enum: ident; $($Variant: ident = $val: expr $(,)?)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum $Enum {
            $($Variant = $val,)*
        }

        $crate::impl_display_from_debug!($Enum);

        impl TryFrom<u8> for $Enum {
            type Error = ();
            fn try_from(byte: u8) -> Result<Self, Self::Error> {
                match byte {
                    $($val => Ok($Enum::$Variant),)*
                    _ => Err(()),
                }
            }
        }

        impl From<$Enum> for u8 {
            fn from(op: $Enum) -> u8 {
                op as u8
            }
        }

        impl $Enum {
            pub const fn take_arg(&self) -> bool {
                90 <= (*self as u8) && (*self as u8) < 220
            }
        }
    };
    ($Enum: ident; $size: tt; $($Variant: ident = $val: expr $(,)?)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr($size)]
        pub enum $Enum {
            $($Variant = $val,)*
        }

        $crate::impl_display_from_debug!($Enum);

        impl TryFrom<$size> for $Enum {
            type Error = ();
            fn try_from(byte: $size) -> Result<Self, Self::Error> {
                match byte {
                    $($val => Ok($Enum::$Variant),)*
                    _ => Err(()),
                }
            }
        }

        impl From<$Enum> for $size {
            fn from(op: $Enum) -> $size {
                op as $size
            }
        }
    };
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
/// Macros do not expand parameters, eliminating the cost of `format!`
#[macro_export]
macro_rules! switch_lang {
    (
        $should_english: literal => $msg: expr,
    ) => {{ $msg }};
    (
        $lang_name: literal => $msg: expr,
        $($rest_lang_name: literal => $rest_msg: expr,)+
    ) => {{
        if cfg!(feature = $lang_name) {
            $msg
        } else {
            switch_lang!($($rest_lang_name => $rest_msg,)+)
        }
    }};
}

/// Supports up to double unwrap
/// `:` is a dummy token to bypass restrictions
/// ```
/// # use erg_common::enum_unwrap;
/// use erg_common::Str;
/// let i: i32 = enum_unwrap!(Some(1), Some);
/// let s: &str = enum_unwrap!(Some(Str::ever("a")), Some:(Str::Static:(_)));
/// ```
#[macro_export]
macro_rules! enum_unwrap {
    ($ex: expr, $Enum: path $(,)*) => {{
        if let $Enum(res) = $ex { res } else { $crate::switch_unreachable!() }
    }};
    ($ex: expr, $Enum: path :( $Cons: path :(_) ) $(,)*) => {{
        if let $Enum($Cons(res)) = $ex { res } else { $crate::switch_unreachable!() }
    }};
    ($ex: expr, $Enum: path :( $Cons: path :( $Cons2: path :(_) ) ) $(,)*) => {{
        if let $Enum($Cons($Cons2(res))) = $ex { res } else { $crate::switch_unreachable!() }
    }};
    // X::A{a, b}
    ($ex: expr, $Enum: path {$($fields: ident $(,)*)*}) => {{
        if let $Enum{$($fields,)*} = $ex { ($($fields,)*) } else { $crate::switch_unreachable!() }
    }};
}

#[macro_export]
macro_rules! option_enum_unwrap {
    ($ex: expr, $Enum: path $(,)*) => {{
        if let $Enum(res) = $ex { Some(res) } else { None }
    }};
    ($ex: expr, $Enum: path :( $Cons: path :(_) ) $(,)*) => {{
        if let $Enum($Cons(res)) = $ex { Some(res) } else { None }
    }};
    ($ex: expr, $Enum: path :( $Cons: path :( $Cons2: path :(_) ) ) $(,)*) => {{
        if let $Enum($Cons($Cons2(res))) = $ex { Some(res) } else { None }
    }};
    ($ex: expr, $Enum: path {$($fields: ident $(,)*)*}) => {{
        if let $Enum{$($fields,)*} = $ex { Some(($($fields,)*)) } else { None }
    }};
}

/// ```rust
/// # use erg_common::fmt_option;
/// assert_eq!(fmt_option!(Some(1)), "1");
/// assert_eq!(fmt_option!(None::<i32>), "");
/// assert_eq!(fmt_option!(None::<i32>, else 1), "1");
/// assert_eq!(fmt_option!(Some(1), post ","), "1,");
/// assert_eq!(fmt_option!("[", Some(1), "]"), "[1]");
/// ```
#[macro_export]
macro_rules! fmt_option {
    ($ex: expr $(,)*) => {
        if let Some(x) = &$ex {
            format!("{}", x)
        } else {
            "".to_string()
        }
    };
    ($ex: expr $(,)*, else $els: expr $(,)*) => {
        if let Some(x) = &$ex {
            format!("{}", x)
        } else {
            $els.to_string()
        }
    };
    (pre $prefix: expr, $ex: expr $(,)*) => {
        if let Some(x) = &$ex {
            format!("{}{}", $prefix, x)
        } else {
            "".to_string()
        }
    };
    ($ex: expr, post $postfix: expr $(,)*) => {
        if let Some(x) = &$ex {
            format!("{}{}", x, $postfix)
        } else {
            "".to_string()
        }
    };
    ($prefix: expr, $ex: expr, $postfix: expr $(,)*) => {
        if let Some(x) = &$ex {
            format!("{}{}{}", $prefix, x, $postfix)
        } else {
            "".to_string()
        }
    };
}

#[macro_export]
macro_rules! fmt_option_map {
    ($ex: expr, $f: expr $(,)*) => {
        if let Some(x) = &$ex {
            format!("{}", $f(x))
        } else {
            "".to_string()
        }
    };
    ($ex: expr $(,)*, else $els: expr, $f: expr $(,)*) => {
        if let Some(x) = &$ex {
            format!("{}", $f(x))
        } else {
            $els.to_string()
        }
    };
    (pre $prefix: expr, $ex: expr, $f: expr $(,)*) => {
        if let Some(x) = &$ex {
            format!("{}{}", $prefix, $f(x))
        } else {
            "".to_string()
        }
    };
    ($ex: expr, post $postfix: expr, $f: expr $(,)*) => {
        if let Some(x) = &$ex {
            format!("{}{}", $f(x), $postfix)
        } else {
            "".to_string()
        }
    };
    ($prefix: expr, $ex: expr, $postfix: expr, $f: expr $(,)*) => {
        if let Some(x) = &$ex {
            format!("{}{}{}", $prefix, $f(x), $postfix)
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
        let mut names = type_name_of(dummy).rsplit("::");
        let mut name = names.nth(1).unwrap_or("?");
        while name == "{{closure}}" {
            name = names.next().unwrap_or("?");
        }
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

/// Once the argument is parsed into String, so
/// use `addr_eq!` or `AddrEq` trait if you want to compare addresses
#[macro_export]
macro_rules! addr {
    ($obj: expr) => {{
        let s = format!("{:p}", &$obj);
        let s = s.trim_start_matches("0x");
        u64::from_str_radix(&s, 16).unwrap()
    }};
}

/// do not use for reference types
#[macro_export]
macro_rules! addr_eq {
    ($l: expr, $r: expr $(,)*) => {{
        &$l as *const _ == &$r as *const _
    }};
}

#[macro_export]
macro_rules! ref_addr_eq {
    ($l: expr, $r: expr $(,)*) => {{
        $l as *const _ == $r as *const _
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
macro_rules! debug_info {
    ($output:ident) => {{
        #[allow(unused_imports)]
        use $crate::style::{colors::DEBUG, RESET};
        write!(
            $output,
            "[{}DEBUG{}] {}:{:04}: ",
            DEBUG,
            RESET,
            file!(),
            line!()
        )
        .unwrap();
    }};
    () => {{
        #[allow(unused_imports)]
        use $crate::style::{colors::DEBUG, RESET};
        print!("[{}DEBUG{}] {}:{:04}: ", DEBUG, RESET, file!(), line!());
    }};
}

/// Debug log utility.
/// directives:
///     c: colored output
///     f: file specified
///     f+c: file specified and colored (e.g. colored output to stderr)
///     info: info logging, (comprehensive shorthand for "c GREEN")
///     info_f: file version of info
///     err: error logging, (comprehensive shorthand for "c RED")
///     err_f: file version of err
#[macro_export]
macro_rules! log {
    (info $($arg: tt)*) => {{
        $crate::log!(c DEBUG_MAIN, $($arg)*);
    }};

    (err $($arg: tt)*) => {{
        $crate::log!(c DEBUG_ERROR, $($arg)*);
    }};

    (info_f $output:ident, $($arg: tt)*) => {{
        $crate::log!(f+c $output, DEBUG_MAIN, $($arg)*);
    }};

    (err_f $output:ident, $($arg: tt)*) => {{
        $crate::log!(f+c $output, DEBUG_ERROR, $($arg)*);
    }};

    (f $output: ident, $($arg: tt)*) => {{
        if cfg!(feature = "debug") {
            #[allow(unused_imports)]
            use $crate::color::{RESET, colors::DEBUG_MAIN, colors::DEBUG_ERROR};
            $crate::debug_info!($output);
            write!($output, $($arg)*).unwrap();
            write!($output, "{}", RESET).unwrap(); // color color anyway
            $output.flush().unwrap();
        }
    }};

    (c $color:ident, $($arg: tt)*) => {{
        if cfg!(feature = "debug") {
            #[allow(unused_imports)]
            use $crate::style::{RESET, colors::DEBUG_MAIN, colors::DEBUG_ERROR};
            $crate::debug_info!();
            print!("{}", $color);
            println!($($arg)*);
            print!("{}", RESET); // reset color anyway
        }
    }};

    (f+c $output:ident, $color:ident, $($arg: tt)*) => {{
        if cfg!(feature = "debug") {
            #[allow(unused_imports)]
            use $crate::style::{RESET, colors::DEBUG_MAIN};
            $crate::debug_info!($output);
            write!($output, "{}{}{}", $color, $($arg)*, RESET).unwrap();
            write!($output, $($arg)*).unwrap();
            write!($output, "{}", RESET).unwrap(); // reset color anyway
            $output.flush().unwrap();
        }
    }};

    (backtrace) => {{
        if cfg!(feature = "debug") {
            use $crate::style::*;
            $crate::debug_info!();
            println!("\n{}", std::backtrace::Backtrace::capture());
        }
    }};

    (caller) => {{
        if cfg!(feature = "debug") {
            use $crate::style::*;
            $crate::debug_info!();
            println!("\n{}", std::panic::Location::caller());
        }
    }};

    ($($arg: tt)*) => {{
        if cfg!(feature = "debug") {
            use $crate::style::*;
            $crate::debug_info!();
            println!($($arg)*);
            print!("{}", RESET); // reset color anyway
        }
    }};
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
        if cfg!(feature = "debug") { print!("{}:{:04}:\n", file!(), line!());
            print!("{} = ", stringify!($arg));
            println!("{}", $arg);
        }
    };
    ($head: expr, $($arg: expr $(,)*)+) => {
        if cfg!(feature = "debug") { print!("{}:{:04}:\n", file!(), line!());
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
