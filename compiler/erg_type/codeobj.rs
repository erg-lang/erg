use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;

use erg_common::impl_display_from_debug;
use erg_common::opcode::Opcode;
use erg_common::python_util::detect_magic_number;
use erg_common::serialize::*;
use erg_common::Str;

use crate::deserialize::{DeserializeResult, Deserializer};
use crate::value::ValueObj;
use crate::{HasType, Type, TypePair};

pub fn consts_into_bytes(consts: Vec<ValueObj>) -> Vec<u8> {
    let mut tuple = vec![];
    if consts.len() > u8::MAX as usize {
        tuple.push(DataTypePrefix::Tuple as u8);
        tuple.append(&mut (consts.len() as u32).to_le_bytes().to_vec());
    } else {
        tuple.push(DataTypePrefix::SmallTuple as u8);
        tuple.push(consts.len() as u8);
    }
    for obj in consts {
        tuple.append(&mut obj.into_bytes());
    }
    tuple
}

/// Bit masks for CodeObj.flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CodeObjFlags {
    Optimized = 0x0001,
    NewLocals = 0x0002,
    VarArgs = 0x0004,
    VarKeywords = 0x0008,
    Nested = 0x0010,
    Generator = 0x0020,
    NoFree = 0x0040,
    Coroutine = 0x0080,
    IterableCoroutine = 0x0100,
    AsyncGenerator = 0x0200,
    // CO_GENERATOR_ALLOWED    = 0x0400,
    FutureDivision = 0x2000,
    FutureAbsoluteImport = 0x4000,
    FutureWithStatement = 0x8000,
    FuturePrintFunction = 0x1_0000,
    FutureUnicodeLiterals = 0x2_0000,
    FutureBarryAsBDFL = 0x4_0000,
    FutureGeneratorStop = 0x8_0000,
    FutureAnnotations = 0x10_0000,
    // Erg-specific flags
    EvmDynParam = 0x1000_0000,
    EvmNoGC = 0x4000_0000,
    Illegal = 0x0000,
}

impl From<u32> for CodeObjFlags {
    fn from(flags: u32) -> Self {
        match flags {
            0x0001 => Self::Optimized,
            0x0002 => Self::NewLocals,
            0x0004 => Self::VarArgs,
            0x0008 => Self::VarKeywords,
            0x0010 => Self::Nested,
            0x0020 => Self::Generator,
            0x0040 => Self::NoFree,
            0x0080 => Self::Coroutine,
            0x0100 => Self::IterableCoroutine,
            0x0200 => Self::AsyncGenerator,
            // CO_GENERATOR_ALLOWED,
            0x2000 => Self::FutureDivision,
            0x4000 => Self::FutureAbsoluteImport,
            0x8000 => Self::FutureWithStatement,
            0x1_0000 => Self::FuturePrintFunction,
            0x2_0000 => Self::FutureUnicodeLiterals,
            0x4_0000 => Self::FutureBarryAsBDFL,
            0x8_0000 => Self::FutureGeneratorStop,
            0x10_0000 => Self::FutureAnnotations,
            // EVM flags
            0x1000_0000 => Self::EvmDynParam,
            0x4000_0000 => Self::EvmNoGC,
            _ => Self::Illegal,
        }
    }
}

impl CodeObjFlags {
    pub const fn is_in(&self, flags: u32) -> bool {
        (flags & *self as u32) != 0
    }
}

/// Implementation of `PyCodeObject`, see Include/cpython/code.h in CPython for details.
///
/// 各属性をErg側のObjに変換すると遅くなりそうなので、アクサスされたときのみ変換して提供する
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CodeObj {
    pub argcount: u32,
    pub posonlyargcount: u32,
    pub kwonlyargcount: u32,
    pub nlocals: u32, // == params + local vars
    pub stacksize: u32,
    pub flags: u32,
    pub code: Vec<u8>,
    pub consts: Vec<ValueObj>, // objects used in the code (literal)
    pub names: Vec<Str>,       // names used in the code object
    pub varnames: Vec<Str>,    // names defined in the code object
    pub freevars: Vec<Str>,    // names captured from the outer scope
    pub cellvars: Vec<Str>,    // names used in the inner function (closure)
    pub filename: Str,
    pub name: Str,
    pub firstlineno: u32,
    // lnotab (line number table): see Object/lnotab_notes.txt in CPython for details
    // e.g. +12bytes, +3line -> [.., 0x1C, 0x03, ..]
    // ([sdelta, ldelta, sdelta, ldelta, ..])
    // if delta > 255 -> [255, 0, 255-delta, ...]
    pub lnotab: Vec<u8>,
}

