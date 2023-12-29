//! defines `Opcode` (represents Python bytecode opcodes).
//!
//! Opcode(Pythonバイトコードオペコードを表す)を定義する

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use crate::{impl_display_from_debug, impl_u8_enum};

/// Based on Python opcodes.
/// This is represented by u8.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
#[repr(u8)]
pub enum CommonOpcode {
    POP_TOP = 1,
    // ROT_TWO = 2,
    // ROT_THREE = 3,
    // DUP_TOP = 4,
    // DUP_TOP2 = 5,
    // ROT_FOUR = 6,
    NOP = 9,
    UNARY_POSITIVE = 10,
    UNARY_NEGATIVE = 11,
    UNARY_NOT = 12,
    UNARY_INVERT = 15,
    BINARY_MATRIX_MULTIPLY = 16,
    INPLACE_MATRIX_MULTIPLY = 17,
    STORE_SUBSCR = 60,
    GET_ITER = 68,
    GET_YIELD_FROM_ITER = 69,
    PRINT_EXPR = 70,
    LOAD_BUILD_CLASS = 71,
    RETURN_VALUE = 83,
    IMPORT_STAR = 84,
    YIELD_VALUE = 86,
    POP_BLOCK = 87,
    POP_EXCEPT = 89,
    /* ↓ These opcodes take an arg */
    STORE_NAME = 90,
    DELETE_NAME = 91,
    FOR_ITER = 93,
    UNPACK_EX = 94,
    STORE_ATTR = 95,
    STORE_GLOBAL = 97,
    LOAD_CONST = 100,
    LOAD_NAME = 101,
    BUILD_TUPLE = 102,
    BUILD_LIST = 103,
    BUILD_SET = 104,
    BUILD_MAP = 105, // build a Dict object
    LOAD_ATTR = 106,
    COMPARE_OP = 107,
    IMPORT_NAME = 108,
    IMPORT_FROM = 109,
    // JUMP_FORWARD = 110,
    JUMP_IF_FALSE_OR_POP = 111,
    JUMP_IF_TRUE_OR_POP = 112,
    // JUMP_ABSOLUTE = 113,
    // POP_JUMP_IF_FALSE = 114,
    // POP_JUMP_IF_TRUE = 115,
    LOAD_GLOBAL = 116,
    CONTAINS_OP = 118,
    LOAD_FAST = 124,
    STORE_FAST = 125,
    DELETE_FAST = 126,
    RAISE_VARARGS = 130,
    MAKE_FUNCTION = 132,
    CALL_FUNCTION_EX = 142,
    EXTENDED_ARG = 144,
    BUILD_CONST_KEY_MAP = 156,
    BUILD_STRING = 157,
    LOAD_METHOD = 160,
    NOT_IMPLEMENTED = 255,
}

use CommonOpcode::*;

impl_display_from_debug!(CommonOpcode);

impl TryFrom<u8> for CommonOpcode {
    type Error = ();
    fn try_from(byte: u8) -> Result<Self, ()> {
        Ok(match byte {
            1 => POP_TOP,
            // 2 => ROT_TWO,
            // 3 => ROT_THREE,
            // 4 => DUP_TOP,
            // 5 => DUP_TOP2,
            // 6 => ROT_FOUR,
            9 => NOP,
            10 => UNARY_POSITIVE,
            11 => UNARY_NEGATIVE,
            12 => UNARY_NOT,
            15 => UNARY_INVERT,
            60 => STORE_SUBSCR,
            68 => GET_ITER,
            69 => GET_YIELD_FROM_ITER,
            70 => PRINT_EXPR,
            71 => LOAD_BUILD_CLASS,
            83 => RETURN_VALUE,
            84 => IMPORT_STAR,
            86 => YIELD_VALUE,
            87 => POP_BLOCK,
            89 => POP_EXCEPT,
            /* ↓ These opcodes take an arg */
            90 => STORE_NAME,
            91 => DELETE_NAME,
            93 => FOR_ITER,
            94 => UNPACK_EX,
            95 => STORE_ATTR,
            97 => STORE_GLOBAL,
            100 => LOAD_CONST,
            101 => LOAD_NAME,
            102 => BUILD_TUPLE,
            103 => BUILD_LIST,
            104 => BUILD_SET,
            105 => BUILD_MAP,
            106 => LOAD_ATTR,
            107 => COMPARE_OP,
            108 => IMPORT_NAME,
            109 => IMPORT_FROM,
            // 110 => JUMP_FORWARD,
            111 => JUMP_IF_FALSE_OR_POP,
            112 => JUMP_IF_TRUE_OR_POP,
            // 113 => JUMP_ABSOLUTE,
            // 114 => POP_JUMP_IF_FALSE,
            // 115 => POP_JUMP_IF_TRUE,
            116 => LOAD_GLOBAL,
            118 => CONTAINS_OP,
            124 => LOAD_FAST,
            125 => STORE_FAST,
            126 => DELETE_FAST,
            130 => RAISE_VARARGS,
            132 => MAKE_FUNCTION,
            142 => CALL_FUNCTION_EX,
            144 => EXTENDED_ARG,
            156 => BUILD_CONST_KEY_MAP,
            157 => BUILD_STRING,
            160 => LOAD_METHOD,
            255 => NOT_IMPLEMENTED,
            _other => return Err(()),
        })
    }
}

impl From<CommonOpcode> for u8 {
    fn from(op: CommonOpcode) -> u8 {
        op as u8
    }
}

impl CommonOpcode {
    pub const fn take_arg(&self) -> bool {
        90 <= (*self as u8) && (*self as u8) < 220
    }

    pub fn is_jump_op(op: u8) -> bool {
        [93, 110, 111, 112, 113, 114, 115, 140, 143, 175, 176].contains(&op)
    }
}

impl_u8_enum! {CompareOp;
    LT = 0,
    LE = 1,
    EQ = 2,
    NE = 3,
    GT = 4,
    GE = 5,
}

impl CompareOp {
    fn show_op(&self) -> &str {
        match self {
            CompareOp::LT => "<",
            CompareOp::LE => "<=",
            CompareOp::EQ => "==",
            CompareOp::NE => "!=",
            CompareOp::GT => ">",
            CompareOp::GE => ">=",
        }
    }
}
