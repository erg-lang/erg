//! オブジェクトのシリアライズ(バイナリ列化)のためのユーティリティーを定義・実装する
use std::time::{SystemTime, UNIX_EPOCH};

use crate::impl_display_from_debug;
use crate::Str;

/* Python bytecode specification */
// 0~3 byte: magic number
// 4~7 byte: padding (0;4)
// 8~B byte: UNIX timestamp
// C~F byte: padding (0;4)
// 10~ byte: marshalled code objects
// the unary magic number of Python bytecode
// magic number = version number (2byte) + 168624128 (0x0A0D0000)
// e.g. Python 3.7.4 b5's version number: 3394
// -> magic number: 0x0AD0D042
// -> bytes (little endian): 42 0D 0D 0A
pub const fn get_magic_num_bytes(python_ver: u32) -> [u8; 4] {
    pub const PREFIX: u32 = 0xA0D0000;
    (PREFIX | python_ver).to_le_bytes()
}

pub const fn get_magic_num_from_bytes(bytes: &[u8; 4]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], 0, 0])
}

pub fn get_timestamp_bytes() -> [u8; 4] {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32;
    secs.to_le_bytes()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DataTypePrefix {
    /* sized objects */
    Illegal = 0,
    Int32 = b'i',      // 0x69
    Int64 = b'I',      // 0x49
    Float = b'f',      // 0x66 (float32, not generated anymore?)
    BinFloat = b'g',   // 0x67 (float64)
    Complex = b'x',    // 0x78
    BinComplex = b'y', // 0x79
    True = b'T',       // 0x54
    False = b'F',      // 0x46
    None = b'N',       // 0x4E
    StopIter = b'S',   // 0x53
    Ref = b'r',
    /* unsized objects (ref counted) */
    Long = b'l',       // 0x6C + len:  u32 + payload: 2*len+3byte (~ -2^31-1 && 2^31 ~)
    Str = b's',        // 0x73 + len:  u32 + payload
    ShortAscii = b'z', // 0x7A + len:  u8 + payload
    ShortAsciiInterned = b'Z', //  0x5A + len:  u8 + payload
    Unicode = b'u',    // 0x75 + len:  u32 + payload
    Interned = b't',   // 0x74 + len + payload
    SmallTuple = b')', // 0x29 + len:  u8 + payload
    Tuple = b'(',      // 0x28 + len:  u32 + payload
    Code = b'c',       // 0x63
    /* Erg specific prefix */
    Builtin = b'b', // 0x62 + str
    Nat = b'n',
}

impl_display_from_debug!(DataTypePrefix);

impl From<u8> for DataTypePrefix {
    fn from(item: u8) -> Self {
        match item as char {
            'i' | '\u{00E9}' => Self::Int32,
            'I' => Self::Int64,
            'l' => Self::Long,
            'f' => Self::Float,
            'g' => Self::BinFloat,
            'x' => Self::Complex,
            'y' => Self::BinComplex,
            'T' => Self::True,
            'F' => Self::False,
            'N' => Self::None,
            'S' => Self::StopIter,
            's' | '\u{00F3}' => Self::Str,
            'Z' | '\u{00DA}' => Self::ShortAsciiInterned,
            'z' | '\u{00FA}' => Self::ShortAscii,
            'u' => Self::Unicode,
            't' => Self::Interned,
            '(' | '\u{00A8}' => Self::Tuple,
            ')' | '\u{00A9}' => Self::SmallTuple,
            'c' | '\u{00E3}' => Self::Code,
            'b' => Self::Builtin,
            'n' => Self::Nat,
            /*'\u{00F9}' => DataTypeUnaryOp::ErgInt8,
            '\u{00FA}' => DataTypeUnaryOp::ErgInt32,
            '\u{00FB}' => DataTypeUnaryOp::ErgFloat32,
            '\u{00FC}' => DataTypeUnaryOp::ErgStr,
            '\u{00FD}' => DataTypeUnaryOp::ErgFloat,
            '\u{00FE}' => DataTypeUnaryOp::HeapArray,
            '\u{00FF}' => DataTypeUnaryOp::NumArray,*/
            _ => Self::Illegal,
        }
    }
}

impl DataTypePrefix {
    pub const fn is_sized(&self) -> bool {
        matches!(
            self,
            Self::Long
                | Self::Str
                | Self::ShortAscii
                | Self::ShortAsciiInterned
                | Self::Unicode
                | Self::Interned
                | Self::SmallTuple
                | Self::Tuple
                | Self::Code
                | Self::Builtin
        )
    }
}

pub fn strs_into_bytes(names: Vec<Str>) -> Vec<u8> {
    let mut tuple = vec![];
    if names.len() > u8::MAX as usize {
        tuple.push(DataTypePrefix::Tuple as u8);
        tuple.append(&mut (names.len() as u32).to_le_bytes().to_vec());
    } else {
        tuple.push(DataTypePrefix::SmallTuple as u8);
        tuple.push(names.len() as u8);
    }
    for name in names.into_iter() {
        tuple.append(&mut str_into_bytes(name, true));
    }
    tuple
}

pub fn str_into_bytes(cont: Str, is_interned: bool) -> Vec<u8> {
    let mut bytes = vec![];
    if cont.is_ascii() {
        if is_interned {
            bytes.push(DataTypePrefix::ShortAsciiInterned as u8);
        } else {
            bytes.push(DataTypePrefix::ShortAscii as u8);
        }
        bytes.push(cont.len() as u8);
    } else {
        bytes.push(DataTypePrefix::Unicode as u8);
        bytes.append(&mut (cont.len() as u32).to_le_bytes().to_vec());
    };
    bytes.append(&mut cont.as_bytes().to_vec());
    bytes
}

pub fn raw_string_into_bytes(mut cont: Vec<u8>) -> Vec<u8> {
    let mut tuple = vec![DataTypePrefix::Str as u8];
    tuple.append(&mut (cont.len() as u32).to_le_bytes().to_vec());
    tuple.append(&mut cont);
    tuple
}