impl HasType for CodeObj {
    fn ref_t(&self) -> &Type {
        &Type::Code
    }
    fn ref_mut_t(&mut self) -> &mut Type {
        todo!()
    }
    fn signature_t(&self) -> Option<&Type> {
        None
    }
    fn signature_mut_t(&mut self) -> Option<&mut Type> {
        None
    }
}

impl fmt::Debug for CodeObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<code object {} at {:p}, file \"{}\", line {}>",
            self.name, self, self.filename, self.firstlineno
        )
    }
}

impl_display_from_debug!(CodeObj);

impl Default for CodeObj {
    fn default() -> Self {
        Self {
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 2, // Seems to be the default in CPython, but not sure why
            flags: CodeObjFlags::NoFree as u32,
            code: Vec::new(),
            consts: Vec::new(),
            names: Vec::new(),
            varnames: Vec::new(),
            freevars: Vec::new(),
            cellvars: Vec::new(),
            filename: "<dummy>".into(),
            name: "<dummy>".into(),
            firstlineno: 1,
            lnotab: Vec::new(),
        }
    }
}

impl CodeObj {
    pub fn new<S: Into<Str>>(
        argcount: u32,
        posonlyargcount: u32,
        kwonlyargcount: u32,
        nlocals: u32,
        stacksize: u32,
        flags: u32,
        code: Vec<u8>,
        consts: Vec<ValueObj>,
        names: Vec<Str>,
        varnames: Vec<Str>,
        freevars: Vec<Str>,
        cellvars: Vec<Str>,
        filename: Str,
        name: S,
        firstlineno: u32,
        lnotab: Vec<u8>,
    ) -> Self {
        Self {
            argcount,
            posonlyargcount,
            kwonlyargcount,
            nlocals,
            stacksize,
            flags,
            code,
            consts,
            names,
            varnames,
            freevars,
            cellvars,
            filename,
            name: name.into(),
            firstlineno,
            lnotab,
        }
    }

    pub fn empty<S: Into<Str>, T: Into<Str>>(
        params: Vec<Str>,
        filename: S,
        name: T,
        firstlineno: u32,
    ) -> Self {
        Self {
            argcount: params.len() as u32,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: params.len() as u32,
            stacksize: 2, // Seems to be the default in CPython, but not sure why
            flags: CodeObjFlags::NoFree as u32,
            code: Vec::with_capacity(8),
            consts: Vec::with_capacity(4),
            names: Vec::with_capacity(3),
            varnames: params,
            freevars: Vec::new(),
            cellvars: Vec::new(),
            filename: filename.into(),
            name: name.into(),
            firstlineno,
            lnotab: Vec::with_capacity(4),
        }
    }

    pub fn from_pyc<P: AsRef<Path>>(path: P) -> DeserializeResult<Self> {
        let mut f = BufReader::new(File::open(path)?);
        let v = &mut Vec::with_capacity(16);
        f.read_to_end(v)?;
        let python_ver = get_magic_num_from_bytes(&Deserializer::consume::<4>(v));
        let _padding = Deserializer::deserialize_u32(v);
        let _timestamp = Deserializer::deserialize_u32(v);
        let _padding = Deserializer::deserialize_u32(v);
        let code = Self::from_bytes(v, python_ver)?;
        Ok(code)
    }

    pub fn from_bytes(v: &mut Vec<u8>, python_ver: u32) -> DeserializeResult<Self> {
        let mut des = Deserializer::new();
        let argcount = Deserializer::deserialize_u32(v);
        let posonlyargcount = if python_ver >= 3413 {
            Deserializer::deserialize_u32(v)
        } else {
            0
        };
        let kwonlyargcount = Deserializer::deserialize_u32(v);
        let nlocals = Deserializer::deserialize_u32(v);
        let stacksize = Deserializer::deserialize_u32(v);
        let flags = Deserializer::deserialize_u32(v);
        let code = des.deserialize_bytes(v)?;
        let consts = des.deserialize_const_vec(v, python_ver)?;
        let names = des.deserialize_str_vec(v, python_ver)?;
        let varnames = des.deserialize_str_vec(v, python_ver)?;
        let freevars = des.deserialize_str_vec(v, python_ver)?;
        let cellvars = des.deserialize_str_vec(v, python_ver)?;
        let filename = des.deserialize_str(v, python_ver)?;
        let name = des.deserialize_str(v, python_ver)?;
        let firstlineno = Deserializer::deserialize_u32(v);
        let lnotab = des.deserialize_bytes(v)?;
        Ok(CodeObj::new(
            argcount,
            posonlyargcount,
            kwonlyargcount,
            nlocals,
            stacksize,
            flags,
            code,
            consts,
            names,
            varnames,
            freevars,
            cellvars,
            filename,
            name,
            firstlineno,
            lnotab,
        ))
    }

