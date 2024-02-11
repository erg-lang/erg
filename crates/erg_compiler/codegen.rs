//! generates `CodeObj` (equivalent to PyCodeObject of CPython) from `AST`.
//!
//! ASTからPythonバイトコード(コードオブジェクト)を生成する
use std::fmt;
use std::process;

use erg_common::cache::CacheSet;
use erg_common::config::ErgConfig;
use erg_common::env::erg_core_path;
use erg_common::error::{ErrorDisplay, Location};
use erg_common::fresh::SharedFreshNameGenerator;
use erg_common::io::Input;
use erg_common::opcode::{CommonOpcode, CompareOp};
use erg_common::opcode308::Opcode308;
use erg_common::opcode309::Opcode309;
use erg_common::opcode310::Opcode310;
use erg_common::opcode311::{BinOpCode, Opcode311};
use erg_common::option_enum_unwrap;
use erg_common::python_util::{env_python_version, PythonVersion};
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{
    debug_power_assert, fmt_option, fn_name, fn_name_full, impl_stream, log, set,
    switch_unreachable,
};
use erg_parser::ast::VisModifierSpec;
use erg_parser::ast::{DefId, DefKind};
use CommonOpcode::*;

use erg_parser::ast::{ParamPattern, TypeBoundSpecs, VarName};
use erg_parser::token::DOT;
use erg_parser::token::EQUAL;
use erg_parser::token::{Token, TokenKind};

use crate::compile::{AccessKind, Name, StoreLoadKind};
use crate::context::ControlKind;
use crate::error::CompileError;
use crate::hir::ArrayWithLength;
use crate::hir::DefaultParamSignature;
use crate::hir::{
    Accessor, Args, Array, BinOp, Block, Call, ClassDef, Def, DefBody, Expr, GuardClause,
    Identifier, Lambda, Literal, NonDefaultParamSignature, Params, PatchDef, PosArg, ReDef, Record,
    Signature, SubrSignature, Tuple, UnaryOp, VarSignature, HIR,
};
use crate::ty::codeobj::{CodeObj, CodeObjFlags, MakeFunctionFlags};
use crate::ty::value::{GenTypeObj, ValueObj};
use crate::ty::{HasType, Type, TypeCode, TypePair, VisibilityModifier};
use crate::varinfo::VarInfo;
use AccessKind::*;
use Type::*;

#[derive(Debug)]
pub enum RegisterNameKind {
    Import,
    Fast,
    NonFast,
}

use RegisterNameKind::*;

impl RegisterNameKind {
    pub const fn is_fast(&self) -> bool {
        matches!(self, Fast)
    }

    pub fn from_ident(ident: &Identifier) -> Self {
        if ident.vi.is_fast_value() {
            Fast
        } else {
            NonFast
        }
    }
}

/// patch method -> function
/// patch attr -> variable
fn debind(ident: &Identifier) -> Option<Str> {
    match ident.vi.py_name.as_ref().map(|s| &s[..]) {
        Some(name) if name.starts_with("Function::") => {
            Some(Str::from(name.replace("Function::", "")))
        }
        Some(patch_method) if patch_method.contains("::") || patch_method.contains('.') => {
            Some(Str::rc(patch_method))
        }
        _ => None,
    }
}

fn escape_name(
    name: &str,
    vis: &VisibilityModifier,
    def_line: u32,
    def_col: u32,
    is_attr: bool,
) -> Str {
    let name = name.replace('!', "__erg_proc__");
    let name = name.replace('$', "__erg_shared__");
    // For public APIs, mangling is not performed because `hasattr`, etc. cannot be used.
    // For automatically generated variables, there is no possibility of conflict.
    if vis.is_private() && !name.starts_with('%') {
        if is_attr {
            return Str::from(format!("::{name}"));
        }
        let line_mangling = match (def_line, def_col) {
            (0, 0) => "".to_string(),
            (0, _) => format!("_C{def_col}"),
            (_, 0) => format!("_L{def_line}"),
            (_, _) => format!("_L{def_line}_C{def_col}"),
        };
        Str::from(format!("::{name}{line_mangling}"))
    } else {
        Str::from(name)
    }
}

