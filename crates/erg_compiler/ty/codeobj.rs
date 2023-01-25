use std::fmt;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufReader, Read, Write as _};
use std::path::Path;

use erg_common::impl_display_from_debug;
#[allow(unused_imports)]
use erg_common::log;
use erg_common::opcode::CommonOpcode;
use erg_common::opcode308::Opcode308;
use erg_common::opcode310::Opcode310;
use erg_common::opcode311::{BinOpCode, Opcode311};
use erg_common::python_util::{env_magic_number, PythonVersion};
use erg_common::serialize::*;
use erg_common::Str;

use super::deserialize::{DeserializeResult, Deserializer};
use super::value::ValueObj;
use super::{HasType, Type, TypePair};

pub fn consts_into_bytes(consts: Vec<ValueObj>, python_ver: PythonVersion) -> Vec<u8> {
    let mut tuple = vec![];
    if consts.len() > u8::MAX as usize {
        tuple.push(DataTypePrefix::Tuple as u8);
        tuple.append(&mut (consts.len() as u32).to_le_bytes().to_vec());
    } else {
        tuple.push(DataTypePrefix::SmallTuple as u8);
        tuple.push(consts.len() as u8);
    }
    for obj in consts {
        tuple.append(&mut obj.into_bytes(python_ver));
    }
    tuple
}

/// Kind can be multiple (e.g. Local + Cell = 0x60)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FastKind {
    Local = 0x20,
    Cell = 0x40,
    Free = 0x80,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MakeFunctionFlags {
    None = 0,
    Defaults = 1,
    KwDefaults = 2,
    Annotations = 4,
    Closure = 8,
}

impl From<u8> for MakeFunctionFlags {
    fn from(flags: u8) -> Self {
        match flags {
            0 => Self::None,
            1 => Self::Defaults,
            2 => Self::KwDefaults,
            4 => Self::Annotations,
            8 => Self::Closure,
            _ => unreachable!(),
        }
    }
}

/// Implementation of `PyCodeObject`, see Include/cpython/code.h in CPython for details.
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
    pub qualname: Str,
    pub firstlineno: u32,
    // lnotab (line number table): see Object/lnotab_notes.txt in CPython for details
    // e.g. +12bytes, +3line -> [.., 0x1C, 0x03, ..]
    // ([sdelta, ldelta, sdelta, ldelta, ..])
    // if delta > 255 -> [255, 0, 255-delta, ...]
    pub lnotab: Vec<u8>,
    pub exceptiontable: Vec<u8>,
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
            flags: 0,     //CodeObjFlags::NoFree as u32,
            code: Vec::new(),
            consts: Vec::new(),
            names: Vec::new(),
            varnames: Vec::new(),
            freevars: Vec::new(),
            cellvars: Vec::new(),
            filename: "<dummy>".into(),
            name: "<dummy>".into(),
            qualname: "<dummy>".into(),
            firstlineno: 1,
            lnotab: Vec::new(),
            exceptiontable: Vec::new(),
        }
    }
}

impl CodeObj {
    pub fn empty<S: Into<Str>, T: Into<Str>>(
        params: Vec<Str>,
        filename: S,
        name: T,
        firstlineno: u32,
        flags: u32,
    ) -> Self {
        let name = name.into();
        let var_args_defined = (flags & CodeObjFlags::VarArgs as u32 != 0) as u32;
        Self {
            argcount: params.len() as u32 - var_args_defined,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: params.len() as u32,
            stacksize: 2, // Seems to be the default in CPython, but not sure why
            flags,        // CodeObjFlags::NoFree as u32,
            code: Vec::with_capacity(8),
            consts: Vec::with_capacity(4),
            names: Vec::with_capacity(3),
            varnames: params,
            freevars: Vec::new(),
            cellvars: Vec::new(),
            filename: filename.into(),
            name: name.clone(),
            qualname: name,
            firstlineno,
            lnotab: Vec::with_capacity(4),
            exceptiontable: Vec::with_capacity(0),
        }
    }