    pub fn into_bytes(self, python_ver: u32) -> Vec<u8> {
        let mut bytes = vec![DataTypePrefix::Code as u8];
        bytes.append(&mut self.argcount.to_le_bytes().to_vec());
        if python_ver >= 3413 {
            bytes.append(&mut self.posonlyargcount.to_le_bytes().to_vec());
        }
        bytes.append(&mut self.kwonlyargcount.to_le_bytes().to_vec());
        bytes.append(&mut self.nlocals.to_le_bytes().to_vec());
        bytes.append(&mut self.stacksize.to_le_bytes().to_vec());
        bytes.append(&mut self.flags.to_le_bytes().to_vec());
        // co_code is represented as PyStrObject (Not Ascii, Unicode)
        bytes.append(&mut raw_string_into_bytes(self.code));
        bytes.append(&mut consts_into_bytes(self.consts)); // write as PyTupleObject
        bytes.append(&mut strs_into_bytes(self.names));
        bytes.append(&mut strs_into_bytes(self.varnames));
        bytes.append(&mut strs_into_bytes(self.freevars));
        bytes.append(&mut strs_into_bytes(self.cellvars));
        bytes.append(&mut str_into_bytes(self.filename, false));
        bytes.append(&mut str_into_bytes(self.name, true));
        bytes.append(&mut self.firstlineno.to_le_bytes().to_vec());
        // lnotab is represented as PyStrObject
        bytes.append(&mut raw_string_into_bytes(self.lnotab));
        bytes
    }