fn escape_ident(ident: Identifier) -> Str {
    let vis = ident.vis();
    if &ident.inspect()[..] == "Self" {
        // reference the self type or the self type constructor
        let ty = ident
            .vi
            .t
            .singleton_value()
            .and_then(|tp| <&Type>::try_from(tp).ok())
            .or_else(|| ident.vi.t.return_t())
            .unwrap();
        escape_name(
            &ty.local_name(),
            &ident.vi.vis.modifier,
            ident.vi.def_loc.loc.ln_begin().unwrap_or(0),
            ident.vi.def_loc.loc.col_begin().unwrap_or(0),
            ident.vi.kind.is_instance_attr(),
        )
    } else if let Some(py_name) = ident.vi.py_name {
        py_name
    } else if ident.vi.is_parameter() || ident.inspect() == "self" {
        ident.inspect().clone()
    } else {
        escape_name(
            ident.inspect(),
            vis,
            ident.vi.def_loc.loc.ln_begin().unwrap_or(0),
            ident.vi.def_loc.loc.col_begin().unwrap_or(0),
            ident.vi.kind.is_instance_attr(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct PyCodeGenUnit {
    pub(crate) id: usize,
    pub(crate) py_version: PythonVersion,
    pub(crate) codeobj: CodeObj,
    pub(crate) captured_vars: Vec<Str>,
    pub(crate) stack_len: u32, // the maximum stack size
    pub(crate) prev_lineno: u32,
    pub(crate) lasti: usize,
    pub(crate) prev_lasti: usize,
    pub(crate) _refs: Vec<ValueObj>, // ref-counted objects
}

impl PartialEq for PyCodeGenUnit {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Display for PyCodeGenUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CompilerUnit{{\nid: {}\ncode:\n{}\n}}",
            self.id,
            self.codeobj.code_info(Some(self.py_version))
        )
    }
}

impl PyCodeGenUnit {
    #[allow(clippy::too_many_arguments)]
    pub fn new<S: Into<Str>, T: Into<Str>>(
        id: usize,
        py_version: PythonVersion,
        params: Vec<Str>,
        kwonlyargcount: u32,
        filename: S,
        name: T,
        firstlineno: u32,
        flags: u32,
    ) -> Self {
        Self {
            id,
            py_version,
            codeobj: CodeObj::empty(params, kwonlyargcount, filename, name, firstlineno, flags),
            captured_vars: vec![],
            stack_len: 0,
            prev_lineno: firstlineno,
            lasti: 0,
            prev_lasti: 0,
            _refs: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct PyCodeGenStack(Vec<PyCodeGenUnit>);

impl_stream!(PyCodeGenStack, PyCodeGenUnit);

#[derive(Debug, Default)]
pub struct PyCodeGenerator {
    cfg: ErgConfig,
    pub(crate) py_version: PythonVersion,
    str_cache: CacheSet<str>,
    prelude_loaded: bool,
    mutate_op_loaded: bool,
    contains_op_loaded: bool,
    record_type_loaded: bool,
    module_type_loaded: bool,
    control_loaded: bool,
    convertors_loaded: bool,
    operators_loaded: bool,
    union_loaded: bool,
    fake_generic_loaded: bool,
    abc_loaded: bool,
    builtins_loaded: bool,
    unit_size: usize,
    units: PyCodeGenStack,
    fresh_gen: SharedFreshNameGenerator,
}

impl PyCodeGenerator {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            py_version: cfg.target_version.unwrap_or_else(|| {
                let Some(version) = env_python_version() else {
                    panic!("Failed to get python version");
                };
                version
            }),
            cfg,
            str_cache: CacheSet::new(),
            prelude_loaded: false,
            mutate_op_loaded: false,
            contains_op_loaded: false,
            record_type_loaded: false,
            module_type_loaded: false,
            control_loaded: false,
            convertors_loaded: false,
            operators_loaded: false,
            union_loaded: false,
            fake_generic_loaded: false,
            abc_loaded: false,
            builtins_loaded: false,
            unit_size: 0,
            units: PyCodeGenStack::empty(),
            fresh_gen: SharedFreshNameGenerator::new("codegen"),
        }
    }

    pub fn inherit(&self) -> Self {
        Self {
            cfg: self.cfg.clone(),
            py_version: self.py_version,
            str_cache: self.str_cache.clone(),
            prelude_loaded: false,
            mutate_op_loaded: false,
            contains_op_loaded: false,
            record_type_loaded: false,
            module_type_loaded: false,
            control_loaded: false,
            convertors_loaded: false,
            operators_loaded: false,
            union_loaded: false,
            fake_generic_loaded: false,
            abc_loaded: false,
            builtins_loaded: false,
            unit_size: 0,
            units: PyCodeGenStack::empty(),
            fresh_gen: self.fresh_gen.clone(),
        }
    }

    pub fn clear(&mut self) {
        self.units.clear();
    }

    pub fn initialize(&mut self) {
        self.prelude_loaded = false;
        self.mutate_op_loaded = false;
        self.contains_op_loaded = false;
        self.record_type_loaded = false;
        self.module_type_loaded = false;
        self.control_loaded = false;
        self.convertors_loaded = false;
        self.operators_loaded = false;
        self.union_loaded = false;
        self.fake_generic_loaded = false;
        self.abc_loaded = false;
        self.builtins_loaded = false;
    }

    #[inline]
    fn input(&self) -> &Input {
        &self.cfg.input
    }

    fn get_cached(&self, s: &str) -> Str {
        self.str_cache.get(s)
    }

    #[inline]
    fn toplevel_block(&self) -> &PyCodeGenUnit {
        self.units.first().unwrap()
    }

    #[inline]
    fn cur_block(&self) -> &PyCodeGenUnit {
        self.units.last().unwrap()
    }

    fn is_toplevel(&self) -> bool {
        self.cur_block() == self.toplevel_block()
    }

    fn captured_vars(&self) -> Vec<&str> {
        let mut caps = vec![];
        for unit in self.units.iter() {
            caps.extend(unit.captured_vars.iter().map(|s| &**s));
        }
        caps
    }

    #[inline]
    fn mut_cur_block(&mut self) -> &mut PyCodeGenUnit {
        self.units.last_mut().unwrap()
    }

    #[inline]
    fn cur_block_codeobj(&self) -> &CodeObj {
        &self.cur_block().codeobj
    }

    #[inline]
    fn mut_cur_block_codeobj(&mut self) -> &mut CodeObj {
        &mut self.mut_cur_block().codeobj
    }

    #[inline]
    fn toplevel_block_codeobj(&self) -> &CodeObj {
        &self.toplevel_block().codeobj
    }

    #[inline]
    fn stack_len(&self) -> u32 {
        self.cur_block().stack_len
    }

    #[inline]
    fn lasti(&self) -> usize {
        self.cur_block().lasti
    }

    #[inline]
    #[allow(dead_code)]
    fn debug_print(&mut self, value: impl Into<ValueObj>) {
        self.emit_load_const(value);
        self.emit_print_expr();
    }

    #[inline]
    #[allow(dead_code)]
    fn emit_print_expr(&mut self) {
        self.write_instr(Opcode311::PRINT_EXPR);
        self.write_arg(0);
        self.stack_dec();
    }

    fn _emit_compare_op(&mut self, op: CompareOp) {
        self.write_instr(Opcode311::COMPARE_OP);
        self.write_arg(op as usize);
        self.stack_dec();
        if self.py_version.minor >= Some(11) {
            self.write_bytes(&[0; 4]);
        }
    }

    /// shut down the interpreter
    #[allow(unused)]
    fn terminate(&mut self) {
        self.emit_push_null();
        self.emit_load_name_instr(Identifier::public("exit"));
        self.emit_load_const(1);
        if self.py_version.minor >= Some(11) {
            self.emit_precall_and_call(1);
        } else {
            self.write_instr(Opcode310::CALL_FUNCTION);
            self.write_arg(1);
        }
        self.stack_dec();
    }

    /// swap TOS and TOS1
    #[allow(unused)]
    fn rot2(&mut self) {
        if self.py_version.minor >= Some(11) {
            self.write_instr(Opcode311::SWAP);
            self.write_arg(2);
        } else {
            self.write_instr(Opcode310::ROT_TWO);
            self.write_arg(0);
        }
    }

    #[allow(unused)]
    fn dup_top(&mut self) {
        if self.py_version.minor >= Some(11) {
            self.write_instr(Opcode311::COPY);
            self.write_arg(1);
        } else {
            self.write_instr(Opcode310::DUP_TOP);
            self.write_arg(0);
        }
        self.stack_inc();
    }

    /// COPY(1) == DUP_TOP
    fn copy(&mut self, i: usize) {
        debug_power_assert!(i, >, 0);
        if self.py_version.minor >= Some(11) {
            self.write_instr(Opcode311::COPY);
            self.write_arg(i);
        } else if i == 1 {
            self.write_instr(Opcode310::DUP_TOP);
            self.write_arg(0);
        } else {
            todo!()
        }
        self.stack_inc();
    }

    /// 0 origin
    #[allow(dead_code)]
    fn peek_stack(&mut self, i: usize) {
        self.copy(i + 1);
        self.emit_print_expr();
    }

    fn fill_jump(&mut self, idx: usize, jump_to: usize) {
        let arg = if self.py_version.minor >= Some(10) {
            jump_to / 2
        } else {
            jump_to
        };
        let bytes = u16::try_from(arg).unwrap().to_be_bytes();
        *self.mut_cur_block_codeobj().code.get_mut(idx).unwrap() = bytes[0];
        *self.mut_cur_block_codeobj().code.get_mut(idx + 2).unwrap() = bytes[1];
    }

    /// returns: shift bytes
    fn calc_edit_jump(&mut self, idx: usize, jump_to: usize) -> usize {
        let arg = if self.py_version.minor >= Some(10) {
            jump_to / 2
        } else {
            jump_to
        };
        if idx == 0
            || !CommonOpcode::is_jump_op(*self.cur_block_codeobj().code.get(idx - 1).unwrap())
        {
            self.crash(&format!("calc_edit_jump: not jump op: {idx} {jump_to}"));
        }
        self.edit_code(idx, arg)
    }

    /// returns: shift bytes
    #[inline]
    fn edit_code(&mut self, idx: usize, arg: usize) -> usize {
        log!(err "editing: {idx} {arg}");
        match u8::try_from(arg) {
            Ok(u8code) => {
                *self.mut_cur_block_codeobj().code.get_mut(idx).unwrap() = u8code;
                0
            }
            Err(_e) => {
                // TODO: use u16 as long as possible
                // see write_arg's comment
                let bytes = u32::try_from(arg).unwrap().to_be_bytes();
                let before_instr = idx.saturating_sub(1);
                *self.mut_cur_block_codeobj().code.get_mut(idx).unwrap() = bytes[3];
                self.extend_arg(before_instr, &bytes)
            }
        }
    }

    // e.g. JUMP_ABSOLUTE 264, lasti: 100
    // 6 more instructions will be added after this, so 264 + 6 => 270
    // this is greater than u8::MAX, so we need to extend the arg
    // first, split `code + delta` into 4 u8s (as __Big__ endian)
    // 270.to_be_bytes() == [0, 0, 1, 14]
    // then, write the bytes in reverse order
    // [..., EXTENDED_ARG 0, EXTENDED_ARG 0, EXTENDED_ARG 1, JUMP_ABSOLUTE 14]
    /// returns: shift bytes
    #[inline]
    fn extend_arg(&mut self, before_instr: usize, bytes: &[u8]) -> usize {
        let mut shift_bytes = 0;
        for byte in bytes.iter().rev().skip(1) {
            self.mut_cur_block_codeobj()
                .code
                .insert(before_instr, *byte);
            self.mut_cur_block_codeobj()
                .code
                .insert(before_instr, CommonOpcode::EXTENDED_ARG as u8);
            self.mut_cur_block().lasti += 2;
            shift_bytes += 2;
        }
        shift_bytes
    }

    fn write_instr<C: Into<u8>>(&mut self, code: C) {
        self.mut_cur_block_codeobj().code.push(code.into());
        self.mut_cur_block().lasti += 1;
        // log!(info "wrote: {}", code);
    }

    /// returns: shift bytes
    fn write_arg(&mut self, code: usize) -> usize {
        match u8::try_from(code) {
            Ok(u8code) => {
                self.mut_cur_block_codeobj().code.push(u8code);
                self.mut_cur_block().lasti += 1;
                1
            }
            Err(_) => match u16::try_from(code) {
                Ok(_) => {
                    let delta =
                        if CommonOpcode::is_jump_op(*self.cur_block_codeobj().code.last().unwrap())
                        {
                            2
                        } else {
                            0
                        };
                    let arg = code + delta;
                    let bytes = u16::try_from(arg).unwrap().to_be_bytes(); // [u8; 2]
                    let before_instr = self.lasti().saturating_sub(1);
                    self.mut_cur_block_codeobj().code.push(bytes[1]);
                    self.mut_cur_block().lasti += 1;
                    self.extend_arg(before_instr, &bytes) + 1
                }
                Err(_) => {
                    let delta = 0;
                    if CommonOpcode::is_jump_op(*self.cur_block_codeobj().code.last().unwrap()) {
                        6
                    } else {
                        0
                    };
                    let arg = code + delta;
                    let bytes = u32::try_from(arg).unwrap().to_be_bytes(); // [u8; 4]
                    let before_instr = self.lasti().saturating_sub(1);
                    self.mut_cur_block_codeobj().code.push(bytes[3]);
                    self.mut_cur_block().lasti += 1;
                    self.extend_arg(before_instr, &bytes) + 1
                }
            },
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.mut_cur_block_codeobj().code.extend_from_slice(bytes);
        self.mut_cur_block().lasti += bytes.len();
    }

    fn stack_inc(&mut self) {
        self.mut_cur_block().stack_len += 1;
        if self.stack_len() > self.cur_block_codeobj().stacksize {
            self.mut_cur_block_codeobj().stacksize = self.stack_len();
        }
    }

    fn stack_dec(&mut self) {
        if self.stack_len() == 0 {
            let lasti = self.lasti();
            let last = self.cur_block_codeobj().code.last().unwrap();
            self.crash(&format!(
                "the stack size becomes -1\nlasti: {lasti}\nlast code: {last}"
            ));
        } else {
            self.mut_cur_block().stack_len -= 1;
        }
    }

    /// NOTE: For example, an operation that increases the stack by 2 and decreases it by 1 should be `stack_inc_n(2); stack_dec();` not `stack_inc(1);`.
    /// This is because the stack size will not increase correctly.
    fn stack_inc_n(&mut self, n: usize) {
        self.mut_cur_block().stack_len += n as u32;
        if self.stack_len() > self.cur_block_codeobj().stacksize {
            self.mut_cur_block_codeobj().stacksize = self.stack_len();
        }
    }

    fn stack_dec_n(&mut self, n: usize) {
        if n > 0 && self.stack_len() == 0 {
            let lasti = self.lasti();
            let last = self.cur_block_codeobj().code.last().unwrap();
            self.crash(&format!(
                "the stack size becomes -1\nlasti: {lasti}\nlast code: {last}"
            ));
        } else {
            self.mut_cur_block().stack_len -= n as u32;
        }
    }

    fn emit_load_const<C: Into<ValueObj>>(&mut self, cons: C) {
        let value: ValueObj = cons.into();
        let idx = self
            .mut_cur_block_codeobj()
            .consts
            .iter()
            .position(|c| c == &value)
            .unwrap_or_else(|| {
                self.mut_cur_block_codeobj().consts.push(value);
                self.mut_cur_block_codeobj().consts.len() - 1
            });
        self.write_instr(LOAD_CONST);
        self.write_arg(idx);
        self.stack_inc();
    }

    fn register_const<C: Into<ValueObj>>(&mut self, cons: C) -> usize {
        let value = cons.into();
        self.mut_cur_block_codeobj()
            .consts
            .iter()
            .position(|c| c == &value)
            .unwrap_or_else(|| {
                self.mut_cur_block_codeobj().consts.push(value);
                self.mut_cur_block_codeobj().consts.len() - 1
            })
    }

    fn local_search(&self, name: &str, acc_kind: AccessKind) -> Option<Name> {
        if self.py_version.minor < Some(11) {
            if let Some(idx) = self
                .cur_block_codeobj()
                .cellvars
                .iter()
                .position(|v| &**v == name)
            {
                return Some(Name::deref(idx));
            }
        }
        match acc_kind {
            AccessKind::Name => {
                if let Some(idx) = self
                    .cur_block_codeobj()
                    .freevars
                    .iter()
                    .position(|f| &**f == name)
                {
                    Some(Name::deref(idx))
                } else if let Some(idx) = self
                    .cur_block_codeobj()
                    .varnames
                    .iter()
                    .position(|v| &**v == name)
                {
                    if self.captured_vars().contains(&name) {
                        None
                    } else {
                        Some(Name::fast(idx))
                    }
                } else if let Some(idx) = self
                    .cur_block_codeobj()
                    .names
                    .iter()
                    .position(|n| &**n == name)
                {
                    if !self.is_toplevel() {
                        None
                    } else {
                        Some(Name::local(idx))
                    }
                } else {
                    None
                }
            }
            _ => self
                .cur_block_codeobj()
                .names
                .iter()
                .position(|n| &**n == name)
                .map(Name::local),
        }
    }

    // local_searchで見つからなかった変数を探索する
    fn rec_search(&mut self, name: &str) -> Option<StoreLoadKind> {
        // search_name()を実行した後なのでcur_blockはskipする
        for (nth_from_toplevel, block) in self.units.iter_mut().enumerate().rev().skip(1) {
            let block_is_toplevel = nth_from_toplevel == 0;
            if block.codeobj.cellvars.iter().any(|c| &**c == name) {
                return Some(StoreLoadKind::Deref);
            } else if let Some(idx) = block.codeobj.varnames.iter().position(|v| &**v == name) {
                if block_is_toplevel {
                    return Some(StoreLoadKind::Global);
                } else {
                    // the outer scope variable
                    let cellvar_name = block.codeobj.varnames.get(idx).unwrap().clone();
                    block.codeobj.cellvars.push(cellvar_name);
                    return Some(StoreLoadKind::Deref);
                }
            }
            if block_is_toplevel && block.codeobj.names.iter().any(|n| &**n == name) {
                return Some(StoreLoadKind::Global);
            }
        }
        // 見つからなかった変数(前方参照変数など)はグローバル
        Some(StoreLoadKind::Global)
    }

    fn register_name(&mut self, name: Str, kind: RegisterNameKind) -> Name {
        let current_is_toplevel = self.is_toplevel();
        match self.rec_search(&name) {
            Some(st @ (StoreLoadKind::Local | StoreLoadKind::Global)) => {
                if kind.is_fast() {
                    self.mut_cur_block_codeobj().varnames.push(name);
                    Name::fast(self.cur_block_codeobj().varnames.len() - 1)
                } else {
                    let st = if current_is_toplevel {
                        StoreLoadKind::Local
                    } else {
                        st
                    };
                    self.mut_cur_block_codeobj().names.push(name);
                    Name::new(st, self.cur_block_codeobj().names.len() - 1)
                }
            }
            Some(StoreLoadKind::Deref) => {
                self.mut_cur_block_codeobj().freevars.push(name.clone());
                if self.py_version.minor >= Some(11) {
                    // in 3.11 freevars are unified with varnames
                    self.mut_cur_block_codeobj().varnames.push(name);
                    Name::deref(self.cur_block_codeobj().varnames.len() - 1)
                } else {
                    // cellvarsのpushはrec_search()で行われる
                    Name::deref(self.cur_block_codeobj().freevars.len() - 1)
                }
            }
            None => {
                // new variable
                if current_is_toplevel {
                    self.mut_cur_block_codeobj().names.push(name);
                    Name::local(self.cur_block_codeobj().names.len() - 1)
                } else {
                    self.mut_cur_block_codeobj().varnames.push(name);
                    Name::fast(self.cur_block_codeobj().varnames.len() - 1)
                }
            }
            Some(_) => {
                switch_unreachable!()
            }
        }
    }

    fn register_attr(&mut self, name: Str) -> Name {
        self.mut_cur_block_codeobj().names.push(name);
        Name::local(self.cur_block_codeobj().names.len() - 1)
    }

    fn register_method(&mut self, name: Str) -> Name {
        self.mut_cur_block_codeobj().names.push(name);
        Name::local(self.cur_block_codeobj().names.len() - 1)
    }

    fn select_load_instr(&self, kind: StoreLoadKind, acc_kind: AccessKind) -> u8 {
        match kind {
            StoreLoadKind::Fast | StoreLoadKind::FastConst => LOAD_FAST as u8,
            StoreLoadKind::Global | StoreLoadKind::GlobalConst => LOAD_NAME as u8, //LOAD_GLOBAL as u8,
            StoreLoadKind::Deref | StoreLoadKind::DerefConst => {
                if self.py_version.minor >= Some(11) {
                    Opcode311::LOAD_DEREF as u8
                } else {
                    Opcode310::LOAD_DEREF as u8
                }
            }
            StoreLoadKind::Local | StoreLoadKind::LocalConst => match acc_kind {
                Name => LOAD_NAME as u8,
                UnboundAttr => LOAD_ATTR as u8,
                BoundAttr => LOAD_METHOD as u8,
            },
        }
    }

    fn select_store_instr(&self, kind: StoreLoadKind, acc_kind: AccessKind) -> u8 {
        match kind {
            StoreLoadKind::Fast => STORE_FAST as u8,
            StoreLoadKind::FastConst => STORE_FAST as u8, // ERG_STORE_FAST_IMMUT,
            // NOTE: First-time variables are treated as GLOBAL, but they are always first-time variables when assigned, so they are just NAME
            // NOTE: 初見の変数はGLOBAL扱いになるが、代入時は必ず初見であるので単なるNAME
            StoreLoadKind::Global | StoreLoadKind::GlobalConst => STORE_NAME as u8,
            StoreLoadKind::Deref | StoreLoadKind::DerefConst => {
                if self.py_version.minor >= Some(11) {
                    Opcode311::STORE_DEREF as u8
                } else {
                    Opcode310::STORE_DEREF as u8
                }
            }
            StoreLoadKind::Local | StoreLoadKind::LocalConst => {
                match acc_kind {
                    Name => STORE_NAME as u8,
                    UnboundAttr => STORE_ATTR as u8,
                    // cannot overwrite methods directly
                    BoundAttr => STORE_ATTR as u8,
                }
            }
        }
    }

    fn emit_load_name_instr(&mut self, ident: Identifier) {
        log!(info "entered {}({ident})", fn_name!());
        if &ident.inspect()[..] == "#ModuleType" && !self.module_type_loaded {
            self.load_module_type();
            self.module_type_loaded = true;
        }
        let kind = RegisterNameKind::from_ident(&ident);
        let escaped = escape_ident(ident);
        match &escaped[..] {
            "if__" | "for__" | "while__" | "with__" | "discard__" | "assert__" => {
                self.load_control();
            }
            "int__" | "nat__" | "str__" | "float__" => {
                self.load_convertors();
            }
            "add" | "sub" | "mul" | "truediv" | "floordiv" | "mod" | "pow" | "eq" | "ne" | "lt"
            | "le" | "gt" | "ge" | "and_" | "or_" | "xor" | "lshift" | "rshift" | "pos" | "neg"
            | "invert" | "is_" | "is_not" | "call" => {
                self.load_operators();
            }
            "CodeType" => {
                self.emit_global_import_items(
                    Identifier::public("types"),
                    vec![(Identifier::public("CodeType"), None)],
                );
            }
            // NoneType is not defined in the global scope, use `type(None)` instead
            "NoneType" => {
                self.emit_push_null();
                self.emit_load_name_instr(Identifier::public("type"));
                let none = Expr::Literal(Literal::new(ValueObj::None, Token::DUMMY));
                let args = Args::single(PosArg::new(none));
                self.emit_args_311(args, AccessKind::Name);
                return;
            }
            _ => {}
        }
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped, kind));
        let instr = self.select_load_instr(name.kind, Name);
        self.write_instr(instr);
        self.write_arg(name.idx);
        self.stack_inc();
        self.mut_cur_block_codeobj().stacksize += 2;
        if instr == LOAD_GLOBAL as u8 && self.py_version.minor >= Some(11) {
            self.write_bytes(&[0; 2]);
            self.write_bytes(&[0; 8]);
        }
    }

    fn emit_load_global_instr(&mut self, ident: Identifier) {
        log!(info "entered {} ({ident})", fn_name!());
        let escaped = escape_ident(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped, NonFast));
        let instr = LOAD_GLOBAL;
        self.write_instr(instr);
        self.write_arg(name.idx);
        self.stack_inc();
    }

    fn emit_import_name_instr(&mut self, ident: Identifier, items_len: usize) {
        log!(info "entered {}({ident})", fn_name!());
        let escaped = escape_ident(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped, Import));
        self.write_instr(IMPORT_NAME);
        self.write_arg(name.idx);
        self.stack_inc_n(items_len);
        self.stack_dec(); // (level + from_list) -> module object
    }

    fn emit_import_from_instr(&mut self, ident: Identifier) {
        log!(info "entered {}", fn_name!());
        let escaped = escape_ident(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped, Import));
        self.write_instr(IMPORT_FROM);
        self.write_arg(name.idx);
        // self.stack_inc(); (module object) -> attribute
    }

    fn emit_import_all_instr(&mut self, ident: Identifier) {
        log!(info "entered {}", fn_name!());
        self.emit_load_const(0i32); // escaping to call access `Nat` before importing `Nat`
        self.emit_load_const([Str::ever("*")]);
        let escaped = escape_ident(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped, Import));
        self.write_instr(IMPORT_NAME);
        self.write_arg(name.idx);
        self.stack_inc();
        self.write_instr(IMPORT_STAR);
        self.write_arg(0);
        self.stack_dec_n(3);
    }

    /// item: (name, renamed)
    fn emit_global_import_items(
        &mut self,
        module: Identifier,
        items: Vec<(Identifier, Option<Identifier>)>,
    ) {
        self.emit_load_const(0);
        let item_name_tuple = items
            .iter()
            .map(|ident| ValueObj::Str(ident.0.inspect().clone()))
            .collect::<Vec<_>>();
        let items_len = item_name_tuple.len();
        self.emit_load_const(item_name_tuple);
        self.emit_import_name_instr(module, items_len);
        for (item, renamed) in items.into_iter() {
            if let Some(renamed) = renamed {
                self.emit_import_from_instr(item);
                self.emit_store_global_instr(renamed);
            } else {
                self.emit_import_from_instr(item.clone());
                self.emit_store_global_instr(item);
            }
        }
        self.emit_pop_top(); // discard IMPORT_FROM object
    }

    fn emit_load_attr_instr(&mut self, ident: Identifier) {
        log!(info "entered {} ({ident})", fn_name!());
        let escaped = escape_ident(ident);
        let name = self
            .local_search(&escaped, UnboundAttr)
            .unwrap_or_else(|| self.register_attr(escaped));
        let instr = self.select_load_instr(name.kind, UnboundAttr);
        self.write_instr(instr);
        self.write_arg(name.idx);
        if self.py_version.minor >= Some(11) {
            self.write_bytes(&[0; 8]);
        }
    }

    fn emit_load_method_instr(&mut self, ident: Identifier) {
        log!(info "entered {} ({ident})", fn_name!());
        let escaped = escape_ident(ident);
        let name = self
            .local_search(&escaped, BoundAttr)
            .unwrap_or_else(|| self.register_method(escaped));
        let instr = self.select_load_instr(name.kind, BoundAttr);
        self.write_instr(instr);
        self.write_arg(name.idx);
        if self.py_version.minor >= Some(11) {
            self.stack_inc(); // instead of PUSH_NULL
            self.write_bytes(&[0; 20]);
        }
    }

    fn emit_store_instr(&mut self, ident: Identifier, acc_kind: AccessKind) {
        log!(info "entered {} ({ident})", fn_name!());
        let kind = RegisterNameKind::from_ident(&ident);
        let escaped = escape_ident(ident);
        let name = self.local_search(&escaped, acc_kind).unwrap_or_else(|| {
            if acc_kind.is_local() {
                self.register_name(escaped, kind)
            } else {
                self.register_attr(escaped)
            }
        });
        let instr = self.select_store_instr(name.kind, acc_kind);
        self.write_instr(instr);
        self.write_arg(name.idx);
        self.stack_dec();
        if instr == STORE_ATTR as u8 {
            if self.py_version.minor >= Some(11) {
                self.write_bytes(&[0; 8]);
            }
            self.stack_dec();
        } else if instr == STORE_FAST as u8 {
            self.mut_cur_block_codeobj().nlocals += 1;
        }
    }

    /// used for importing Erg builtin objects, etc. normally, this is not used
    // Ergの組み込みオブジェクトをimportするときなどに使う、通常は使わない
    fn emit_store_global_instr(&mut self, ident: Identifier) {
        log!(info "entered {} ({ident})", fn_name!());
        let escaped = escape_ident(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped, NonFast));
        let instr = STORE_GLOBAL;
        self.write_instr(instr);
        self.write_arg(name.idx);
        self.stack_dec();
    }

    /// Ergの文法として、属性への代入は存在しない(必ずオブジェクトはすべての属性を初期化しなくてはならないため)
    /// この関数はPythonへ落とし込むときに使う
    fn store_acc(&mut self, acc: Accessor) {
        log!(info "entered {} ({acc})", fn_name!());
        match acc {
            Accessor::Ident(ident) => {
                self.emit_store_instr(ident, Name);
            }
            Accessor::Attr(attr) => {
                self.emit_expr(*attr.obj);
                self.emit_store_instr(attr.ident, UnboundAttr);
            }
        }
    }

    fn emit_pop_top(&mut self) {
        self.write_instr(POP_TOP);
        self.write_arg(0);
        self.stack_dec();
    }

    fn cancel_if_pop_top(&mut self) {
        if self.cur_block_codeobj().code.len() < 2 {
            return;
        }
        let lasop_t_idx = self.cur_block_codeobj().code.len() - 2;
        if self.cur_block_codeobj().code.get(lasop_t_idx) == Some(&(POP_TOP as u8)) {
            self.mut_cur_block_codeobj().code.pop();
            self.mut_cur_block_codeobj().code.pop();
            self.mut_cur_block().lasti -= 2;
            self.stack_inc();
        }
    }

    /// Compileが継続不能になった際呼び出す
    /// 極力使わないこと
    #[track_caller]
    fn crash(&mut self, description: &str) -> ! {
        if cfg!(debug_assertions) || cfg!(feature = "debug") {
            println!("internal error: {description}");
            panic!("current block: {}", self.cur_block());
        } else {
            let err = CompileError::compiler_bug(
                0,
                self.input().clone(),
                Location::Unknown,
                fn_name!(),
                line!(),
            );
            err.write_to_stderr();
            process::exit(1);
        }
    }

    fn gen_param_names(&self, params: &Params) -> Vec<Str> {
        params
            .non_defaults
            .iter()
            .map(|p| (p.inspect().map(|s| &s[..]).unwrap_or("_"), &p.vi))
            .chain(
                params
                    .defaults
                    .iter()
                    .map(|p| (p.inspect().map(|s| &s[..]).unwrap_or("_"), &p.sig.vi)),
            )
            .chain(if let Some(var_args) = &params.var_params {
                vec![(
                    var_args.inspect().map(|s| &s[..]).unwrap_or("_"),
                    &var_args.vi,
                )]
            } else {
                vec![]
            })
            .chain(if let Some(kw_var_args) = &params.kw_var_params {
                vec![(
                    kw_var_args.inspect().map(|s| &s[..]).unwrap_or("_"),
                    &kw_var_args.vi,
                )]
            } else {
                vec![]
            })
            .enumerate()
            .map(|(i, (s, vi))| {
                if s == "_" {
                    format!("_{i}")
                } else {
                    escape_name(
                        s,
                        &VisibilityModifier::Public,
                        vi.def_loc.loc.ln_begin().unwrap_or(0),
                        vi.def_loc.loc.col_begin().unwrap_or(0),
                        false,
                    )
                    .to_string()
                }
            })
            .map(|s| self.get_cached(&s))
            .collect()
    }

    fn emit_acc(&mut self, acc: Accessor) {
        log!(info "entered {} ({acc})", fn_name!());
        let init_stack_len = self.stack_len();
        match acc {
            Accessor::Ident(ident) => {
                self.emit_load_name_instr(ident);
            }
            Accessor::Attr(mut a) => {
                // Python's namedtuple, a representation of Record, does not allow attribute names such as `::x`.
                // Since Erg does not allow the coexistence of private and public variables with the same name, there is no problem in this trick.
                let is_record = a.obj.ref_t().is_record();
                if is_record {
                    a.ident.raw.vis = VisModifierSpec::Public(DOT);
                }
                if let Some(varname) = debind(&a.ident) {
                    a.ident.raw.vis = VisModifierSpec::Private;
                    a.ident.raw.name = VarName::from_str(varname);
                    self.emit_load_name_instr(a.ident);
                } else {
                    self.emit_expr(*a.obj);
                    self.emit_load_attr_instr(a.ident);
                }
            }
        }
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    fn emit_def(&mut self, def: Def) {
        log!(info "entered {} ({})", fn_name!(), def.sig);
        if def.def_kind().is_trait() {
            return self.emit_trait_def(def);
        }
        match def.sig {
            Signature::Subr(sig) => self.emit_subr_def(None, sig, def.body),
            Signature::Var(sig) => self.emit_var_def(sig, def.body),
        }
    }

    fn emit_push_null(&mut self) {
        if self.py_version.minor >= Some(11) {
            self.write_instr(Opcode311::PUSH_NULL);
            self.write_arg(0);
            self.stack_inc();
        }
    }

    fn emit_precall_and_call(&mut self, argc: usize) {
        self.write_instr(Opcode311::PRECALL);
        self.write_arg(argc);
        self.write_arg(0);
        self.write_arg(0);
        self.write_instr(Opcode311::CALL);
        self.write_arg(argc);
        self.write_bytes(&[0; 8]);
        self.stack_dec();
    }

    fn emit_call_instr(&mut self, argc: usize, kind: AccessKind) {
        if self.py_version.minor >= Some(11) {
            self.emit_precall_and_call(argc);
        } else {
            match kind {
                AccessKind::BoundAttr => self.write_instr(Opcode310::CALL_METHOD),
                _ => self.write_instr(Opcode310::CALL_FUNCTION),
            }
            self.write_arg(argc);
        }
    }

    fn emit_call_kw_instr(&mut self, argc: usize, kws: Vec<ValueObj>) {
        if self.py_version.minor >= Some(11) {
            let idx = self.register_const(kws);
            self.write_instr(Opcode311::KW_NAMES);
            self.write_arg(idx);
            self.emit_precall_and_call(argc);
        } else {
            self.emit_load_const(kws);
            self.write_instr(Opcode310::CALL_FUNCTION_KW);
            self.write_arg(argc);
        }
    }

    fn emit_trait_def(&mut self, def: Def) {
        if !self.abc_loaded {
            self.load_abc();
            self.abc_loaded = true;
        }
        self.emit_push_null();
        self.write_instr(LOAD_BUILD_CLASS);
        self.write_arg(0);
        self.stack_inc();
        let kind = def.def_kind();
        let code = self.emit_trait_block(kind, &def.sig, def.body.block);
        self.emit_load_const(code);
        if self.py_version.minor < Some(11) {
            self.emit_load_const(def.sig.ident().inspect().clone());
        } else {
            self.stack_inc();
        }
        self.write_instr(MAKE_FUNCTION);
        self.write_arg(0);
        self.emit_load_const(def.sig.ident().inspect().clone());
        self.emit_load_name_instr(Identifier::private("#ABCMeta"));
        let subclasses_len = 1;
        self.emit_call_kw_instr(2 + subclasses_len, vec![ValueObj::from("metaclass")]);
        let sum = if self.py_version.minor >= Some(11) {
            1 + 2 + subclasses_len
        } else {
            1 + 2 + 1 + subclasses_len
        };
        self.stack_dec_n(sum - 1);
        self.emit_store_instr(def.sig.into_ident(), Name);
        self.stack_dec();
    }

    // trait variables will be removed
    // T = Trait {
    //     x = Int
    //     f = (self: Self) -> Int
    // }
    // ↓
    // class T(metaclass=ABCMeta):
    //    def f(): pass
    fn emit_trait_block(&mut self, kind: DefKind, sig: &Signature, mut block: Block) -> CodeObj {
        debug_assert_eq!(kind, DefKind::Trait);
        let name = sig.ident().inspect().clone();
        let Expr::Call(mut trait_call) = block.remove(0) else {
            unreachable!()
        };
        let req = if let Some(Expr::Record(req)) = trait_call.args.remove_left_or_key("Requirement")
        {
            req.attrs.into_iter()
        } else {
            vec![].into_iter()
        };
        self.unit_size += 1;
        let firstlineno = block
            .get(0)
            .and_then(|def| def.ln_begin())
            .unwrap_or_else(|| sig.ln_begin().unwrap());
        self.units.push(PyCodeGenUnit::new(
            self.unit_size,
            self.py_version,
            vec![],
            0,
            Str::rc(self.cfg.input.enclosed_name()),
            &name,
            firstlineno,
            0,
        ));
        let mod_name = self.toplevel_block_codeobj().name.clone();
        self.emit_load_const(mod_name);
        self.emit_store_instr(Identifier::public("__module__"), Name);
        self.emit_load_const(name);
        self.emit_store_instr(Identifier::public("__qualname__"), Name);
        for def in req {
            self.emit_empty_func(
                Some(sig.ident().inspect()),
                def.sig.into_ident(),
                Some(Identifier::private("#abstractmethod")),
            );
        }
        self.emit_load_const(ValueObj::None);
        self.write_instr(RETURN_VALUE);
        self.write_arg(0);
        if self.stack_len() > 1 {
            let block_id = self.cur_block().id;
            let stack_len = self.stack_len();
            CompileError::stack_bug(
                self.input().clone(),
                Location::Unknown,
                stack_len,
                block_id,
                fn_name_full!(),
            )
            .write_to_stderr();
            self.crash("error in emit_trait_block: invalid stack size");
        }
        // flagging
        if !self.cur_block_codeobj().varnames.is_empty() {
            self.mut_cur_block_codeobj().flags += CodeObjFlags::NewLocals as u32;
        }
        // end of flagging
        let unit = self.units.pop().unwrap();
        if !self.units.is_empty() {
            let ld = unit.prev_lineno - self.cur_block().prev_lineno;
            if ld != 0 {
                if let Some(l) = self.mut_cur_block_codeobj().lnotab.last_mut() {
                    *l += u8::try_from(ld).unwrap();
                }
                self.mut_cur_block().prev_lineno += ld;
            }
        }
        unit.codeobj
    }

    fn emit_empty_func(
        &mut self,
        class_name: Option<&str>,
        ident: Identifier,
        deco: Option<Identifier>,
    ) {
        log!(info "entered {} ({ident})", fn_name!());
        self.emit_push_null();
        let deco_is_some = deco.is_some();
        if let Some(deco) = deco {
            self.emit_load_name_instr(deco);
        }
        let code = {
            self.unit_size += 1;
            self.units.push(PyCodeGenUnit::new(
                self.unit_size,
                self.py_version,
                vec![],
                0,
                Str::rc(self.cfg.input.enclosed_name()),
                ident.inspect(),
                ident.ln_begin().unwrap_or(0),
                0,
            ));
            self.emit_load_const(ValueObj::None);
            self.write_instr(RETURN_VALUE);
            self.write_arg(0);
            let unit = self.units.pop().unwrap();
            if !self.units.is_empty() {
                let ld = unit
                    .prev_lineno
                    .saturating_sub(self.cur_block().prev_lineno);
                if ld != 0 {
                    if let Some(l) = self.mut_cur_block_codeobj().lnotab.last_mut() {
                        *l += u8::try_from(ld).unwrap();
                    }
                    self.mut_cur_block().prev_lineno += ld;
                }
            }
            unit.codeobj
        };
        self.emit_load_const(code);
        if self.py_version.minor < Some(11) {
            if let Some(class) = class_name {
                self.emit_load_const(Str::from(format!("{class}.{}", ident.inspect())));
            } else {
                self.emit_load_const(ident.inspect().clone());
            }
        } else {
            self.stack_inc();
        }
        self.write_instr(MAKE_FUNCTION);
        self.write_arg(0);
        if deco_is_some {
            self.emit_call_instr(1, Name);
            self.stack_dec();
        }
        // stack_dec: (<abstractmethod>) + <code obj> + <name> -> <function>
        self.stack_dec();
        self.emit_store_instr(ident, Name);
    }

    fn emit_class_def(&mut self, class_def: ClassDef) {
        log!(info "entered {} ({})", fn_name!(), class_def.sig);
        self.emit_push_null();
        let ident = class_def.sig.ident().clone();
        let require_or_sup = class_def.require_or_sup.clone().map(|x| *x);
        let obj = class_def.obj.clone();
        self.write_instr(LOAD_BUILD_CLASS);
        self.write_arg(0);
        self.stack_inc();
        let code = self.emit_class_block(class_def);
        self.emit_load_const(code);
        if self.py_version.minor < Some(11) {
            self.emit_load_const(ident.inspect().clone());
        } else {
            self.stack_inc();
        }
        self.write_instr(MAKE_FUNCTION);
        self.write_arg(0);
        self.emit_load_const(ident.inspect().clone());
        // LOAD subclasses
        let subclasses_len = self.emit_require_type(obj, require_or_sup);
        self.emit_call_instr(2 + subclasses_len, Name);
        self.stack_dec_n((1 + 2 + subclasses_len) - 1);
        self.emit_store_instr(ident, Name);
        self.stack_dec();
    }

    fn emit_patch_def(&mut self, patch_def: PatchDef) {
        log!(info "entered {} ({})", fn_name!(), patch_def.sig);
        for def in patch_def.methods {
            // Invert.
            //     invert self = ...
            // ↓
            // def Invert::invert(self): ...
            let Expr::Def(mut def) = def else { todo!() };
            let namespace = self.cur_block_codeobj().name.trim_start_matches("::");
            let name = format!(
                "{}{}{}",
                namespace,
                patch_def.sig.ident().to_string_notype(),
                def.sig.ident().to_string_notype()
            );
            def.sig.ident_mut().raw.name = VarName::from_str(Str::from(name));
            def.sig.ident_mut().raw.vis = VisModifierSpec::Private;
            self.emit_def(def);
        }
    }

    // NOTE: use `TypeVar`, `Generic` in `typing` module
    // fn emit_poly_type_def(&mut self, sig: SubrSignature, body: DefBody) {}

    /// Y = Inherit X => class Y(X): ...
    /// N = Inherit 1..10 => class N(Nat): ...
    fn emit_require_type(&mut self, obj: GenTypeObj, require_or_sup: Option<Expr>) -> usize {
        log!(info "entered {} ({obj}, {})", fn_name!(), fmt_option!(require_or_sup));
        match obj {
            GenTypeObj::Class(_) => 0,
            GenTypeObj::Subclass(typ) => {
                let require_or_sup = require_or_sup.unwrap();
                let typ = if require_or_sup.is_acc() {
                    require_or_sup
                } else {
                    Expr::try_from_type(typ.sup.typ().derefine()).unwrap_or(require_or_sup)
                };
                self.emit_expr(typ);
                1 // TODO: not always 1
            }
            _ => todo!(),
        }
    }

    fn emit_redef(&mut self, redef: ReDef) {
        log!(info "entered {} ({redef})", fn_name!());
        self.emit_simple_block(redef.block);
        self.store_acc(redef.attr);
    }

    fn emit_var_def(&mut self, sig: VarSignature, mut body: DefBody) {
        log!(info "entered {} ({sig} = {})", fn_name!(), body.block);
        if body.block.len() == 1 {
            self.emit_expr(body.block.remove(0));
        } else {
            self.emit_simple_block(body.block);
        }
        if sig.global {
            self.emit_store_global_instr(sig.ident);
        } else {
            self.emit_store_instr(sig.ident, Name);
        }
    }

    /// No parameter mangling is used
    /// so that Erg functions can be called from Python with keyword arguments.
    fn emit_params(
        &mut self,
        var_params: Option<&NonDefaultParamSignature>,
        defaults: Vec<DefaultParamSignature>,
        function_flag: &mut usize,
    ) {
        if var_params.is_some() && !defaults.is_empty() {
            let defaults_len = defaults.len();
            let names = defaults
                .iter()
                .map(|default| {
                    escape_name(
                        default.sig.inspect().map_or("_", |s| &s[..]),
                        &VisibilityModifier::Public,
                        0,
                        0,
                        false,
                    )
                })
                .collect::<Vec<_>>();
            defaults
                .into_iter()
                .for_each(|default| self.emit_expr(default.default_val));
            self.emit_load_const(names);
            self.stack_dec();
            self.write_instr(BUILD_CONST_KEY_MAP);
            self.write_arg(defaults_len);
            self.stack_dec_n(defaults_len - 1);
            *function_flag += MakeFunctionFlags::KwDefaults as usize;
        } else if !defaults.is_empty() {
            let defaults_len = defaults.len();
            defaults
                .into_iter()
                .for_each(|default| self.emit_expr(default.default_val));
            self.write_instr(BUILD_TUPLE);
            self.write_arg(defaults_len);
            self.stack_dec_n(defaults_len - 1);
            *function_flag += MakeFunctionFlags::Defaults as usize;
        }
    }

    fn emit_subr_def(&mut self, class_name: Option<&str>, sig: SubrSignature, body: DefBody) {
        log!(info "entered {} ({sig} = {})", fn_name!(), body.block);
        let name = sig.ident.inspect().clone();
        let mut make_function_flag = 0;
        let params = self.gen_param_names(&sig.params);
        let kwonlyargcount = if sig.params.var_params.is_some() {
            sig.params.defaults.len()
        } else {
            0
        };
        self.emit_params(
            sig.params.var_params.as_deref(),
            sig.params.defaults,
            &mut make_function_flag,
        );
        let mut flags = 0;
        if sig.params.var_params.is_some() {
            flags += CodeObjFlags::VarArgs as u32;
        }
        if sig.params.kw_var_params.is_some() {
            flags += CodeObjFlags::VarKeywords as u32;
        }
        let code = self.emit_block(
            body.block,
            sig.params.guards,
            Some(name.clone()),
            params,
            kwonlyargcount as u32,
            sig.captured_names.clone(),
            flags,
        );
        // code.flags += CodeObjFlags::Optimized as u32;
        self.enclose_vars(&mut make_function_flag);
        let n_decos = sig.decorators.len();
        for deco in sig.decorators {
            self.emit_expr(deco);
        }
        self.rewrite_captured_fast(&code);
        self.emit_load_const(code);
        if self.py_version.minor < Some(11) {
            if let Some(class) = class_name {
                self.emit_load_const(Str::from(format!("{class}.{name}")));
            } else {
                self.emit_load_const(name);
            }
        } else {
            self.stack_inc();
        }
        self.write_instr(MAKE_FUNCTION);
        self.write_arg(make_function_flag);
        for _ in 0..n_decos {
            let argc = if self.py_version.minor >= Some(11) {
                0
            } else {
                self.stack_dec();
                1
            };
            self.emit_call_instr(argc, Name);
        }
        // stack_dec: <code obj> + <name> -> <function>
        self.stack_dec();
        if make_function_flag
            & (MakeFunctionFlags::Defaults as usize | MakeFunctionFlags::KwDefaults as usize)
            != 0
        {
            self.stack_dec();
        }
        self.emit_store_instr(sig.ident, Name);
    }

    fn emit_lambda(&mut self, lambda: Lambda) {
        log!(info "entered {} ({lambda})", fn_name!());
        let init_stack_len = self.stack_len();
        let mut make_function_flag = 0;
        let params = self.gen_param_names(&lambda.params);
        let kwonlyargcount = if lambda.params.var_params.is_some() {
            lambda.params.defaults.len()
        } else {
            0
        };
        self.emit_params(
            lambda.params.var_params.as_deref(),
            lambda.params.defaults,
            &mut make_function_flag,
        );
        let flags = if lambda.params.var_params.is_some() {
            CodeObjFlags::VarArgs as u32
        } else {
            0
        };
        let code = self.emit_block(
            lambda.body,
            lambda.params.guards,
            Some(format!("<lambda_{}>", lambda.id).into()),
            params,
            kwonlyargcount as u32,
            lambda.captured_names.clone(),
            flags,
        );
        self.enclose_vars(&mut make_function_flag);
        self.rewrite_captured_fast(&code);
        self.emit_load_const(code);
        if self.py_version.minor < Some(11) {
            self.emit_load_const(format!("<lambda_{}>", lambda.id));
        } else {
            self.stack_inc();
        }
        self.write_instr(MAKE_FUNCTION);
        self.write_arg(make_function_flag);
        // stack_dec: <lambda code obj> + <name "<lambda>"> -> <function>
        self.stack_dec();
        if make_function_flag & MakeFunctionFlags::Defaults as usize != 0 {
            self.stack_dec();
        }
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    fn enclose_vars(&mut self, flag: &mut usize) {
        if !self.cur_block_codeobj().cellvars.is_empty() {
            let cellvars_len = self.cur_block_codeobj().cellvars.len();
            let cellvars = self.cur_block_codeobj().cellvars.clone();
            for (i, name) in cellvars.iter().enumerate() {
                // Since 3.11, LOAD_CLOSURE is simply an alias for LOAD_FAST.
                if self.py_version.minor >= Some(11) {
                    let idx = self
                        .cur_block_codeobj()
                        .varnames
                        .iter()
                        .position(|n| n == name)
                        .unwrap();
                    self.write_instr(Opcode311::LOAD_CLOSURE);
                    self.write_arg(idx);
                } else {
                    self.write_instr(Opcode310::LOAD_CLOSURE);
                    self.write_arg(i);
                }
            }
            self.write_instr(BUILD_TUPLE);
            self.write_arg(cellvars_len);
            *flag += MakeFunctionFlags::Closure as usize;
        }
    }

    /// ```erg
    /// f = i ->
    ///     log i
    ///     do i
    /// ```
    /// ↓
    /// ```pyc
    /// Disassembly of <code object f ...>
    /// 0 LOAD_NAME                0 (print)
    /// 2 LOAD_NAME                1 (Nat)
    /// 4 LOAD_DEREF               0 (::i) # not LOAD_FAST!
    /// 6 CALL_FUNCTION            1
    /// 8 CALL_FUNCTION            1
    /// 10 POP_TOP
    /// 12 LOAD_CLOSURE             0 (::i)
    /// 14 BUILD_TUPLE              1
    /// 16 LOAD_CONST               0 (<code object ...>)
    /// 18 LOAD_CONST               1 ("<lambda>")
    /// 20 MAKE_FUNCTION            8 (closure)
    /// 22 RETURN_VALUE
    /// ```
    fn rewrite_captured_fast(&mut self, code: &CodeObj) {
        if self.py_version.minor >= Some(11) {
            return;
        }
        let cellvars = self.cur_block_codeobj().cellvars.clone();
        for cellvar in cellvars {
            if code.freevars.iter().any(|n| n == &cellvar) {
                let old_idx = self
                    .cur_block_codeobj()
                    .varnames
                    .iter()
                    .position(|n| n == &cellvar)
                    .unwrap();
                let new_idx = self
                    .cur_block_codeobj()
                    .cellvars
                    .iter()
                    .position(|n| n == &cellvar)
                    .unwrap();
                self.mut_cur_block().captured_vars.push(cellvar);
                let mut op_idx = 0;
                while let Some([op, arg]) = self
                    .mut_cur_block_codeobj()
                    .code
                    .get_mut(op_idx..=op_idx + 1)
                {
                    match Opcode310::try_from(*op) {
                        Ok(Opcode310::LOAD_FAST) if *arg == old_idx as u8 => {
                            *op = Opcode310::LOAD_DEREF as u8;
                            *arg = new_idx as u8;
                        }
                        Ok(Opcode310::STORE_FAST) if *arg == old_idx as u8 => {
                            *op = Opcode310::STORE_DEREF as u8;
                            *arg = new_idx as u8;
                        }
                        _ => {}
                    }
                    op_idx += 2;
                }
            }
        }
    }

    fn emit_unaryop(&mut self, unary: UnaryOp) {
        log!(info "entered {} ({unary})", fn_name!());
        let init_stack_len = self.stack_len();
        let val_t = unary
            .info
            .t
            .non_default_params()
            .and_then(|tys| tys.first().map(|pt| pt.typ()))
            .unwrap_or(Type::FAILURE);
        let tycode = TypeCode::from(val_t);
        let instr = match &unary.op.kind {
            // TODO:
            TokenKind::PrePlus => UNARY_POSITIVE,
            TokenKind::PreMinus => UNARY_NEGATIVE,
            TokenKind::PreBitNot => UNARY_INVERT,
            TokenKind::Mutate => {
                if !self.mutate_op_loaded {
                    self.load_mutate_op();
                }
                if self.py_version.minor >= Some(11) {
                    self.emit_push_null();
                }
                self.emit_load_name_instr(Identifier::private("#mutate_operator"));
                NOP // ERG_MUTATE,
            }
            _ => {
                CompileError::feature_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    unary.op.loc(),
                    &unary.op.inspect().clone(),
                    String::from(unary.op.content),
                )
                .write_to_stderr();
                NOT_IMPLEMENTED
            }
        };
        self.emit_expr(*unary.expr);
        if instr != NOP {
            self.write_instr(instr);
            self.write_arg(tycode as usize);
        } else {
            if self.py_version.minor >= Some(11) {
                self.emit_precall_and_call(1);
            } else {
                self.write_instr(Opcode310::CALL_FUNCTION);
                self.write_arg(1);
            }
            self.stack_dec();
        }
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    fn emit_binop(&mut self, bin: BinOp) {
        log!(info "entered {} ({bin})", fn_name!());
        let init_stack_len = self.stack_len();
        // TODO: and/orのプリミティブ命令の実装
        // Range operators are not operators in Python
        match &bin.op.kind {
            // l..<r == range(l, r)
            TokenKind::RightOpen => {
                self.emit_push_null();
                self.emit_load_name_instr(Identifier::public("RightOpenRange"));
            }
            TokenKind::LeftOpen => {
                self.emit_push_null();
                self.emit_load_name_instr(Identifier::public("LeftOpenRange"));
            }
            TokenKind::Closed => {
                self.emit_push_null();
                self.emit_load_name_instr(Identifier::public("ClosedRange"));
            }
            TokenKind::Open => {
                self.emit_push_null();
                self.emit_load_name_instr(Identifier::public("OpenRange"));
            }
            // From 3.10, `or` can be used for types.
            // But Erg supports Python 3.7~, so we should use `typing.Union`.
            TokenKind::OrOp if bin.lhs.ref_t().is_type() => {
                self.load_union();
                let args = Args::pos_only(vec![PosArg::new(*bin.lhs), PosArg::new(*bin.rhs)], None);
                self.emit_push_null();
                self.emit_load_name_instr(Identifier::private("#UnionType"));
                self.emit_args_311(args, Name);
                return;
            }
            // short circuiting
            TokenKind::OrOp => {
                self.emit_expr(*bin.lhs);
                let idx = self.lasti();
                self.write_instr(EXTENDED_ARG);
                self.write_arg(0);
                self.write_instr(Opcode311::JUMP_IF_TRUE_OR_POP);
                self.write_arg(0);
                self.emit_expr(*bin.rhs);
                let arg = match self.py_version.minor {
                    Some(11) => self.lasti() - idx - 4,
                    _ => self.lasti(),
                };
                self.fill_jump(idx + 1, arg);
                self.stack_dec();
                return;
            }
            TokenKind::AndOp => {
                self.emit_expr(*bin.lhs);
                let idx = self.lasti();
                self.write_instr(EXTENDED_ARG);
                self.write_arg(0);
                self.write_instr(Opcode311::JUMP_IF_FALSE_OR_POP);
                self.write_arg(0);
                self.emit_expr(*bin.rhs);
                let arg = match self.py_version.minor {
                    Some(11) => self.lasti() - idx - 4,
                    _ => self.lasti(),
                };
                self.fill_jump(idx + 1, arg);
                self.stack_dec();
                return;
            }
            TokenKind::ContainsOp => {
                // if no-std, always `x contains y == True`
                if self.cfg.no_std {
                    self.emit_load_const(true);
                    return;
                }
                if !self.contains_op_loaded {
                    self.load_contains_op();
                }
                self.emit_push_null();
                self.emit_load_name_instr(Identifier::private("#contains_operator"));
            }
            _ => {}
        }
        let lhs_t = bin
            .info
            .t
            .non_default_params()
            .and_then(|tys| tys.first().map(|pt| pt.typ()))
            .unwrap_or(Type::FAILURE);
        let rhs_t = bin
            .info
            .t
            .non_default_params()
            .and_then(|tys| tys.get(1).map(|pt| pt.typ()))
            .unwrap_or(Type::FAILURE);
        let type_pair = TypePair::new(lhs_t, rhs_t);
        self.emit_expr(*bin.lhs);
        self.emit_expr(*bin.rhs);
        self.emit_binop_instr(bin.op, type_pair);
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    fn emit_binop_instr(&mut self, binop: Token, type_pair: TypePair) {
        if self.py_version.minor >= Some(11) {
            self.emit_binop_instr_311(binop, type_pair);
        } else if self.py_version.minor >= Some(9) {
            self.emit_binop_instr_309(binop, type_pair);
        } else {
            self.emit_binop_instr_307(binop, type_pair);
        }
    }

    fn emit_binop_instr_307(&mut self, binop: Token, type_pair: TypePair) {
        let instr = match &binop.kind {
            TokenKind::Plus => Opcode308::BINARY_ADD,
            TokenKind::Minus => Opcode308::BINARY_SUBTRACT,
            TokenKind::Star => Opcode308::BINARY_MULTIPLY,
            TokenKind::Slash => Opcode308::BINARY_TRUE_DIVIDE,
            TokenKind::FloorDiv => Opcode308::BINARY_FLOOR_DIVIDE,
            TokenKind::Pow => Opcode308::BINARY_POWER,
            TokenKind::Mod => Opcode308::BINARY_MODULO,
            TokenKind::AndOp | TokenKind::BitAnd => Opcode308::BINARY_AND,
            TokenKind::OrOp | TokenKind::BitOr => Opcode308::BINARY_OR,
            TokenKind::BitXor => Opcode308::BINARY_XOR,
            TokenKind::Less
            | TokenKind::LessEq
            | TokenKind::DblEq
            | TokenKind::NotEq
            | TokenKind::Gre
            | TokenKind::GreEq
            | TokenKind::InOp
            | TokenKind::NotInOp
            | TokenKind::IsOp
            | TokenKind::IsNotOp => Opcode308::COMPARE_OP,
            TokenKind::LeftOpen
            | TokenKind::RightOpen
            | TokenKind::Closed
            | TokenKind::Open
            | TokenKind::ContainsOp => Opcode308::CALL_FUNCTION, // ERG_BINARY_RANGE,
            _ => {
                CompileError::feature_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    binop.loc(),
                    &binop.inspect().clone(),
                    String::from(binop.content),
                )
                .write_to_stderr();
                Opcode308::NOT_IMPLEMENTED
            }
        };
        let arg = match &binop.kind {
            TokenKind::Less => 0,
            TokenKind::LessEq => 1,
            TokenKind::DblEq => 2,
            TokenKind::NotEq => 3,
            TokenKind::Gre => 4,
            TokenKind::GreEq => 5,
            TokenKind::InOp => 6,
            TokenKind::NotInOp => 7,
            TokenKind::IsOp => 8,
            TokenKind::IsNotOp => 9,
            TokenKind::LeftOpen
            | TokenKind::RightOpen
            | TokenKind::Closed
            | TokenKind::Open
            | TokenKind::ContainsOp => 2,
            _ => type_pair as usize,
        };
        self.write_instr(instr);
        self.write_arg(arg);
        self.stack_dec();
        match &binop.kind {
            TokenKind::LeftOpen
            | TokenKind::RightOpen
            | TokenKind::Open
            | TokenKind::Closed
            | TokenKind::ContainsOp => {
                self.stack_dec();
            }
            _ => {}
        }
    }

    fn emit_binop_instr_309(&mut self, binop: Token, type_pair: TypePair) {
        let instr = match &binop.kind {
            TokenKind::Plus => Opcode309::BINARY_ADD,
            TokenKind::Minus => Opcode309::BINARY_SUBTRACT,
            TokenKind::Star => Opcode309::BINARY_MULTIPLY,
            TokenKind::Slash => Opcode309::BINARY_TRUE_DIVIDE,
            TokenKind::FloorDiv => Opcode309::BINARY_FLOOR_DIVIDE,
            TokenKind::Pow => Opcode309::BINARY_POWER,
            TokenKind::Mod => Opcode309::BINARY_MODULO,
            TokenKind::AndOp | TokenKind::BitAnd => Opcode309::BINARY_AND,
            TokenKind::OrOp | TokenKind::BitOr => Opcode309::BINARY_OR,
            TokenKind::BitXor => Opcode309::BINARY_XOR,
            TokenKind::IsOp | TokenKind::IsNotOp => Opcode309::IS_OP,
            TokenKind::Less
            | TokenKind::LessEq
            | TokenKind::DblEq
            | TokenKind::NotEq
            | TokenKind::Gre
            | TokenKind::GreEq => Opcode309::COMPARE_OP,
            TokenKind::LeftOpen
            | TokenKind::RightOpen
            | TokenKind::Closed
            | TokenKind::Open
            | TokenKind::ContainsOp => Opcode309::CALL_FUNCTION, // ERG_BINARY_RANGE,
            _ => {
                CompileError::feature_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    binop.loc(),
                    &binop.inspect().clone(),
                    String::from(binop.content),
                )
                .write_to_stderr();
                Opcode309::NOT_IMPLEMENTED
            }
        };
        let arg = match &binop.kind {
            TokenKind::Less => 0,
            TokenKind::LessEq => 1,
            TokenKind::DblEq => 2,
            TokenKind::NotEq => 3,
            TokenKind::Gre => 4,
            TokenKind::GreEq => 5,
            TokenKind::IsOp => 0,
            TokenKind::IsNotOp => 1,
            TokenKind::LeftOpen
            | TokenKind::RightOpen
            | TokenKind::Closed
            | TokenKind::Open
            | TokenKind::ContainsOp => 2,
            _ => type_pair as usize,
        };
        self.write_instr(instr);
        self.write_arg(arg);
        self.stack_dec();
        match &binop.kind {
            TokenKind::LeftOpen
            | TokenKind::RightOpen
            | TokenKind::Open
            | TokenKind::Closed
            | TokenKind::ContainsOp => {
                self.stack_dec();
            }
            _ => {}
        }
    }

    fn emit_binop_instr_311(&mut self, binop: Token, type_pair: TypePair) {
        let instr = match &binop.kind {
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::FloorDiv
            | TokenKind::Pow
            | TokenKind::Mod
            | TokenKind::AndOp
            | TokenKind::OrOp
            | TokenKind::BitAnd
            | TokenKind::BitOr
            | TokenKind::BitXor => Opcode311::BINARY_OP,
            TokenKind::IsOp | TokenKind::IsNotOp => Opcode311::IS_OP,
            TokenKind::Less
            | TokenKind::LessEq
            | TokenKind::DblEq
            | TokenKind::NotEq
            | TokenKind::Gre
            | TokenKind::GreEq => Opcode311::COMPARE_OP,
            TokenKind::LeftOpen
            | TokenKind::RightOpen
            | TokenKind::Closed
            | TokenKind::Open
            | TokenKind::ContainsOp => {
                self.write_instr(Opcode311::PRECALL);
                self.write_arg(2);
                self.write_arg(0);
                self.write_arg(0);
                Opcode311::CALL
            }
            _ => {
                CompileError::feature_error(
                    self.cfg.input.clone(),
                    line!() as usize,
                    binop.loc(),
                    &binop.inspect().clone(),
                    String::from(binop.content),
                )
                .write_to_stderr();
                Opcode311::NOT_IMPLEMENTED
            }
        };
        let arg = match &binop.kind {
            TokenKind::Plus => BinOpCode::Add as usize,
            TokenKind::Minus => BinOpCode::Subtract as usize,
            TokenKind::Star => BinOpCode::Multiply as usize,
            TokenKind::Slash => BinOpCode::TrueDivide as usize,
            TokenKind::FloorDiv => BinOpCode::FloorDiv as usize,
            TokenKind::Pow => BinOpCode::Power as usize,
            TokenKind::Mod => BinOpCode::Remainder as usize,
            TokenKind::AndOp | TokenKind::BitAnd => BinOpCode::And as usize,
            TokenKind::OrOp | TokenKind::BitOr => BinOpCode::Or as usize,
            TokenKind::BitXor => BinOpCode::Xor as usize,
            TokenKind::Less => 0,
            TokenKind::LessEq => 1,
            TokenKind::DblEq => 2,
            TokenKind::NotEq => 3,
            TokenKind::Gre => 4,
            TokenKind::GreEq => 5,
            TokenKind::IsOp => 0,
            TokenKind::IsNotOp => 1,
            TokenKind::LeftOpen
            | TokenKind::RightOpen
            | TokenKind::Closed
            | TokenKind::Open
            | TokenKind::ContainsOp => 2,
            _ => type_pair as usize,
        };
        self.write_instr(instr);
        self.write_arg(arg);
        match instr {
            Opcode311::CALL => {
                self.write_bytes(&[0; 8]);
            }
            Opcode311::BINARY_OP => {
                self.write_bytes(&[0; 2]);
            }
            Opcode311::COMPARE_OP => {
                self.write_bytes(&[0; 4]);
            }
            _ => {}
        }
        self.stack_dec();
        match &binop.kind {
            TokenKind::LeftOpen
            | TokenKind::RightOpen
            | TokenKind::Open
            | TokenKind::Closed
            | TokenKind::ContainsOp => {
                self.stack_dec();
                if self.py_version.minor >= Some(11) {
                    self.stack_dec();
                }
            }
            _ => {}
        }
    }

    fn emit_del_instr(&mut self, mut args: Args) {
        let Some(Expr::Accessor(Accessor::Ident(ident))) = args.remove_left_or_key("obj") else {
            log!(err "del instruction requires an identifier");
            return;
        };
        log!(info "entered {} ({ident})", fn_name!());
        let escaped = escape_ident(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped, Fast));
        self.write_instr(DELETE_NAME);
        self.write_arg(name.idx);
        self.emit_load_const(ValueObj::None);
    }

    fn emit_not_instr(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        let expr = args.remove_left_or_key("b").unwrap();
        self.emit_expr(expr);
        self.write_instr(UNARY_NOT);
        self.write_arg(0);
    }

    fn emit_discard_instr(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        while let Some(arg) = args.try_remove(0) {
            self.emit_expr(arg);
            self.emit_pop_top();
        }
        self.emit_load_const(ValueObj::None);
    }

    fn deopt_instr(&mut self, kind: ControlKind, args: Args) {
        if !self.control_loaded {
            self.load_control();
        }
        let local = match kind {
            ControlKind::If => Identifier::public("if__"),
            ControlKind::For => Identifier::public("for__"),
            ControlKind::While => Identifier::public("while__"),
            ControlKind::With => Identifier::public("with__"),
            ControlKind::Discard => Identifier::public("discard__"),
            ControlKind::Assert => Identifier::public("assert__"),
            kind => todo!("{kind:?}"),
        };
        self.emit_call_local(local, args);
    }

    fn emit_if_instr(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        let init_stack_len = self.stack_len();
        let cond = args.remove(0);
        self.emit_expr(cond);
        let idx_pop_jump_if_false = self.lasti();
        self.write_instr(EXTENDED_ARG);
        self.write_arg(0);
        // Opcode310::POP_JUMP_IF_FALSE == Opcode311::POP_JUMP_FORWARD_IF_FALSE
        self.write_instr(Opcode310::POP_JUMP_IF_FALSE);
        // cannot detect where to jump to at this moment, so put as 0
        self.write_arg(0);
        match args.remove(0) {
            // then block
            Expr::Lambda(lambda) => {
                // let params = self.gen_param_names(&lambda.params);
                self.emit_simple_block(lambda.body);
            }
            other => {
                self.emit_expr(other);
            }
        }
        if args.get(0).is_some() {
            let idx_jump_forward = self.lasti();
            self.write_instr(EXTENDED_ARG);
            self.write_arg(0);
            self.write_instr(Opcode308::JUMP_FORWARD); // jump to end
            self.write_arg(0);
            // else block
            let idx_else_begin = match self.py_version.minor {
                Some(11) => self.lasti() - idx_pop_jump_if_false - 4,
                Some(7..=10) => self.lasti(),
                _ => self.lasti(),
            };
            self.fill_jump(idx_pop_jump_if_false + 1, idx_else_begin);
            match args.remove(0) {
                Expr::Lambda(lambda) => {
                    // let params = self.gen_param_names(&lambda.params);
                    self.emit_simple_block(lambda.body);
                }
                other => {
                    self.emit_expr(other);
                }
            }
            let idx_end = self.lasti();
            self.fill_jump(idx_jump_forward + 1, idx_end - idx_jump_forward - 2 - 1);
            // FIXME: this is a hack to make sure the stack is balanced
            while self.stack_len() != init_stack_len + 1 {
                self.stack_dec();
            }
        } else {
            self.write_instr(Opcode311::JUMP_FORWARD);
            let jump_to = match self.py_version.minor {
                Some(11 | 10) => 1,
                _ => 2,
            };
            self.write_arg(jump_to);
            // no else block
            let idx_end = if self.py_version.minor >= Some(11) {
                self.lasti() - idx_pop_jump_if_false - 3
            } else {
                self.lasti()
            };
            self.fill_jump(idx_pop_jump_if_false + 1, idx_end);
            self.emit_load_const(ValueObj::None);
            while self.stack_len() != init_stack_len + 1 {
                self.stack_dec();
            }
        }
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    fn emit_for_instr(&mut self, mut args: Args) {
        log!(info "entered {} ({})", fn_name!(), args);
        if !matches!(args.get(1).unwrap(), Expr::Lambda(_)) {
            return self.deopt_instr(ControlKind::For, args);
        }
        let _init_stack_len = self.stack_len();
        let iterable = args.remove(0);
        self.emit_expr(iterable);
        self.write_instr(GET_ITER);
        self.write_arg(0);
        let idx_for_iter = self.lasti();
        self.write_instr(EXTENDED_ARG);
        self.write_arg(0);
        self.write_instr(FOR_ITER);
        self.stack_inc();
        // FOR_ITER pushes a value onto the stack, but we can't know how many
        // but after executing this instruction, stack_len should be 1
        // cannot detect where to jump to at this moment, so put as 0
        self.write_arg(0);
        let Expr::Lambda(lambda) = args.remove(0) else {
            unreachable!()
        };
        // If there is nothing on the stack at the start, init_stack_len == 2 (an iterator and the first iterator value)
        let init_stack_len = self.stack_len();
        // store the iterator value, stack_len == 1 or 2 in the end
        self.emit_control_block(lambda.body, lambda.params);
        if self.stack_len() > init_stack_len - 1 {
            self.emit_pop_top();
        }
        debug_assert_eq!(self.stack_len(), init_stack_len - 1); // the iterator is remained
        let idx = self.lasti();
        self.write_instr(EXTENDED_ARG);
        self.write_arg(0);
        match self.py_version.minor {
            Some(11) => {
                self.write_instr(Opcode311::JUMP_BACKWARD);
                self.write_arg(0);
                self.fill_jump(idx + 1, self.lasti() - idx_for_iter);
            }
            Some(7..=10) => {
                self.write_instr(Opcode309::JUMP_ABSOLUTE);
                self.write_arg(0);
                self.fill_jump(idx + 1, idx_for_iter);
            }
            _ => todo!("not supported Python version"),
        }
        let idx_end = self.lasti();
        self.fill_jump(idx_for_iter + 1, idx_end - idx_for_iter - 2 - 2);
        self.stack_dec();
        self.emit_load_const(ValueObj::None);
        debug_assert_eq!(self.stack_len(), _init_stack_len + 1);
    }

    fn emit_while_instr(&mut self, mut args: Args) {
        log!(info "entered {} ({})", fn_name!(), args);
        if !matches!(args.get(1).unwrap(), Expr::Lambda(_)) {
            return self.deopt_instr(ControlKind::While, args);
        }
        let _init_stack_len = self.stack_len();
        // e.g. is_foo!: () => Bool, do!(is_bar)
        let cond_block = args.remove(0);
        let cond = match cond_block {
            Expr::Lambda(mut lambda) => lambda.body.remove(0),
            Expr::Accessor(acc) => Expr::Accessor(acc).call_expr(Args::empty()),
            _ => todo!(),
        };
        // Evaluate again at the end of the loop
        self.emit_expr(cond.clone());
        let idx_while = self.lasti();
        self.write_instr(EXTENDED_ARG);
        self.write_arg(0);
        self.write_instr(Opcode310::POP_JUMP_IF_FALSE);
        self.write_arg(0);
        self.stack_dec();
        let Expr::Lambda(lambda) = args.remove(0) else {
            unreachable!()
        };
        let init_stack_len = self.stack_len();
        self.emit_control_block(lambda.body, lambda.params);
        if self.stack_len() > init_stack_len {
            self.emit_pop_top();
        }
        self.emit_expr(cond);
        let idx = self.lasti();
        self.write_instr(EXTENDED_ARG);
        self.write_arg(0);
        let arg = if self.py_version.minor >= Some(11) {
            let arg = self.lasti() - (idx_while + 2);
            self.write_instr(Opcode311::POP_JUMP_BACKWARD_IF_TRUE);
            self.write_arg(0);
            arg
        } else {
            self.write_instr(Opcode310::POP_JUMP_IF_TRUE);
            self.write_arg(0);
            idx_while + 4
        };
        self.fill_jump(idx + 1, arg);
        self.stack_dec();
        let idx_end = match self.py_version.minor {
            Some(11) => self.lasti() - idx_while - 3,
            _ => self.lasti(),
        };
        self.fill_jump(idx_while + 1, idx_end);
        self.emit_load_const(ValueObj::None);
        debug_assert_eq!(self.stack_len(), _init_stack_len + 1);
    }

    fn emit_match_instr(&mut self, mut args: Args, _use_erg_specific: bool) {
        log!(info "entered {}", fn_name!());
        let init_stack_len = self.stack_len();
        let expr = args.remove(0);
        self.emit_expr(expr);
        let len = args.len();
        let mut jump_forward_points = vec![];
        while let Some(expr) = args.try_remove(0) {
            if len > 1 && !args.is_empty() {
                self.dup_top();
            }
            // compilerで型チェック済み(可読性が下がるため、matchでNamedは使えない)
            let Expr::Lambda(lambda) = expr else {
                unreachable!()
            };
            debug_power_assert!(lambda.params.len(), ==, 1);
            if !lambda.params.defaults.is_empty() {
                todo!("default values in match expression are not supported yet")
            }
            let pop_jump_points = self.emit_match_pattern(lambda.params, args.is_empty());
            self.emit_simple_block(lambda.body);
            // If we move on to the next arm, the stack size will increase
            // so `self.stack_dec();` for now (+1 at the end).
            self.stack_dec();
            for pop_jump_point in pop_jump_points {
                let idx = match self.py_version.minor {
                    Some(11) => self.lasti() - pop_jump_point,
                    Some(10) => self.lasti() + 4,
                    _ => self.lasti() + 4,
                };
                self.fill_jump(pop_jump_point + 1, idx); // jump to POP_TOP
            }
            jump_forward_points.push(self.lasti());
            self.write_instr(EXTENDED_ARG);
            self.write_arg(0);
            self.write_instr(Opcode308::JUMP_FORWARD); // jump to the end
            self.write_arg(0);
        }
        let lasti = self.lasti();
        for jump_point in jump_forward_points.into_iter() {
            let jump_to = match self.py_version.minor {
                Some(11 | 10) => lasti - jump_point - 2 - 2,
                _ => lasti - jump_point - 2 - 2,
            };
            self.fill_jump(jump_point + 1, jump_to);
        }
        self.stack_inc();
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    /// return `None` if the arm is the last one
    fn emit_match_pattern(&mut self, mut params: Params, is_last_arm: bool) -> Vec<usize> {
        let param = params.non_defaults.remove(0);
        log!(info "entered {}({param})", fn_name!());
        match &param.raw.pat {
            ParamPattern::VarName(name) => {
                let ident = erg_parser::ast::Identifier::private_from_varname(name.clone());
                let ident = Identifier::new(ident, None, param.vi);
                self.emit_store_instr(ident, AccessKind::Name);
            }
            ParamPattern::Discard(_) => {
                self.emit_pop_top();
            }
            _other => unreachable!(),
        }
        let mut pop_jump_points = vec![];
        let last = params.guards.len().saturating_sub(1);
        for (i, mut guard) in params.guards.into_iter().enumerate() {
            if let GuardClause::Condition(Expr::BinOp(BinOp {
                op:
                    Token {
                        kind: TokenKind::ContainsOp,
                        ..
                    },
                lhs,
                rhs,
                ..
            })) = &mut guard
            {
                // Guards have not been checked.
                // Therefore, an invalid guard may be generated.
                if lhs.var_info().is_some_and(|vi| vi == &VarInfo::ILLEGAL) {
                    continue;
                }
                if rhs.var_info().is_some_and(|vi| vi == &VarInfo::ILLEGAL) {
                    continue;
                }
                // *lhs.ref_mut_t().unwrap() = Type::Obj;
                *rhs.ref_mut_t().unwrap() = Type::Obj;
            }
            match guard {
                GuardClause::Bind(def) => {
                    self.emit_def(def);
                }
                GuardClause::Condition(cond) => {
                    self.emit_expr(cond);
                    if is_last_arm {
                        self.emit_pop_top();
                    } else {
                        pop_jump_points.push(self.lasti());
                        // HACK: match branches often jump very far (beyond the u8 range),
                        // so the jump destination should be reserved as the u16 range.
                        // Other jump instructions may need to be replaced by this way.
                        self.write_instr(EXTENDED_ARG);
                        self.write_arg(0);
                        // in 3.11, POP_JUMP_IF_FALSE is replaced with POP_JUMP_FORWARD_IF_FALSE
                        // but the numbers are the same, only the way the jumping points are calculated is different.
                        self.write_instr(Opcode310::POP_JUMP_IF_FALSE); // jump to the next case
                        self.write_arg(0);
                        // if matched, pop original
                        if i == last {
                            self.emit_pop_top();
                        } else {
                            self.stack_dec();
                        }
                    }
                }
            }
        }
        pop_jump_points
    }

    fn emit_with_instr_311(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        if !matches!(args.get(1).unwrap(), Expr::Lambda(_)) {
            return self.deopt_instr(ControlKind::With, args);
        }
        let expr = args.remove(0);
        let Expr::Lambda(lambda) = args.remove(0) else {
            unreachable!()
        };
        let params = self.gen_param_names(&lambda.params);
        self.emit_expr(expr);
        self.write_instr(Opcode311::BEFORE_WITH);
        self.write_arg(0);
        // push __exit__, __enter__() to the stack
        self.stack_inc_n(2);
        let lambda_line = lambda.body.last().unwrap().ln_begin().unwrap_or(0);
        self.emit_with_block(lambda.body, params);
        let stash = Identifier::private_with_line(self.fresh_gen.fresh_varname(), lambda_line);
        self.emit_store_instr(stash.clone(), Name);
        self.emit_load_const(ValueObj::None);
        self.emit_load_const(ValueObj::None);
        self.emit_load_const(ValueObj::None);
        self.emit_precall_and_call(2);
        self.emit_pop_top();
        let idx_jump_forward = self.lasti();
        self.write_instr(Opcode311::JUMP_FORWARD);
        self.write_arg(0);
        self.write_instr(Opcode311::PUSH_EXC_INFO);
        self.write_arg(0);
        self.write_instr(Opcode309::WITH_EXCEPT_START);
        self.write_arg(0);
        self.write_instr(Opcode311::POP_JUMP_FORWARD_IF_TRUE);
        self.write_arg(4);
        self.write_instr(Opcode311::RERAISE);
        self.write_arg(0);
        self.write_instr(Opcode311::COPY);
        self.write_arg(3);
        self.write_instr(Opcode311::POP_EXCEPT);
        self.write_arg(0);
        self.write_instr(Opcode311::RERAISE);
        self.write_arg(1);
        self.emit_pop_top();
        self.write_instr(Opcode311::POP_EXCEPT);
        self.write_arg(0);
        self.emit_pop_top();
        self.emit_pop_top();
        self.calc_edit_jump(idx_jump_forward + 1, self.lasti() - idx_jump_forward - 2);
        self.emit_load_name_instr(stash);
    }

    fn emit_with_instr_310(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        if !matches!(args.get(1).unwrap(), Expr::Lambda(_)) {
            return self.deopt_instr(ControlKind::With, args);
        }
        let expr = args.remove(0);
        let Expr::Lambda(lambda) = args.remove(0) else {
            unreachable!()
        };
        let params = self.gen_param_names(&lambda.params);
        self.emit_expr(expr);
        let idx_setup_with = self.lasti();
        self.write_instr(Opcode310::SETUP_WITH);
        self.write_arg(0);
        // push __exit__, __enter__() to the stack
        self.stack_inc_n(2);
        let lambda_line = lambda.body.last().unwrap().ln_begin().unwrap_or(0);
        self.emit_with_block(lambda.body, params);
        let stash = Identifier::private_with_line(self.fresh_gen.fresh_varname(), lambda_line);
        self.emit_store_instr(stash.clone(), Name);
        self.write_instr(POP_BLOCK);
        self.write_arg(0);
        self.emit_load_const(ValueObj::None);
        self.write_instr(Opcode310::DUP_TOP);
        self.write_arg(0);
        self.stack_inc();
        self.write_instr(Opcode310::DUP_TOP);
        self.write_arg(0);
        self.stack_inc();
        self.write_instr(Opcode310::CALL_FUNCTION);
        self.write_arg(3);
        self.stack_dec_n((1 + 3) - 1);
        self.emit_pop_top();
        let idx_jump_forward = self.lasti();
        self.write_instr(Opcode310::JUMP_FORWARD);
        self.write_arg(0);
        self.edit_code(idx_setup_with + 1, (self.lasti() - idx_setup_with - 2) / 2);
        self.write_instr(Opcode310::WITH_EXCEPT_START);
        self.write_arg(0);
        let idx_pop_jump_if_true = self.lasti();
        self.write_instr(Opcode310::POP_JUMP_IF_TRUE);
        self.write_arg(0);
        self.write_instr(Opcode310::RERAISE);
        self.write_arg(1);
        self.edit_code(idx_pop_jump_if_true + 1, self.lasti() / 2);
        // self.emit_pop_top();
        // self.emit_pop_top();
        self.emit_pop_top();
        self.write_instr(Opcode310::POP_EXCEPT);
        self.write_arg(0);
        let idx_end = self.lasti();
        self.edit_code(idx_jump_forward + 1, (idx_end - idx_jump_forward - 2) / 2);
        self.emit_load_name_instr(stash);
    }

    fn emit_with_instr_309(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        if !matches!(args.get(1).unwrap(), Expr::Lambda(_)) {
            return self.deopt_instr(ControlKind::With, args);
        }
        let expr = args.remove(0);
        let Expr::Lambda(lambda) = args.remove(0) else {
            unreachable!()
        };
        let params = self.gen_param_names(&lambda.params);
        self.emit_expr(expr);
        let idx_setup_with = self.lasti();
        self.write_instr(Opcode310::SETUP_WITH);
        self.write_arg(0);
        // push __exit__, __enter__() to the stack
        self.stack_inc_n(2);
        let lambda_line = lambda.body.last().unwrap().ln_begin().unwrap_or(0);
        self.emit_with_block(lambda.body, params);
        let stash = Identifier::private_with_line(self.fresh_gen.fresh_varname(), lambda_line);
        self.emit_store_instr(stash.clone(), Name);
        self.write_instr(POP_BLOCK);
        self.write_arg(0);
        self.emit_load_const(ValueObj::None);
        self.write_instr(Opcode310::DUP_TOP);
        self.write_arg(0);
        self.stack_inc();
        self.write_instr(Opcode310::DUP_TOP);
        self.write_arg(0);
        self.stack_inc();
        self.write_instr(Opcode310::CALL_FUNCTION);
        self.write_arg(3);
        self.stack_dec_n((1 + 3) - 1);
        self.emit_pop_top();
        let idx_jump_forward = self.lasti();
        self.write_instr(Opcode311::JUMP_FORWARD);
        self.write_arg(0);
        self.edit_code(idx_setup_with + 1, self.lasti() - idx_setup_with - 2);
        self.write_instr(Opcode310::WITH_EXCEPT_START);
        self.write_arg(0);
        let idx_pop_jump_if_true = self.lasti();
        self.write_instr(Opcode310::POP_JUMP_IF_TRUE);
        self.write_arg(0);
        self.write_instr(Opcode309::RERAISE);
        self.write_arg(1);
        self.edit_code(idx_pop_jump_if_true + 1, self.lasti());
        // self.emit_pop_top();
        // self.emit_pop_top();
        self.emit_pop_top();
        self.write_instr(Opcode310::POP_EXCEPT);
        self.write_arg(0);
        let idx_end = self.lasti();
        self.edit_code(idx_jump_forward + 1, idx_end - idx_jump_forward - 2);
        self.emit_load_name_instr(stash);
    }

    fn emit_with_instr_308(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        if !matches!(args.get(1).unwrap(), Expr::Lambda(_)) {
            return self.deopt_instr(ControlKind::With, args);
        }
        let expr = args.remove(0);
        let Expr::Lambda(lambda) = args.remove(0) else {
            unreachable!()
        };
        let params = self.gen_param_names(&lambda.params);
        self.emit_expr(expr);
        let idx_setup_with = self.lasti();
        self.write_instr(Opcode309::SETUP_WITH);
        self.write_arg(0);
        // push __exit__, __enter__() to the stack
        // self.stack_inc_n(2);
        let lambda_line = lambda.body.last().unwrap().ln_begin().unwrap_or(0);
        self.emit_with_block(lambda.body, params);
        let stash = Identifier::private_with_line(self.fresh_gen.fresh_varname(), lambda_line);
        self.emit_store_instr(stash.clone(), Name);
        self.write_instr(POP_BLOCK);
        self.write_arg(0);
        self.write_instr(Opcode308::BEGIN_FINALLY);
        self.write_arg(0);
        self.write_instr(Opcode308::WITH_CLEANUP_START);
        self.write_arg(0);
        self.edit_code(idx_setup_with + 1, (self.lasti() - idx_setup_with - 2) / 2);
        self.write_instr(Opcode308::WITH_CLEANUP_FINISH);
        self.write_arg(0);
        self.write_instr(Opcode308::END_FINALLY);
        self.write_arg(0);
        self.emit_load_name_instr(stash);
    }

    fn emit_with_instr_307(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        if !matches!(args.get(1).unwrap(), Expr::Lambda(_)) {
            return self.deopt_instr(ControlKind::With, args);
        }
        let expr = args.remove(0);
        let Expr::Lambda(lambda) = args.remove(0) else {
            unreachable!()
        };
        let params = self.gen_param_names(&lambda.params);
        self.emit_expr(expr);
        let idx_setup_with = self.lasti();
        self.write_instr(Opcode309::SETUP_WITH);
        self.write_arg(0);
        // push __exit__, __enter__() to the stack
        // self.stack_inc_n(2);
        let lambda_line = lambda.body.last().unwrap().ln_begin().unwrap_or(0);
        self.emit_with_block(lambda.body, params);
        let stash = Identifier::private_with_line(self.fresh_gen.fresh_varname(), lambda_line);
        self.emit_store_instr(stash.clone(), Name);
        self.write_instr(POP_BLOCK);
        self.write_arg(0);
        self.emit_load_const(ValueObj::None);
        self.stack_dec();
        self.write_instr(Opcode308::WITH_CLEANUP_START);
        self.write_arg(0);
        self.edit_code(idx_setup_with + 1, (self.lasti() - idx_setup_with - 2) / 2);
        self.write_instr(Opcode308::WITH_CLEANUP_FINISH);
        self.write_arg(0);
        self.write_instr(Opcode308::END_FINALLY);
        self.write_arg(0);
        self.emit_load_name_instr(stash);
    }

    fn emit_call(&mut self, call: Call) {
        log!(info "entered {} ({call})", fn_name!());
        let init_stack_len = self.stack_len();
        // Python cannot distinguish at compile time between a method call and a attribute call
        if let Some(attr_name) = call.attr_name {
            self.emit_call_method(*call.obj, attr_name, call.args);
        } else {
            match *call.obj {
                Expr::Accessor(Accessor::Ident(ident)) if ident.vis().is_private() => {
                    self.emit_call_local(ident, call.args)
                }
                other if other.ref_t().is_poly_type_meta() => {
                    self.emit_expr(other);
                    self.emit_index_args(call.args);
                }
                other => {
                    self.emit_push_null();
                    self.emit_expr(other);
                    self.emit_args_311(call.args, Name);
                }
            }
        }
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    fn emit_call_local(&mut self, local: Identifier, args: Args) {
        log!(info "entered {}", fn_name!());
        match &local.inspect()[..] {
            "assert" => self.emit_assert_instr(args),
            "Del" => self.emit_del_instr(args),
            "not" => self.emit_not_instr(args),
            "discard" => self.emit_discard_instr(args),
            "for" | "for!" => self.emit_for_instr(args),
            "while!" => self.emit_while_instr(args),
            "if" | "if!" => self.emit_if_instr(args),
            "match" | "match!" => self.emit_match_instr(args, true),
            "with!" => match self.py_version.minor {
                Some(11) => self.emit_with_instr_311(args),
                Some(10) => self.emit_with_instr_310(args),
                Some(9) => self.emit_with_instr_309(args),
                Some(8) => self.emit_with_instr_308(args),
                Some(7) => self.emit_with_instr_307(args),
                _ => todo!("not supported Python version"),
            },
            "sum" if self.py_version.minor <= Some(7) && args.get_kw("start").is_some() => {
                self.load_builtins();
                self.emit_load_name_instr(Identifier::private("#sum"));
                self.emit_args_311(args, Name);
            }
            other if local.ref_t().is_poly_type_meta() && other != "classof" => {
                if self.py_version.minor <= Some(9) {
                    self.load_fake_generic();
                    self.emit_load_name_instr(Identifier::private("#FakeGenericAlias"));
                    let mut args = args;
                    args.insert_pos(0, PosArg::new(Expr::Accessor(Accessor::Ident(local))));
                    self.emit_args_311(args, Name);
                } else {
                    self.emit_load_name_instr(local);
                    self.emit_index_args(args);
                }
            }
            // "pyimport" | "py" are here
            _ => {
                self.emit_push_null();
                self.emit_load_name_instr(local);
                self.emit_args_311(args, Name);
            }
        }
    }

    fn emit_call_method(&mut self, obj: Expr, method_name: Identifier, args: Args) {
        log!(info "entered {}", fn_name!());
        match &method_name.inspect()[..] {
            // mut value class `update!` can be optimized
            "update!" if method_name.ref_t().self_t().is_some_and(|t| t.ref_mut_inner().is_some_and(|t| t.is_mut_value_class())) => {
                if self.py_version.minor >= Some(11) {
                    return self.emit_call_update_311(obj, args);
                } else {
                    return self.emit_call_update_310(obj, args);
                }
            }
            "return" if obj.ref_t().is_callable() => {
                return self.emit_return_instr(args);
            }
            // TODO: create `Generator` type
            "yield" /* if obj.ref_t().is_callable() */ => {
                return self.emit_yield_instr(args);
            }
            _ => {}
        }
        if let Some(func_name) = debind(&method_name) {
            return self.emit_call_fake_method(obj, func_name, method_name, args);
        }
        let is_type = method_name.ref_t().is_poly_type_meta();
        self.emit_expr(obj);
        self.emit_load_method_instr(method_name);
        if is_type {
            self.emit_index_args(args);
        } else {
            self.emit_args_311(args, BoundAttr);
        }
    }

    fn emit_var_args_311(&mut self, pos_len: usize, var_args: &PosArg) {
        if pos_len > 0 {
            self.write_instr(BUILD_LIST);
            self.write_arg(pos_len);
        }
        self.emit_expr(var_args.expr.clone());
        if pos_len > 0 {
            self.write_instr(Opcode310::LIST_EXTEND);
            self.write_arg(1);
            self.write_instr(Opcode310::LIST_TO_TUPLE);
            self.write_arg(0);
        }
    }

    fn emit_var_args_308(&mut self, pos_len: usize, var_args: &PosArg) {
        if pos_len > 0 {
            self.write_instr(BUILD_TUPLE);
            self.write_arg(pos_len);
        }
        self.emit_expr(var_args.expr.clone());
        if pos_len > 0 {
            self.write_instr(Opcode309::BUILD_TUPLE_UNPACK_WITH_CALL);
            self.write_arg(2);
        }
    }

    fn emit_args_311(&mut self, mut args: Args, kind: AccessKind) {
        let argc = args.len();
        let pos_len = args.pos_args.len();
        let mut kws = Vec::with_capacity(args.kw_len());
        while let Some(arg) = args.try_remove_pos(0) {
            self.emit_expr(arg.expr);
        }
        if let Some(var_args) = &args.var_args {
            if self.py_version.minor >= Some(10) {
                self.emit_var_args_311(pos_len, var_args);
            } else {
                self.emit_var_args_308(pos_len, var_args);
            }
        }
        while let Some(arg) = args.try_remove_kw(0) {
            kws.push(ValueObj::Str(arg.keyword.content));
            self.emit_expr(arg.expr);
        }
        let kwsc = if !kws.is_empty() {
            self.emit_call_kw_instr(argc, kws);
            #[allow(clippy::bool_to_int_with_if)]
            if self.py_version.minor >= Some(11) {
                0
            } else {
                1
            }
        } else {
            if args.var_args.is_some() {
                self.write_instr(CALL_FUNCTION_EX);
                if kws.is_empty() {
                    self.write_arg(0);
                } else {
                    self.write_arg(1);
                }
            } else {
                self.emit_call_instr(argc, kind);
            }
            0
        };
        // (1 (subroutine) + argc + kwsc) input objects -> 1 return object
        self.stack_dec_n((1 + argc + kwsc) - 1);
    }

    fn emit_index_args(&mut self, mut args: Args) {
        let argc = args.pos_args.len();
        while let Some(arg) = args.try_remove_pos(0) {
            self.emit_expr(arg.expr);
        }
        if argc > 1 {
            self.write_instr(BUILD_TUPLE);
            self.write_arg(argc);
        }
        self.write_instr(Opcode311::BINARY_SUBSCR);
        self.write_arg(0);
        if self.py_version.minor >= Some(11) {
            self.write_bytes(&[0; 8]);
        }
        // (1 (subroutine) + argc) input objects -> 1 return object
        self.stack_dec_n((1 + argc) - 1);
    }

    /// X.update! x -> x + 1
    /// => X = mutate_operator((x -> x + 1)(X))
    /// TODO: should be `X = X + 1` in the above case
    fn emit_call_update_311(&mut self, obj: Expr, mut args: Args) {
        log!(info "entered {}", fn_name!());
        let Expr::Accessor(acc) = obj else {
            unreachable!()
        };
        let func = args.remove_left_or_key("f").unwrap();
        if !self.mutate_op_loaded {
            self.load_mutate_op();
        }
        self.emit_push_null();
        self.emit_load_name_instr(Identifier::private("#mutate_operator"));
        self.emit_push_null();
        self.emit_expr(func);
        self.emit_acc(acc.clone());
        self.emit_precall_and_call(1);
        // (1 (subroutine) + argc) input objects -> 1 return object
        // self.stack_dec_n((1 + 1) - 1);
        self.stack_dec();
        self.emit_precall_and_call(1);
        self.stack_dec();
        self.store_acc(acc);
        self.emit_load_const(ValueObj::None);
    }

    /// X.update! x -> x + 1
    /// X = mutate_operator((x -> x + 1)(X))
    /// X = X + 1
    fn emit_call_update_310(&mut self, obj: Expr, mut args: Args) {
        log!(info "entered {}", fn_name!());
        let Expr::Accessor(acc) = obj else {
            unreachable!()
        };
        let func = args.remove_left_or_key("f").unwrap();
        if !self.mutate_op_loaded {
            self.load_mutate_op();
        }
        self.emit_load_name_instr(Identifier::private("#mutate_operator"));
        self.emit_expr(func);
        self.emit_acc(acc.clone());
        self.write_instr(Opcode310::CALL_FUNCTION);
        self.write_arg(1);
        // (1 (subroutine) + argc) input objects -> 1 return object
        self.stack_dec_n((1 + 1) - 1);
        self.write_instr(Opcode310::CALL_FUNCTION);
        self.write_arg(1);
        self.stack_dec();
        self.store_acc(acc);
        self.emit_load_const(ValueObj::None);
    }

    // TODO: use exception
    fn emit_return_instr(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        if args.is_empty() {
            self.emit_load_const(ValueObj::None);
        } else {
            self.emit_expr(args.remove(0));
        }
        self.write_instr(RETURN_VALUE);
        self.write_arg(0);
    }

    fn emit_yield_instr(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        if args.is_empty() {
            self.emit_load_const(ValueObj::None);
        } else {
            self.emit_expr(args.remove(0));
        }
        self.write_instr(YIELD_VALUE);
        self.write_arg(0);
    }

    /// 1.abs() => abs(1)
    fn emit_call_fake_method(
        &mut self,
        obj: Expr,
        func_name: Str,
        mut method_name: Identifier,
        mut args: Args,
    ) {
        log!(info "entered {}", fn_name!());
        method_name.raw.vis = VisModifierSpec::Private;
        method_name.vi.py_name = Some(func_name);
        self.emit_push_null();
        self.emit_load_name_instr(method_name);
        args.insert_pos(0, PosArg::new(obj));
        self.emit_args_311(args, Name);
    }

    // assert takes 1 or 2 arguments (0: cond, 1: message)
    fn emit_assert_instr(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        let init_stack_len = self.stack_len();
        self.emit_expr(args.remove(0));
        let pop_jump_point = self.lasti();
        self.write_instr(EXTENDED_ARG);
        self.write_arg(0);
        self.write_instr(Opcode310::POP_JUMP_IF_TRUE);
        self.write_arg(0);
        self.stack_dec();
        if self.py_version.minor >= Some(10) {
            self.write_instr(Opcode310::LOAD_ASSERTION_ERROR);
            self.write_arg(0);
            self.stack_inc();
        } else {
            self.emit_load_global_instr(Identifier::public("AssertionError"));
        }
        if let Some(expr) = args.try_remove(0) {
            self.emit_expr(expr);
            if self.py_version.minor >= Some(11) {
                self.emit_precall_and_call(0);
            } else {
                self.write_instr(Opcode310::CALL_FUNCTION);
                self.write_arg(1);
                self.stack_dec();
            }
        }
        self.write_instr(RAISE_VARARGS);
        self.write_arg(1);
        self.stack_dec();
        let idx = match self.py_version.minor {
            Some(11) => self.lasti() - pop_jump_point - 4,
            Some(10) => self.lasti(),
            Some(_) => self.lasti(),
            _ => todo!(),
        };
        self.fill_jump(pop_jump_point + 1, idx);
        self.emit_load_const(ValueObj::None);
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    // TODO: list comprehension
    fn emit_array(&mut self, array: Array) {
        let init_stack_len = self.stack_len();
        if !self.cfg.no_std {
            self.emit_push_null();
            if array.is_unsized() {
                self.emit_load_name_instr(Identifier::public("UnsizedArray"));
            } else {
                self.emit_load_name_instr(Identifier::public("Array"));
            }
        }
        match array {
            Array::Normal(mut arr) => {
                let len = arr.elems.len();
                while let Some(arg) = arr.elems.try_remove_pos(0) {
                    self.emit_expr(arg.expr);
                }
                self.write_instr(BUILD_LIST);
                self.write_arg(len);
                if len == 0 {
                    self.stack_inc();
                } else {
                    self.stack_dec_n(len - 1);
                }
            }
            Array::WithLength(ArrayWithLength {
                elem,
                len: Some(len),
                ..
            }) => {
                self.emit_expr(*elem);
                self.write_instr(BUILD_LIST);
                self.write_arg(1);
                self.emit_call_instr(1, Name);
                self.stack_dec();
                self.emit_expr(*len);
                self.emit_binop_instr(Token::dummy(TokenKind::Star, "*"), TypePair::ArrayNat);
                return;
            }
            Array::WithLength(ArrayWithLength {
                elem, len: None, ..
            }) => {
                self.emit_expr(*elem);
            }
            other => todo!("{other}"),
        }
        if !self.cfg.no_std {
            self.emit_call_instr(1, Name);
            self.stack_dec();
        }
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    // TODO: tuple comprehension
    // TODO: tuples can be const
    fn emit_tuple(&mut self, tuple: Tuple) {
        let init_stack_len = self.stack_len();
        match tuple {
            Tuple::Normal(mut tup) => {
                let len = tup.elems.len();
                while let Some(arg) = tup.elems.try_remove_pos(0) {
                    self.emit_expr(arg.expr);
                }
                self.write_instr(BUILD_TUPLE);
                self.write_arg(len);
                if len == 0 {
                    self.stack_inc();
                } else {
                    self.stack_dec_n(len - 1);
                }
            }
        }
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    fn emit_set(&mut self, set: crate::hir::Set) {
        let init_stack_len = self.stack_len();
        match set {
            crate::hir::Set::Normal(mut set) => {
                let len = set.elems.len();
                while let Some(arg) = set.elems.try_remove_pos(0) {
                    self.emit_expr(arg.expr);
                }
                self.write_instr(BUILD_SET);
                self.write_arg(len);
                if len == 0 {
                    self.stack_inc();
                } else {
                    self.stack_dec_n(len - 1);
                }
            }
            crate::hir::Set::WithLength(st) => {
                self.emit_expr(*st.elem);
                self.write_instr(BUILD_SET);
                self.write_arg(1);
            }
        }
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    fn emit_dict(&mut self, dict: crate::hir::Dict) {
        let init_stack_len = self.stack_len();
        if !self.cfg.no_std {
            self.emit_push_null();
            self.emit_load_name_instr(Identifier::public("Dict"));
        }
        match dict {
            crate::hir::Dict::Normal(dic) => {
                let len = dic.kvs.len();
                for kv in dic.kvs.into_iter() {
                    self.emit_expr(kv.key);
                    self.emit_expr(kv.value);
                }
                self.write_instr(BUILD_MAP);
                self.write_arg(len);
                if len == 0 {
                    self.stack_inc();
                } else {
                    self.stack_dec_n(2 * len - 1);
                }
            }
            other => todo!("{other}"),
        }
        if !self.cfg.no_std {
            self.emit_call_instr(1, Name);
            self.stack_dec();
        }
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    #[allow(clippy::identity_op)]
    fn emit_record(&mut self, rec: Record) {
        log!(info "entered {} ({rec})", fn_name!());
        let init_stack_len = self.stack_len();
        let attrs_len = rec.attrs.len();
        self.emit_push_null();
        // making record type
        let ident = Identifier::private("#NamedTuple");
        self.emit_load_name_instr(ident);
        // record name, let it be anonymous
        self.emit_load_const("Record");
        for field in rec.attrs.iter() {
            self.emit_load_const(ValueObj::Str(field.sig.ident().inspect().clone()));
        }
        self.write_instr(BUILD_LIST);
        self.write_arg(attrs_len);
        if attrs_len == 0 {
            self.stack_inc();
        } else {
            self.stack_dec_n(attrs_len - 1);
        }
        self.emit_call_instr(2, Name);
        // (1 (subroutine) + argc + kwsc) input objects -> 1 return object
        self.stack_dec_n((1 + 2 + 0) - 1);
        let ident = Identifier::private("#rec");
        self.emit_store_instr(ident, Name);
        // making record instance
        let ident = Identifier::private("#rec");
        self.emit_push_null();
        self.emit_load_name_instr(ident);
        for field in rec.attrs.into_iter() {
            self.emit_simple_block(field.body.block);
        }
        self.emit_call_instr(attrs_len, Name);
        // (1 (subroutine) + argc + kwsc) input objects -> 1 return object
        self.stack_dec_n((1 + attrs_len + 0) - 1);
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    /// Emits independent code blocks (e.g., linked other modules)
    fn emit_code(&mut self, code: Block) {
        let mut gen = self.inherit();
        let code = gen.emit_block(code, vec![], None, vec![], 0, vec![], 0);
        self.emit_load_const(code);
    }

    pub(crate) fn get_root(acc: &Accessor) -> Identifier {
        match acc {
            Accessor::Ident(ident) => ident.clone(),
            Accessor::Attr(attr) => {
                if let Expr::Accessor(acc) = attr.obj.as_ref() {
                    Self::get_root(acc)
                } else {
                    todo!("{:?}", attr.obj)
                }
            }
        }
    }

    fn emit_import(&mut self, acc: Accessor) {
        self.emit_load_const(0i32);
        self.emit_load_const(ValueObj::None);
        let full_name = Str::from(
            acc.qual_name()
                .map_or(acc.show(), |s| s.replace(".__init__", "")),
        );
        let name = self
            .local_search(&full_name, Name)
            .unwrap_or_else(|| self.register_name(full_name, Import));
        self.write_instr(IMPORT_NAME);
        self.write_arg(name.idx);
        let root = Self::get_root(&acc);
        self.emit_store_instr(root, Name);
        self.stack_dec();
    }

    fn emit_compound(&mut self, chunks: Block) {
        let is_module_loading_chunks = chunks
            .get(2)
            .map(|chunk| {
                option_enum_unwrap!(chunk, Expr::Call)
                    .map(|call| call.obj.show_acc().as_ref().map(|s| &s[..]) == Some("exec"))
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        if !self.module_type_loaded && is_module_loading_chunks {
            self.load_module_type();
            self.module_type_loaded = true;
        }
        let init_stack_len = self.stack_len();
        for chunk in chunks.into_iter() {
            self.emit_chunk(chunk);
            if self.stack_len() == init_stack_len + 1 {
                self.emit_pop_top();
            }
        }
        self.cancel_if_pop_top();
    }

    /// See `cpython/Object/lnotab_notes.txt` in details
    fn push_lnotab(&mut self, expr: &Expr) {
        let ln_begin = expr.ln_begin().unwrap_or(0);
        if ln_begin > self.cur_block().prev_lineno {
            let mut sd = self.lasti() - self.cur_block().prev_lasti;
            let mut ld = ln_begin - self.cur_block().prev_lineno;
            if ld != 0 {
                if sd != 0 {
                    while sd > 254 {
                        self.mut_cur_block_codeobj().lnotab.push(255);
                        self.mut_cur_block_codeobj().lnotab.push(0);
                        sd -= 254;
                    }
                    while ld > 127 {
                        self.mut_cur_block_codeobj().lnotab.push(0);
                        self.mut_cur_block_codeobj().lnotab.push(127);
                        ld -= 127;
                    }
                    self.mut_cur_block_codeobj().lnotab.push(sd as u8);
                    self.mut_cur_block_codeobj().lnotab.push(ld as u8);
                } else {
                    // empty lines
                    if let Some(&last_ld) = self.cur_block_codeobj().lnotab.last() {
                        if last_ld as u32 + ld > 127 {
                            *self.mut_cur_block_codeobj().lnotab.last_mut().unwrap() = 127;
                            self.mut_cur_block_codeobj().lnotab.push(0);
                            ld -= 127;
                        }
                        while last_ld as u32 + ld > 127 {
                            self.mut_cur_block_codeobj().lnotab.push(127);
                            self.mut_cur_block_codeobj().lnotab.push(0);
                            ld -= 127;
                        }
                        self.mut_cur_block_codeobj().lnotab.push(ld as u8);
                    } else {
                        // a block starts with an empty line
                        self.mut_cur_block_codeobj().lnotab.push(0);
                        self.mut_cur_block_codeobj()
                            .lnotab
                            .push(u8::try_from(ld).unwrap());
                    }
                }
                self.mut_cur_block().prev_lineno += ld;
                self.mut_cur_block().prev_lasti = self.lasti();
            } else {
                CompileError::compiler_bug(
                    0,
                    self.cfg.input.clone(),
                    expr.loc(),
                    fn_name_full!(),
                    line!(),
                )
                .write_to_stderr();
                self.crash("codegen failed: invalid bytecode format");
            }
        }
    }

    fn emit_chunk(&mut self, chunk: Expr) {
        log!(info "entered {} ({chunk})", fn_name!());
        self.push_lnotab(&chunk);
        match chunk {
            Expr::Literal(lit) => self.emit_load_const(lit.value),
            Expr::Accessor(acc) => self.emit_acc(acc),
            Expr::Def(def) => self.emit_def(def),
            Expr::ClassDef(class) => self.emit_class_def(class),
            Expr::PatchDef(patch) => self.emit_patch_def(patch),
            Expr::ReDef(attr) => self.emit_redef(attr),
            Expr::Lambda(lambda) => self.emit_lambda(lambda),
            Expr::UnaryOp(unary) => self.emit_unaryop(unary),
            Expr::BinOp(bin) => self.emit_binop(bin),
            Expr::Call(call) => self.emit_call(call),
            Expr::Array(arr) => self.emit_array(arr),
            Expr::Tuple(tup) => self.emit_tuple(tup),
            Expr::Set(set) => self.emit_set(set),
            Expr::Dict(dict) => self.emit_dict(dict),
            Expr::Record(rec) => self.emit_record(rec),
            Expr::Code(code) => self.emit_code(code),
            Expr::Compound(chunks) => self.emit_compound(chunks),
            Expr::Import(acc) => self.emit_import(acc),
            Expr::Dummy(_) | Expr::TypeAsc(_) => {}
        }
    }

    fn emit_expr(&mut self, expr: Expr) {
        log!(info "entered {} ({expr})", fn_name!());
        self.push_lnotab(&expr);
        let init_stack_len = self.stack_len();
        let mut wrapped = true;
        if !self.cfg.no_std && expr.should_wrap() {
            match expr.ref_t().derefine() {
                Bool => {
                    self.emit_push_null();
                    self.emit_load_name_instr(Identifier::public("Bool"));
                }
                Nat => {
                    self.emit_push_null();
                    self.emit_load_name_instr(Identifier::public("Nat"));
                }
                Int => {
                    self.emit_push_null();
                    self.emit_load_name_instr(Identifier::public("Int"));
                }
                Float => {
                    self.emit_push_null();
                    self.emit_load_name_instr(Identifier::public("Float"));
                }
                Str => {
                    self.emit_push_null();
                    self.emit_load_name_instr(Identifier::public("Str"));
                }
                other => match &other.qual_name()[..] {
                    "Bytes" => {
                        self.emit_push_null();
                        self.emit_load_name_instr(Identifier::public("Bytes"));
                    }
                    "Array" => {
                        self.emit_push_null();
                        self.emit_load_name_instr(Identifier::public("Array"));
                    }
                    "Dict" => {
                        self.emit_push_null();
                        self.emit_load_name_instr(Identifier::public("Dict"));
                    }
                    "Set" => {
                        self.emit_push_null();
                        self.emit_load_name_instr(Identifier::public("Set"));
                    }
                    "Tuple" => {
                        self.emit_push_null();
                        self.emit_load_name_instr(Identifier::public("tuple"));
                    }
                    _ => {
                        wrapped = false;
                    }
                },
            }
        } else {
            wrapped = false;
        }
        match expr {
            Expr::Literal(lit) => self.emit_load_const(lit.value),
            Expr::Accessor(acc) => self.emit_acc(acc),
            Expr::Def(def) => self.emit_def(def),
            Expr::ClassDef(class) => self.emit_class_def(class),
            Expr::PatchDef(patch) => self.emit_patch_def(patch),
            Expr::ReDef(attr) => self.emit_redef(attr),
            Expr::Lambda(lambda) => self.emit_lambda(lambda),
            Expr::UnaryOp(unary) => self.emit_unaryop(unary),
            Expr::BinOp(bin) => self.emit_binop(bin),
            Expr::Call(call) => self.emit_call(call),
            Expr::Array(arr) => self.emit_array(arr),
            Expr::Tuple(tup) => self.emit_tuple(tup),
            Expr::Set(set) => self.emit_set(set),
            Expr::Dict(dict) => self.emit_dict(dict),
            Expr::Record(rec) => self.emit_record(rec),
            Expr::Code(code) => self.emit_code(code),
            Expr::Compound(chunks) => self.emit_compound(chunks),
            Expr::TypeAsc(tasc) => self.emit_expr(*tasc.expr),
            Expr::Import(acc) => self.emit_import(acc),
            Expr::Dummy(_) => {}
        }
        if !self.cfg.no_std && wrapped {
            self.emit_call_instr(1, Name);
            self.stack_dec();
        }
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    /// forブロックなどで使う
    fn emit_control_block(&mut self, block: Block, params: Params) {
        log!(info "entered {}", fn_name!());
        let param_names = self.gen_param_names(&params);
        let line = block.ln_begin().unwrap_or(0);
        for param in param_names {
            self.emit_store_instr(Identifier::public_with_line(DOT, param, line), Name);
        }
        for guard in params.guards {
            if let GuardClause::Bind(bind) = guard {
                self.emit_def(bind);
            }
        }
        if block.is_empty() {
            return;
        }
        let init_stack_len = self.stack_len();
        for chunk in block.into_iter() {
            self.emit_chunk(chunk);
            if self.stack_len() > init_stack_len {
                self.emit_pop_top();
            }
        }
        self.cancel_if_pop_top();
    }

    fn emit_simple_block(&mut self, block: Block) {
        log!(info "entered {}", fn_name!());
        if block.is_empty() {
            return;
        }
        let init_stack_len = self.stack_len();
        for chunk in block.into_iter() {
            self.emit_chunk(chunk);
            if self.stack_len() > init_stack_len {
                self.emit_pop_top();
            }
        }
        self.cancel_if_pop_top();
    }

    fn emit_with_block(&mut self, block: Block, params: Vec<Str>) {
        log!(info "entered {}", fn_name!());
        let line = block.ln_begin().unwrap_or(0);
        for param in params {
            self.emit_store_instr(Identifier::public_with_line(DOT, param, line), Name);
        }
        let init_stack_len = self.stack_len();
        for chunk in block.into_iter() {
            self.emit_chunk(chunk);
            // __exit__, __enter__() are on the stack
            if self.stack_len() > init_stack_len {
                self.emit_pop_top();
            }
        }
        self.cancel_if_pop_top();
    }

    fn emit_class_block(&mut self, class: ClassDef) -> CodeObj {
        log!(info "entered {}", fn_name!());
        let name = class.sig.ident().inspect().clone();
        self.unit_size += 1;
        let firstlineno = match class.methods_list.first().and_then(|def| def.ln_begin()) {
            Some(l) => l,
            None => class.sig.ln_begin().unwrap_or(0),
        };
        self.units.push(PyCodeGenUnit::new(
            self.unit_size,
            self.py_version,
            vec![],
            0,
            Str::rc(self.cfg.input.enclosed_name()),
            &name,
            firstlineno,
            0,
        ));
        let init_stack_len = self.stack_len();
        let mod_name = self.toplevel_block_codeobj().name.clone();
        self.emit_load_const(mod_name);
        self.emit_store_instr(Identifier::public("__module__"), Name);
        self.emit_load_const(name);
        self.emit_store_instr(Identifier::public("__qualname__"), Name);
        let mut methods = ClassDef::take_all_methods(class.methods_list);
        let __init__ = methods
            .remove_def("__init__")
            .or_else(|| methods.remove_def("__init__!"));
        self.emit_init_method(&class.sig, __init__, class.__new__.clone());
        if class.need_to_gen_new {
            self.emit_new_func(&class.sig, class.__new__);
        }
        let __del__ = methods
            .remove_def("__del__")
            .or_else(|| methods.remove_def("__del__!"));
        if let Some(mut __del__) = __del__ {
            __del__.sig.ident_mut().vi.py_name = Some(Str::from("__del__"));
            self.emit_def(__del__);
        }
        if !methods.is_empty() {
            self.emit_simple_block(methods);
        }
        if self.stack_len() == init_stack_len {
            self.emit_load_const(ValueObj::None);
        }
        self.write_instr(RETURN_VALUE);
        self.write_arg(0);
        if self.stack_len() > 1 {
            let block_id = self.cur_block().id;
            let stack_len = self.stack_len();
            CompileError::stack_bug(
                self.input().clone(),
                Location::Unknown,
                stack_len,
                block_id,
                fn_name_full!(),
            )
            .write_to_stderr();
            self.crash("error in emit_class_block: invalid stack size");
        }
        // flagging
        if !self.cur_block_codeobj().varnames.is_empty() {
            self.mut_cur_block_codeobj().flags += CodeObjFlags::NewLocals as u32;
        }
        // end of flagging
        let unit = self.units.pop().unwrap();
        if !self.units.is_empty() {
            let ld = unit.prev_lineno - self.cur_block().prev_lineno;
            if ld != 0 {
                if let Some(l) = self.mut_cur_block_codeobj().lnotab.last_mut() {
                    *l += u8::try_from(ld).unwrap();
                }
                self.mut_cur_block().prev_lineno += ld;
            }
        }
        unit.codeobj
    }

    fn emit_init_method(&mut self, sig: &Signature, __init__: Option<Def>, __new__: Type) {
        log!(info "entered {}", fn_name!());
        let new_first_param = __new__.non_default_params().unwrap().first();
        let line = sig.ln_begin().unwrap_or(0);
        let class_name = sig.ident().inspect();
        let mut ident = Identifier::public_with_line(DOT, Str::ever("__init__"), line);
        ident.vi.t = __new__.clone();
        let self_param = VarName::from_str_and_line(Str::ever("self"), line);
        let vi = VarInfo::nd_parameter(
            __new__.return_t().unwrap().clone(),
            ident.vi.def_loc.clone(),
            "?".into(),
        );
        let raw =
            erg_parser::ast::NonDefaultParamSignature::new(ParamPattern::VarName(self_param), None);
        let self_param = NonDefaultParamSignature::new(raw, vi, None);
        let (param_name, params) = if let Some(new_first_param) = new_first_param {
            let param_name = new_first_param
                .name()
                .cloned()
                .unwrap_or_else(|| self.fresh_gen.fresh_varname());
            let param = VarName::from_str_and_line(param_name.clone(), line);
            let raw =
                erg_parser::ast::NonDefaultParamSignature::new(ParamPattern::VarName(param), None);
            let vi = VarInfo::nd_parameter(
                new_first_param.typ().clone(),
                ident.vi.def_loc.clone(),
                "?".into(),
            );
            let param = NonDefaultParamSignature::new(raw, vi, None);
            let params = Params::new(vec![self_param, param], None, vec![], None, vec![], None);
            (param_name, params)
        } else {
            ("_".into(), Params::single(self_param))
        };
        let bounds = TypeBoundSpecs::empty();
        let subr_sig = SubrSignature::new(
            set! {},
            ident,
            bounds,
            params,
            sig.t_spec_with_op().cloned(),
            vec![],
        );
        let mut attrs = vec![];
        if let Some(__init__) = __init__ {
            attrs.extend(__init__.body.block.clone());
        }
        match new_first_param.map(|pt| pt.typ()) {
            // namedtupleは仕様上::xなどの名前を使えない
            // {x = Int; y = Int}
            //   => self::x = %x.x; self::y = %x.y
            // {.x = Int; .y = Int}
            //   => self.x = %x.x; self.y = %x.y
            // () => pass
            Some(Type::Record(rec)) => {
                for field in rec.keys() {
                    let obj =
                        Expr::Accessor(Accessor::private_with_line(Str::from(&param_name), line));
                    let ident = erg_parser::ast::Identifier::public(field.symbol.clone());
                    let expr = obj.attr_expr(Identifier::bare(ident));
                    let obj = Expr::Accessor(Accessor::private_with_line(Str::ever("self"), line));
                    let dot = if field.vis.is_private() {
                        VisModifierSpec::Private
                    } else {
                        VisModifierSpec::Public(DOT)
                    };
                    let attr = erg_parser::ast::Identifier::new(
                        dot,
                        VarName::from_str(field.symbol.clone()),
                    );
                    let attr = obj.attr(Identifier::bare(attr));
                    let redef = ReDef::new(attr, Block::new(vec![expr]));
                    attrs.push(Expr::ReDef(redef));
                }
            }
            // self::base = %x
            Some(_) => {
                let expr =
                    Expr::Accessor(Accessor::private_with_line(Str::from(&param_name), line));
                let obj = Expr::Accessor(Accessor::private_with_line(Str::ever("self"), line));
                let attr = obj.attr(Identifier::private_with_line(Str::ever("base"), line));
                let redef = ReDef::new(attr, Block::new(vec![expr]));
                attrs.push(Expr::ReDef(redef));
            }
            None => {}
        }
        let none = Token::new_fake(TokenKind::NoneLit, "None", line, 0, 0);
        attrs.push(Expr::Literal(Literal::new(ValueObj::None, none)));
        let block = Block::new(attrs);
        let body = DefBody::new(EQUAL, block, DefId(0));
        self.emit_subr_def(Some(class_name), subr_sig, body);
    }

    /// ```python
    /// class C:
    ///     # __new__ => C
    ///     def new(x): return C(x)
    /// ```
    fn emit_new_func(&mut self, sig: &Signature, __new__: Type) {
        log!(info "entered {}", fn_name!());
        let class_ident = sig.ident();
        let line = sig.ln_begin().unwrap_or(0);
        let mut ident = Identifier::public_with_line(DOT, Str::ever("new"), line);
        let class = Expr::Accessor(Accessor::Ident(class_ident.clone()));
        ident.vi.t = __new__;
        if let Some(new_first_param) = ident.vi.t.non_default_params().unwrap().first() {
            let param_name = new_first_param
                .name()
                .cloned()
                .unwrap_or_else(|| self.fresh_gen.fresh_varname());
            let param = VarName::from_str_and_line(param_name.clone(), line);
            let vi = VarInfo::nd_parameter(
                new_first_param.typ().clone(),
                ident.vi.def_loc.clone(),
                "?".into(),
            );
            let raw =
                erg_parser::ast::NonDefaultParamSignature::new(ParamPattern::VarName(param), None);
            let param = NonDefaultParamSignature::new(raw, vi, None);
            let params = Params::single(param);
            let bounds = TypeBoundSpecs::empty();
            let sig = SubrSignature::new(
                set! {},
                ident,
                bounds,
                params,
                sig.t_spec_with_op().cloned(),
                vec![],
            );
            let arg = PosArg::new(Expr::Accessor(Accessor::private_with_line(
                param_name, line,
            )));
            let call = class.call_expr(Args::single(arg));
            let block = Block::new(vec![call]);
            let body = DefBody::new(EQUAL, block, DefId(0));
            self.emit_subr_def(Some(class_ident.inspect()), sig, body);
        } else {
            let params = Params::empty();
            let bounds = TypeBoundSpecs::empty();
            let sig = SubrSignature::new(
                set! {},
                ident,
                bounds,
                params,
                sig.t_spec_with_op().cloned(),
                vec![],
            );
            let call = class.call_expr(Args::empty());
            let block = Block::new(vec![call]);
            let body = DefBody::new(EQUAL, block, DefId(0));
            self.emit_subr_def(Some(class_ident.inspect()), sig, body);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_block(
        &mut self,
        block: Block,
        guards: Vec<GuardClause>,
        opt_name: Option<Str>,
        params: Vec<Str>,
        kwonlyargcount: u32,
        captured_names: Vec<Identifier>,
        flags: u32,
    ) -> CodeObj {
        log!(info "entered {}", fn_name!());
        self.unit_size += 1;
        let name = if let Some(name) = opt_name {
            name
        } else {
            self.fresh_gen.fresh_varname()
        };
        let firstlineno = block
            .first()
            .and_then(|first| first.ln_begin())
            .unwrap_or(0);
        self.units.push(PyCodeGenUnit::new(
            self.unit_size,
            self.py_version,
            params,
            kwonlyargcount,
            Str::rc(self.cfg.input.enclosed_name()),
            name,
            firstlineno,
            flags,
        ));
        let idx_copy_free_vars = if self.py_version.minor >= Some(11) {
            let idx_copy_free_vars = self.lasti();
            self.write_instr(Opcode311::COPY_FREE_VARS);
            self.write_arg(0);
            self.write_instr(Opcode311::RESUME);
            self.write_arg(0);
            idx_copy_free_vars
        } else {
            0
        };
        let mut cells = vec![];
        for captured in captured_names {
            self.mut_cur_block()
                .captured_vars
                .push(captured.inspect().clone());
            if self.py_version.minor >= Some(11) {
                self.write_instr(Opcode311::MAKE_CELL);
                cells.push((captured, self.lasti()));
                self.write_arg(0);
            }
        }
        let init_stack_len = self.stack_len();
        for guard in guards {
            if let GuardClause::Bind(bind) = guard {
                self.emit_def(bind);
            }
        }
        for chunk in block.into_iter() {
            self.emit_chunk(chunk);
            // NOTE: 各行のトップレベルでは0個または1個のオブジェクトが残っている
            // Pythonの場合使わなかったオブジェクトはそのまま捨てられるが、Ergではdiscardを使う必要がある
            // TODO: discard
            if self.stack_len() > init_stack_len {
                self.emit_pop_top();
            }
        }
        self.cancel_if_pop_top(); // 最後の値は戻り値として取っておく
        if self.stack_len() == init_stack_len {
            self.emit_load_const(ValueObj::None);
        } else if self.stack_len() > init_stack_len + 1 {
            let block_id = self.cur_block().id;
            let stack_len = self.stack_len();
            CompileError::stack_bug(
                self.input().clone(),
                Location::Unknown,
                stack_len,
                block_id,
                fn_name_full!(),
            )
            .write_to_stderr();
            self.crash("error in emit_block: invalid stack size");
        }
        self.write_instr(RETURN_VALUE);
        self.write_arg(0);
        // flagging
        if !self.cur_block_codeobj().varnames.is_empty() {
            self.mut_cur_block_codeobj().flags += CodeObjFlags::NewLocals as u32;
        }
        let freevars_len = self.cur_block_codeobj().freevars.len();
        if freevars_len > 0 {
            self.mut_cur_block_codeobj().flags += CodeObjFlags::Nested as u32;
            if self.py_version.minor >= Some(11) {
                self.edit_code(idx_copy_free_vars + 1, freevars_len);
            }
        } else if self.py_version.minor >= Some(11) {
            // cancel copying
            let code = self.cur_block_codeobj().code.get(idx_copy_free_vars);
            debug_assert_eq!(code, Some(&(Opcode311::COPY_FREE_VARS as u8)));
            self.edit_code(idx_copy_free_vars, CommonOpcode::NOP as usize);
        }
        for (cell, placeholder) in cells {
            let name = escape_ident(cell);
            let Some(idx) = self
                .cur_block_codeobj()
                .varnames
                .iter()
                .position(|v| v == &name)
            else {
                continue;
            };
            self.edit_code(placeholder, idx);
        }
        // end of flagging
        let unit = self.units.pop().unwrap();
        // increase lineno
        if !self.units.is_empty() {
            let ld = unit
                .prev_lineno
                .saturating_sub(self.cur_block().prev_lineno);
            if ld != 0 {
                if let Some(l) = self.mut_cur_block_codeobj().lnotab.last_mut() {
                    *l += u8::try_from(ld).unwrap();
                }
                self.mut_cur_block().prev_lineno += ld;
            }
        }
        unit.codeobj
    }

    fn load_prelude(&mut self) {
        // NOTE: Integers need to be used in IMPORT_NAME
        // but `Int` are called before importing it, so they need to be no_std mode
        let no_std = self.cfg.no_std;
        self.cfg.no_std = true;
        self.load_record_type();
        self.load_prelude_py();
        self.prelude_loaded = true;
        self.record_type_loaded = true;
        self.cfg.no_std = no_std;
    }

    fn load_contains_op(&mut self) {
        let mod_name = Identifier::public("_erg_std_prelude");
        self.emit_global_import_items(
            mod_name,
            vec![(
                Identifier::public("contains_operator"),
                Some(Identifier::private("#contains_operator")),
            )],
        );
        self.contains_op_loaded = true;
    }

    fn load_mutate_op(&mut self) {
        let mod_name = Identifier::public("_erg_std_prelude");
        self.emit_global_import_items(
            mod_name,
            vec![(
                Identifier::public("mutate_operator"),
                Some(Identifier::private("#mutate_operator")),
            )],
        );
        self.mutate_op_loaded = true;
    }

    fn load_control(&mut self) {
        let mod_name = Identifier::public("_erg_control");
        self.emit_import_all_instr(mod_name);
        self.control_loaded = true;
    }

    fn load_convertors(&mut self) {
        let mod_name = Identifier::public("_erg_convertors");
        self.emit_import_all_instr(mod_name);
        self.convertors_loaded = true;
    }

    fn load_operators(&mut self) {
        let mod_name = Identifier::public("operator");
        self.emit_import_all_instr(mod_name);
        self.operators_loaded = true;
    }

    fn load_prelude_py(&mut self) {
        self.emit_global_import_items(
            Identifier::public("sys"),
            vec![(
                Identifier::public("path"),
                Some(Identifier::private("#path")),
            )],
        );
        self.emit_load_name_instr(Identifier::private("#path"));
        self.emit_load_method_instr(Identifier::public("append"));
        self.emit_load_const(erg_core_path().to_str().unwrap());
        self.emit_call_instr(1, BoundAttr);
        self.stack_dec();
        self.emit_pop_top();
        let erg_std_mod = Identifier::public("_erg_std_prelude");
        // escaping
        self.emit_global_import_items(
            erg_std_mod.clone(),
            vec![(
                Identifier::public("contains_operator"),
                Some(Identifier::private("#contains_operator")),
            )],
        );
        self.emit_import_all_instr(erg_std_mod);
    }

    fn load_record_type(&mut self) {
        self.emit_global_import_items(
            Identifier::public("collections"),
            vec![(
                Identifier::public("namedtuple"),
                Some(Identifier::private("#NamedTuple")),
            )],
        );
    }

    fn load_abc(&mut self) {
        self.emit_global_import_items(
            Identifier::public("abc"),
            vec![
                (
                    Identifier::public("ABCMeta"),
                    Some(Identifier::private("#ABCMeta")),
                ),
                (
                    Identifier::public("abstractmethod"),
                    Some(Identifier::private("#abstractmethod")),
                ),
            ],
        );
    }

    fn load_union(&mut self) {
        self.emit_global_import_items(
            Identifier::public("_erg_type"),
            vec![(
                Identifier::public("UnionType"),
                Some(Identifier::private("#UnionType")),
            )],
        );
    }

    fn load_fake_generic(&mut self) {
        self.emit_global_import_items(
            Identifier::public("_erg_type"),
            vec![(
                Identifier::public("FakeGenericAlias"),
                Some(Identifier::private("#FakeGenericAlias")),
            )],
        );
    }

    fn load_module_type(&mut self) {
        self.emit_global_import_items(
            Identifier::public("types"),
            vec![(
                Identifier::public("ModuleType"),
                Some(Identifier::private("#ModuleType")),
            )],
        );
    }

    fn load_builtins(&mut self) {
        self.emit_global_import_items(
            Identifier::public("_erg_builtins"),
            vec![(Identifier::public("sum"), Some(Identifier::private("#sum")))],
        );
    }

    pub fn emit(&mut self, hir: HIR) -> CodeObj {
        log!(info "the code-generating process has started.{RESET}");
        self.unit_size += 1;
        self.units.push(PyCodeGenUnit::new(
            self.unit_size,
            self.py_version,
            vec![],
            0,
            Str::rc(self.cfg.input.enclosed_name()),
            "<module>",
            1,
            0,
        ));
        if self.py_version.minor >= Some(11) {
            self.write_instr(Opcode311::RESUME);
            self.write_arg(0);
        }
        if !self.cfg.no_std && !self.prelude_loaded {
            self.load_prelude();
        }
        for chunk in hir.module.into_iter() {
            self.emit_chunk(chunk);
            // TODO: discard
            if self.stack_len() == 1 {
                self.emit_pop_top();
            }
        }
        self.cancel_if_pop_top(); // 最後の値は戻り値として取っておく
        if self.input().is_repl() {
            if self.stack_len() == 1 {
                self.emit_print_expr();
            }
            self.stack_dec_n(self.stack_len() as usize);
        }
        if self.stack_len() == 0 {
            self.emit_load_const(ValueObj::None);
        } else if self.stack_len() > 1 {
            let block_id = self.cur_block().id;
            let stack_len = self.stack_len();
            CompileError::stack_bug(
                self.input().clone(),
                Location::Unknown,
                stack_len,
                block_id,
                fn_name_full!(),
            )
            .write_to_stderr();
            self.crash("error in emit: invalid stack size");
        }
        self.write_instr(RETURN_VALUE);
        self.write_arg(0);
        // flagging
        if !self.cur_block_codeobj().varnames.is_empty() {
            self.mut_cur_block_codeobj().flags += CodeObjFlags::NewLocals as u32;
        }
        // end of flagging
        let unit = self.units.pop().unwrap();
        if !self.units.is_empty() {
            let ld = unit.prev_lineno - self.cur_block().prev_lineno;
            if ld != 0 {
                if let Some(l) = self.mut_cur_block_codeobj().lnotab.last_mut() {
                    *l += u8::try_from(ld).unwrap();
                }
                self.mut_cur_block().prev_lineno += ld;
            }
        }
        log!(info "the code-generating process has completed.{RESET}");
        unit.codeobj
    }
}