    pub fn from_pyc<P: AsRef<Path>>(path: P) -> DeserializeResult<Self> {
        let mut f = BufReader::new(File::open(path)?);
        let v = &mut Vec::with_capacity(16);
        f.read_to_end(v)?;
        let magic_num = get_magic_num_from_bytes(&Deserializer::consume::<4>(v));
        let python_ver = get_ver_from_magic_num(magic_num);
        let _padding = Deserializer::deserialize_u32(v);
        let _timestamp = Deserializer::deserialize_u32(v);
        let _padding = Deserializer::deserialize_u32(v);
        let code = Self::from_bytes(v, python_ver)?;
        Ok(code)
    }

    pub fn from_bytes(v: &mut Vec<u8>, python_ver: PythonVersion) -> DeserializeResult<Self> {
        let mut des = Deserializer::new();
        let argcount = Deserializer::deserialize_u32(v);
        let posonlyargcount = if python_ver.minor >= Some(8) {
            Deserializer::deserialize_u32(v)
        } else {
            0
        };
        let kwonlyargcount = Deserializer::deserialize_u32(v);
        let nlocals = if python_ver.minor >= Some(11) {
            0
        } else {
            Deserializer::deserialize_u32(v)
        };
        let stacksize = Deserializer::deserialize_u32(v);
        let flags = Deserializer::deserialize_u32(v);
        let code = des.deserialize_bytes(v)?;
        let consts = des.deserialize_const_vec(v, python_ver)?;
        let names = des.deserialize_str_vec(v, python_ver)?;
        // TODO: localplusnames
        let varnames = des.deserialize_str_vec(v, python_ver)?;
        let freevars = des.deserialize_str_vec(v, python_ver)?;
        let cellvars = des.deserialize_str_vec(v, python_ver)?;
        let filename = des.deserialize_str(v, python_ver)?;
        let name = des.deserialize_str(v, python_ver)?;
        let qualname = des.deserialize_str(v, python_ver)?;
        let firstlineno = Deserializer::deserialize_u32(v);
        let lnotab = des.deserialize_bytes(v)?;
        let exceptiontable = if python_ver.minor >= Some(11) {
            des.deserialize_bytes(v)?
        } else {
            vec![]
        };
        Ok(CodeObj {
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
            qualname,
            firstlineno,
            lnotab,
            exceptiontable,
        })
    }

    pub fn into_bytes(self, python_ver: PythonVersion) -> Vec<u8> {
        let mut bytes = vec![DataTypePrefix::Code as u8];
        bytes.append(&mut self.argcount.to_le_bytes().to_vec());
        if python_ver.minor >= Some(8) {
            bytes.append(&mut self.posonlyargcount.to_le_bytes().to_vec());
        }
        bytes.append(&mut self.kwonlyargcount.to_le_bytes().to_vec());
        if python_ver.minor < Some(11) {
            bytes.append(&mut self.nlocals.to_le_bytes().to_vec());
        }
        bytes.append(&mut self.stacksize.to_le_bytes().to_vec());
        bytes.append(&mut self.flags.to_le_bytes().to_vec());
        // co_code is represented as PyStrObject (Not Ascii, Unicode)
        bytes.append(&mut raw_string_into_bytes(self.code));
        bytes.append(&mut consts_into_bytes(self.consts, python_ver)); // write as PyTupleObject
        bytes.append(&mut strs_into_bytes(self.names));
        Self::dump_locals(
            self.varnames,
            self.freevars,
            self.cellvars,
            &mut bytes,
            python_ver,
        );
        bytes.append(&mut str_into_bytes(self.filename, false));
        bytes.append(&mut str_into_bytes(self.name, true));
        if python_ver.minor >= Some(11) {
            bytes.append(&mut str_into_bytes(self.qualname, true));
        }
        bytes.append(&mut self.firstlineno.to_le_bytes().to_vec());
        // lnotab is represented as PyStrObject
        bytes.append(&mut raw_string_into_bytes(self.lnotab));
        if python_ver.minor >= Some(11) {
            bytes.append(&mut raw_string_into_bytes(self.exceptiontable));
        }
        bytes
    }