    pub fn dump_as_pyc<P: AsRef<Path>>(
        self,
        path: P,
        python_ver: Option<u32>,
    ) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let mut bytes = Vec::with_capacity(16);
        let python_ver = python_ver.unwrap_or_else(detect_magic_number);
        bytes.append(&mut get_magic_num_bytes(python_ver).to_vec());
        bytes.append(&mut vec![0; 4]); // padding
        bytes.append(&mut get_timestamp_bytes().to_vec());
        bytes.append(&mut vec![0; 4]); // padding
        bytes.append(&mut self.into_bytes(python_ver));
        file.write_all(&bytes[..])?;
        Ok(())
    }

    fn tables_info(&self) -> String {
        let mut tables = "".to_string();
        if !self.consts.is_empty() {
            tables += "Constants:\n";
        }
        for (i, obj) in self.consts.iter().enumerate() {
            tables += &format!("   {}: {}\n", i, obj);
        }
        if !self.names.is_empty() {
            tables += "Names:\n";
        }
        for (i, name) in self.names.iter().enumerate() {
            tables += &format!("   {}: {}\n", i, name);
        }
        if !self.varnames.is_empty() {
            tables += "Varnames:\n";
        }
        for (i, varname) in self.varnames.iter().enumerate() {
            tables += &format!("   {}: {}\n", i, varname);
        }
        if !self.cellvars.is_empty() {
            tables += "Cellvars:\n";
        }
        for (i, cellvar) in self.cellvars.iter().enumerate() {
            tables += &format!("   {}: {}\n", i, cellvar);
        }
        if !self.freevars.is_empty() {
            tables += "Freevars:\n";
        }
        for (i, freevar) in self.freevars.iter().enumerate() {
            tables += &format!("   {}: {}\n", i, freevar);
        }
        tables
    }

    fn attrs_info(&self) -> String {
        let mut attrs = "".to_string();
        attrs += &format!("Name:              {}\n", self.name);
        attrs += &format!("FileName:          {}\n", self.filename);
        attrs += &format!("Argument count:    {}\n", self.argcount);
        attrs += &format!("Positional-only arguments: {}\n", self.posonlyargcount);
        attrs += &format!("Kw-only arguments: {}\n", self.kwonlyargcount);
        attrs += &format!("Number of locals:  {}\n", self.nlocals);
        attrs += &format!("Stack size:        {}\n", self.stacksize);
        let mut flagged = "".to_string();
        for i in 0..32 {
            if (self.flags & (1 << i)) != 0 {
                let flag: CodeObjFlags = 2u32.pow(i).into();
                flagged += &format!("{:?}, ", flag);
            }
        }
        flagged.pop();
        flagged.pop();
        attrs += &format!("Flags:             {}\n", flagged);
        attrs
    }

    fn instr_info(&self) -> String {
        let mut lnotab_iter = self.lnotab.iter();
        let mut code_iter = self.code.iter();
        let mut idx = 0;
        let mut line_offset = 0;
        let mut lineno = self.firstlineno as u8;
        let mut sdelta = lnotab_iter.next().unwrap_or(&0);
        let mut ldelta = lnotab_iter.next().unwrap_or(&0);
        let mut instrs = "".to_string();
        instrs += &format!("lnotab: {:?}\n", self.lnotab);
        if *sdelta != 0 {
            instrs += &format!("{}:\n", lineno);
        }
        loop {
            if *sdelta == line_offset {
                line_offset = 0;
                lineno += ldelta;
                instrs += &format!("{}:\n", lineno);
                sdelta = lnotab_iter.next().unwrap_or(&0);
                ldelta = lnotab_iter.next().unwrap_or(&0);
            }
            if let (Some(op), Some(arg)) = (code_iter.next(), code_iter.next()) {
                let op = Opcode::from(*op);
                let s_op = op.to_string();
                instrs += &format!("{:>15} {:<25}", idx, s_op);
                match op {
                    Opcode::COMPARE_OP => {
                        let op = match arg {
                            0 => "<",
                            1 => "<=",
                            2 => "==",
                            3 => "!=",
                            4 => ">",
                            5 => ">=",
                            _ => "?",
                        };
                        instrs += &format!("{} ({})", arg, op);
                    }
                    Opcode::STORE_NAME
                    | Opcode::LOAD_NAME
                    | Opcode::STORE_GLOBAL
                    | Opcode::LOAD_GLOBAL
                    | Opcode::STORE_ATTR
                    | Opcode::LOAD_ATTR
                    | Opcode::LOAD_METHOD => {
                        instrs += &format!("{} ({})", arg, self.names.get(*arg as usize).unwrap());
                    }
                    Opcode::STORE_DEREF | Opcode::LOAD_DEREF => {
                        instrs +=
                            &format!("{} ({})", arg, self.freevars.get(*arg as usize).unwrap());
                    }
                    Opcode::STORE_FAST | Opcode::LOAD_FAST => {
                        instrs +=
                            &format!("{} ({})", arg, self.varnames.get(*arg as usize).unwrap());
                    }
                    Opcode::LOAD_CONST => {
                        instrs += &format!("{} ({})", arg, self.consts.get(*arg as usize).unwrap());
                    }
                    Opcode::FOR_ITER => {
                        instrs += &format!("{} (to {})", arg, idx + arg * 2 + 2);
                    }
                    Opcode::JUMP_FORWARD => {
                        instrs += &format!("{} (to {})", arg, idx + arg * 2 + 2);
                    }
                    Opcode::JUMP_ABSOLUTE => {
                        instrs += &format!("{} (to {})", arg, arg * 2);
                    }
                    Opcode::POP_JUMP_IF_FALSE | Opcode::POP_JUMP_IF_TRUE => {
                        instrs += &format!("{} (to {})", arg, arg * 2);
                    }
                    Opcode::MAKE_FUNCTION => {
                        let flag = match arg {
                            8 => "(closure)",
                            // TODO:
                            _ => "",
                        };
                        instrs += &format!("{} {}", arg, flag);
                    }
                    // Ergでは引数で型キャストする
                    Opcode::BINARY_ADD
                    | Opcode::BINARY_SUBTRACT
                    | Opcode::BINARY_MULTIPLY
                    | Opcode::BINARY_TRUE_DIVIDE => {
                        instrs += &format!("{} ({:?})", arg, TypePair::from(*arg));
                    }
                    other if other.take_arg() => {
                        instrs += &format!("{}", arg);
                    }
                    _ => {}
                }
                instrs.push('\n');
                idx += 2;
                line_offset += 2;
            } else {
                break;
            }
        }
        instrs
    }

    pub fn code_info(&self) -> String {
        let mut info = "".to_string();
        info += &format!("Disassembly of {:?}:\n", self);
        info += &self.attrs_info();
        info += &self.tables_info();
        info += &self.instr_info();
        info.push('\n');
        for cons in self.consts.iter() {
            if let ValueObj::Code(c) = cons {
                info += &c.code_info();
            }
        }
        info
    }
}