    fn dump_locals(
        varnames: Vec<Str>,
        freevars: Vec<Str>,
        cellvars: Vec<Str>,
        bytes: &mut Vec<u8>,
        python_ver: PythonVersion,
    ) {
        if python_ver.minor >= Some(11) {
            let varnames = varnames
                .into_iter()
                .filter(|n| !freevars.contains(n) && !cellvars.contains(n))
                .collect::<Vec<_>>();
            let localspluskinds = [
                vec![FastKind::Local as u8; varnames.len()],
                vec![FastKind::Free as u8; freevars.len()],
                vec![FastKind::Cell as u8 + FastKind::Local as u8; cellvars.len()],
            ]
            .concat();
            let localsplusnames = [varnames, freevars, cellvars].concat();
            bytes.append(&mut strs_into_bytes(localsplusnames));
            bytes.append(&mut raw_string_into_bytes(localspluskinds));
        } else {
            bytes.append(&mut strs_into_bytes(varnames));
            bytes.append(&mut strs_into_bytes(freevars));
            bytes.append(&mut strs_into_bytes(cellvars));
        }
    }

    pub fn dump_as_pyc<P: AsRef<Path>>(
        self,
        path: P,
        py_magic_num: Option<u32>,
    ) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let mut bytes = Vec::with_capacity(16);
        let py_magic_num = py_magic_num.unwrap_or_else(env_magic_number);
        let python_ver = get_ver_from_magic_num(py_magic_num);
        bytes.append(&mut get_magic_num_bytes(py_magic_num).to_vec());
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
            writeln!(tables, "   {i}: {obj}").unwrap();
        }
        if !self.names.is_empty() {
            tables += "Names:\n";
        }
        for (i, name) in self.names.iter().enumerate() {
            writeln!(tables, "   {i}: {name}").unwrap();
        }
        if !self.varnames.is_empty() {
            tables += "Varnames:\n";
        }
        for (i, varname) in self.varnames.iter().enumerate() {
            writeln!(tables, "   {i}: {varname}").unwrap();
        }
        if !self.cellvars.is_empty() {
            tables += "Cellvars:\n";
        }
        for (i, cellvar) in self.cellvars.iter().enumerate() {
            writeln!(tables, "   {i}: {cellvar}").unwrap();
        }
        if !self.freevars.is_empty() {
            tables += "Freevars:\n";
        }
        for (i, freevar) in self.freevars.iter().enumerate() {
            writeln!(tables, "   {i}: {freevar}\n").unwrap();
        }
        tables
    }

    fn attrs_info(&self) -> String {
        let mut attrs = "".to_string();
        writeln!(attrs, "Name:              {}", self.name).unwrap();
        writeln!(attrs, "FileName:          {}", self.filename).unwrap();
        writeln!(attrs, "Argument count:    {}", self.argcount).unwrap();
        writeln!(attrs, "Positional-only arguments: {}", self.posonlyargcount).unwrap();
        writeln!(attrs, "Kw-only arguments: {}", self.kwonlyargcount).unwrap();
        writeln!(attrs, "Number of locals:  {}", self.nlocals).unwrap();
        writeln!(attrs, "Stack size:        {}", self.stacksize).unwrap();
        let mut flagged = "".to_string();
        for i in 0..32 {
            if (self.flags & (1 << i)) != 0 {
                let flag: CodeObjFlags = 2u32.pow(i).into();
                write!(flagged, "{flag:?}, ").unwrap();
            }
        }
        flagged.pop();
        flagged.pop();
        writeln!(attrs, "Flags:             {flagged}").unwrap();
        attrs
    }

    fn instr_info(&self, py_ver: Option<PythonVersion>) -> String {
        let mut lnotab_iter = self.lnotab.iter();
        let mut code_iter = self.code.iter();
        let mut idx = 0usize;
        let mut line_offset = 0usize;
        let mut lineno = self.firstlineno;
        let mut sdelta = lnotab_iter.next().unwrap_or(&0);
        let mut ldelta = lnotab_iter.next().unwrap_or(&0);
        let mut instrs = "".to_string();
        writeln!(instrs, "lnotab: {:?}", self.lnotab).unwrap();
        if *sdelta != 0 {
            writeln!(instrs, "{lineno}:").unwrap();
        }
        loop {
            if *sdelta as usize == line_offset {
                line_offset = 0;
                lineno += *ldelta as u32;
                writeln!(instrs, "{lineno}:").unwrap();
                sdelta = lnotab_iter.next().unwrap_or(&0);
                ldelta = lnotab_iter.next().unwrap_or(&0);
            }
            if let (Some(op), Some(arg)) = (code_iter.next(), code_iter.next()) {
                match py_ver.and_then(|pv| pv.minor) {
                    Some(8) => self.read_instr_308(op, arg, idx, &mut instrs),
                    // Some(9) => self.read_instr_3_9(op, arg, idx, &mut instrs),
                    Some(10) => self.read_instr_310(op, arg, idx, &mut instrs),
                    Some(11) => self.read_instr_311(op, arg, idx, &mut instrs),
                    _ => {}
                }
                idx += 2;
                line_offset += 2;
            } else {
                break;
            }
        }
        instrs
    }

    fn read_instr_308(&self, op: &u8, arg: &u8, idx: usize, instrs: &mut String) {
        let op308 = Opcode308::from(*op);
        let s_op = op308.to_string();
        write!(instrs, "{:>15} {:<25}", idx, s_op).unwrap();
        if let Ok(op) = CommonOpcode::try_from(*op) {
            self.dump_additional_info(op, arg, idx, instrs);
        }
        match op308 {
            Opcode308::STORE_DEREF | Opcode308::LOAD_DEREF => {
                write!(
                    instrs,
                    "{arg} ({})",
                    self.freevars.get(*arg as usize).unwrap()
                )
                .unwrap();
            }
            Opcode308::LOAD_CLOSURE => {
                write!(
                    instrs,
                    "{arg} ({})",
                    self.cellvars.get(*arg as usize).unwrap()
                )
                .unwrap();
            }
            Opcode308::JUMP_ABSOLUTE => {
                write!(instrs, "{arg} (to {})", *arg as usize * 2).unwrap();
            }
            // REVIEW: *2?
            Opcode308::POP_JUMP_IF_FALSE | Opcode308::POP_JUMP_IF_TRUE => {
                write!(instrs, "{arg} (to {})", *arg as usize * 2).unwrap();
            }
            Opcode308::BINARY_ADD
            | Opcode308::BINARY_SUBTRACT
            | Opcode308::BINARY_MULTIPLY
            | Opcode308::BINARY_TRUE_DIVIDE => {
                write!(instrs, "{arg} ({:?})", TypePair::from(*arg)).unwrap();
            }
            _ => {}
        }
        instrs.push('\n');
    }

    fn read_instr_310(&self, op: &u8, arg: &u8, idx: usize, instrs: &mut String) {
        let op310 = Opcode310::from(*op);
        let s_op = op310.to_string();
        write!(instrs, "{:>15} {:<25}", idx, s_op).unwrap();
        if let Ok(op) = CommonOpcode::try_from(*op) {
            self.dump_additional_info(op, arg, idx, instrs);
        }
        match op310 {
            Opcode310::STORE_DEREF | Opcode310::LOAD_DEREF => {
                write!(
                    instrs,
                    "{arg} ({})",
                    self.freevars.get(*arg as usize).unwrap()
                )
                .unwrap();
            }
            Opcode310::LOAD_CLOSURE => {
                write!(
                    instrs,
                    "{arg} ({})",
                    self.cellvars.get(*arg as usize).unwrap()
                )
                .unwrap();
            }
            Opcode310::JUMP_ABSOLUTE => {
                write!(instrs, "{arg} (to {})", *arg as usize * 2).unwrap();
            }
            Opcode310::POP_JUMP_IF_FALSE | Opcode310::POP_JUMP_IF_TRUE => {
                write!(instrs, "{arg} (to {})", *arg as usize * 2).unwrap();
            }
            Opcode310::BINARY_ADD
            | Opcode310::BINARY_SUBTRACT
            | Opcode310::BINARY_MULTIPLY
            | Opcode310::BINARY_TRUE_DIVIDE => {
                write!(instrs, "{arg} ({:?})", TypePair::from(*arg)).unwrap();
            }
            Opcode310::SETUP_WITH => {
                write!(instrs, "{arg} (to {})", idx + *arg as usize * 2 + 2).unwrap();
            }
            _ => {}
        }
        instrs.push('\n');
    }

    fn read_instr_311(&self, op: &u8, arg: &u8, idx: usize, instrs: &mut String) {
        let op311 = Opcode311::from(*op);
        let s_op = op311.to_string();
        write!(instrs, "{idx:>15} {s_op:<26}").unwrap();
        if let Ok(op) = CommonOpcode::try_from(*op) {
            self.dump_additional_info(op, arg, idx, instrs);
        }
        match op311 {
            Opcode311::STORE_DEREF | Opcode311::LOAD_DEREF => {
                write!(
                    instrs,
                    "{arg} ({})",
                    self.varnames.get(*arg as usize).unwrap()
                )
                .unwrap();
            }
            Opcode311::MAKE_CELL | Opcode311::LOAD_CLOSURE => {
                write!(
                    instrs,
                    "{arg} ({})",
                    self.cellvars.get(*arg as usize).unwrap()
                )
                .unwrap();
            }
            Opcode311::POP_JUMP_FORWARD_IF_FALSE | Opcode311::POP_JUMP_FORWARD_IF_TRUE => {
                write!(instrs, "{arg} (to {})", idx + *arg as usize * 2 + 2).unwrap();
            }
            Opcode311::POP_JUMP_BACKWARD_IF_FALSE | Opcode311::POP_JUMP_BACKWARD_IF_TRUE => {
                write!(instrs, "{arg} (to {})", idx - *arg as usize * 2 + 2).unwrap();
            }
            Opcode311::JUMP_BACKWARD => {
                write!(instrs, "{arg} (to {})", idx - *arg as usize * 2 + 2).unwrap();
            }
            Opcode311::PRECALL
            | Opcode311::CALL
            | Opcode311::COPY
            | Opcode311::SWAP
            | Opcode311::COPY_FREE_VARS => {
                write!(instrs, "{arg}").unwrap();
            }
            Opcode311::KW_NAMES => {
                write!(
                    instrs,
                    "{arg} ({})",
                    self.consts.get(*arg as usize).unwrap()
                )
                .unwrap();
            }
            Opcode311::BINARY_OP => {
                write!(instrs, "{arg} ({:?})", BinOpCode::from(*arg)).unwrap();
            }
            _ => {}
        }
        instrs.push('\n');
    }

    fn dump_additional_info(&self, op: CommonOpcode, arg: &u8, idx: usize, instrs: &mut String) {
        match op {
            CommonOpcode::COMPARE_OP => {
                let op = match arg {
                    0 => "<",
                    1 => "<=",
                    2 => "==",
                    3 => "!=",
                    4 => ">",
                    5 => ">=",
                    _ => "?",
                };
                write!(instrs, "{arg} ({op})").unwrap();
            }
            CommonOpcode::STORE_NAME
            | CommonOpcode::LOAD_NAME
            | CommonOpcode::STORE_GLOBAL
            | CommonOpcode::LOAD_GLOBAL
            | CommonOpcode::STORE_ATTR
            | CommonOpcode::LOAD_ATTR
            | CommonOpcode::LOAD_METHOD
            | CommonOpcode::IMPORT_NAME
            | CommonOpcode::IMPORT_FROM => {
                write!(instrs, "{arg} ({})", self.names.get(*arg as usize).unwrap()).unwrap();
            }
            CommonOpcode::STORE_FAST | CommonOpcode::LOAD_FAST => {
                write!(
                    instrs,
                    "{arg} ({})",
                    self.varnames.get(*arg as usize).unwrap()
                )
                .unwrap();
            }
            CommonOpcode::LOAD_CONST => {
                write!(
                    instrs,
                    "{arg} ({})",
                    self.consts.get(*arg as usize).unwrap()
                )
                .unwrap();
            }
            CommonOpcode::FOR_ITER => {
                write!(instrs, "{arg} (to {})", idx + *arg as usize * 2 + 2).unwrap();
            }
            CommonOpcode::JUMP_FORWARD => {
                write!(instrs, "{arg} (to {})", idx + *arg as usize * 2 + 2).unwrap();
            }
            CommonOpcode::MAKE_FUNCTION => {
                let flag = match arg {
                    8 => "(closure)",
                    // TODO:
                    _ => "",
                };
                write!(instrs, "{arg} {flag}").unwrap();
            }
            other if other.take_arg() => {
                write!(instrs, "{arg}").unwrap();
            }
            _ => {}
        }
    }

    pub fn code_info(&self, py_ver: Option<PythonVersion>) -> String {
        let mut info = "".to_string();
        writeln!(info, "Disassembly of {self:?}:").unwrap();
        info += &self.attrs_info();
        info += &self.tables_info();
        info += &self.instr_info(py_ver);
        info.push('\n');
        for cons in self.consts.iter() {
            if let ValueObj::Code(c) = cons {
                info += &c.code_info(py_ver);
            }
        }
        info
    }
}
