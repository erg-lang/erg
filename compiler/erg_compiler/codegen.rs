//! generates `CodeObj` (equivalent to PyCodeObject of CPython) from `AST`.
//!
//! ASTからPythonバイトコード(コードオブジェクト)を生成する
use std::fmt;
use std::process;

use crate::ty::codeobj::MakeFunctionFlags;
use crate::ty::codeobj::{CodeObj, CodeObjFlags};
use crate::ty::value::GenTypeObj;
use erg_common::cache::CacheSet;
use erg_common::config::{ErgConfig, Input};
use erg_common::env::erg_std_path;
use erg_common::error::{ErrorDisplay, Location};
use erg_common::opcode::{CommonOpcode, CompareOp};
use erg_common::opcode308::Opcode308;
use erg_common::opcode310::Opcode310;
use erg_common::opcode311::{BinOpCode, Opcode311};
use erg_common::option_enum_unwrap;
use erg_common::python_util::{env_python_version, PythonVersion};
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{
    debug_power_assert, enum_unwrap, fn_name, fn_name_full, impl_stream_for_wrapper, log,
    switch_unreachable,
};
use erg_parser::ast::ConstExpr;
use erg_parser::ast::DefId;
use erg_parser::ast::DefKind;
use CommonOpcode::*;

use erg_parser::ast::PreDeclTypeSpec;
use erg_parser::ast::TypeSpec;
use erg_parser::ast::{NonDefaultParamSignature, ParamPattern, VarName};
use erg_parser::token::DOT;
use erg_parser::token::EQUAL;
use erg_parser::token::{Token, TokenKind};

use crate::compile::{AccessKind, Name, StoreLoadKind};
use crate::context::eval::type_from_token_kind;
use crate::error::CompileError;
use crate::hir::{
    Accessor, Args, Array, AttrDef, BinOp, Block, Call, ClassDef, Def, DefBody, Expr, Identifier,
    Lambda, Literal, Params, PosArg, Record, Signature, SubrSignature, Tuple, UnaryOp,
    VarSignature, HIR,
};
use crate::ty::free::fresh_varname;
use crate::ty::value::ValueObj;
use crate::ty::{HasType, Type, TypeCode, TypePair};
use AccessKind::*;

fn fake_method_to_func(class: &str, name: &str) -> Option<&'static str> {
    match (class, name) {
        (_, "abs") => Some("abs"),
        (_, "iter") => Some("iter"),
        (_, "map") => Some("map"),
        _ => None,
    }
}

fn escape_name(ident: Identifier) -> Str {
    let vis = ident.vis();
    if let Some(py_name) = ident.vi.py_name {
        py_name
    } else {
        let name = ident.name.into_token().content.to_string();
        let name = name.replace('!', "__erg_proc__");
        let name = name.replace('$', "__erg_shared__");
        if vis.is_private() {
            Str::from("::".to_string() + &name)
        } else {
            Str::from(name)
        }
    }
}

#[derive(Debug, Clone)]
pub struct PyCodeGenUnit {
    pub(crate) id: usize,
    pub(crate) py_version: PythonVersion,
    pub(crate) codeobj: CodeObj,
    pub(crate) stack_len: u32, // the maximum stack size
    pub(crate) prev_lineno: usize,
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
    pub fn new<S: Into<Str>, T: Into<Str>>(
        id: usize,
        py_version: PythonVersion,
        params: Vec<Str>,
        filename: S,
        name: T,
        firstlineno: usize,
    ) -> Self {
        Self {
            id,
            py_version,
            codeobj: CodeObj::empty(params, filename, name, firstlineno as u32),
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

impl_stream_for_wrapper!(PyCodeGenStack, PyCodeGenUnit);

#[derive(Debug, Default)]
pub struct PyCodeGenerator {
    cfg: ErgConfig,
    pub(crate) py_version: PythonVersion,
    str_cache: CacheSet<str>,
    prelude_loaded: bool,
    in_op_loaded: bool,
    record_type_loaded: bool,
    module_type_loaded: bool,
    abc_loaded: bool,
    unit_size: usize,
    units: PyCodeGenStack,
}

impl PyCodeGenerator {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            py_version: cfg.target_version.unwrap_or_else(env_python_version),
            cfg,
            str_cache: CacheSet::new(),
            prelude_loaded: false,
            in_op_loaded: false,
            record_type_loaded: false,
            module_type_loaded: false,
            abc_loaded: false,
            unit_size: 0,
            units: PyCodeGenStack::empty(),
        }
    }

    pub fn clear(&mut self) {
        self.units.clear();
    }

    #[inline]
    fn input(&self) -> &Input {
        &self.cfg.input
    }

    fn get_cached(&self, s: &str) -> Str {
        self.str_cache.get(s)
    }

    /// 大抵の場合はモジュールのブロックが返る
    #[inline]
    fn toplevel_block(&self) -> &PyCodeGenUnit {
        self.units.first().unwrap()
    }

    #[inline]
    fn cur_block(&self) -> &PyCodeGenUnit {
        self.units.last().unwrap()
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
    #[allow(dead_code)]
    fn terminate(&mut self) {
        self.emit_push_null();
        self.emit_load_name_instr(Identifier::public("exit"));
        self.emit_load_const(1);
        self.emit_precall_and_call(1);
        self.stack_dec();
    }

    /// swap TOS and TOS1
    fn rot2(&mut self) {
        if self.py_version.minor >= Some(11) {
            self.write_instr(Opcode311::SWAP);
            self.write_arg(2);
        } else {
            self.write_instr(Opcode310::ROT_TWO);
            self.write_arg(0);
        }
    }

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

    #[inline]
    fn jump_delta(&self, jump_to: usize) -> usize {
        if self.py_version.minor >= Some(10) {
            if self.lasti() <= jump_to * 2 {
                3
            } else {
                0
            }
        } else if self.lasti() <= jump_to {
            6
        } else {
            0
        }
    }

    fn calc_edit_jump(&mut self, idx: usize, jump_to: usize) {
        let arg = if self.py_version.minor >= Some(10) {
            jump_to / 2
        } else {
            jump_to
        };
        self.edit_code(idx, arg);
    }

    #[inline]
    fn edit_code(&mut self, idx: usize, arg: usize) {
        match u8::try_from(arg) {
            Ok(u8code) => {
                *self.mut_cur_block_codeobj().code.get_mut(idx).unwrap() = u8code;
            }
            Err(_e) => {
                // TODO: use u16 as long as possible
                // see write_arg's comment
                let delta = self.jump_delta(arg);
                let bytes = u32::try_from(arg + delta).unwrap().to_be_bytes();
                let before_instr = idx.saturating_sub(1);
                *self.mut_cur_block_codeobj().code.get_mut(idx).unwrap() = bytes[3];
                self.extend_arg(before_instr, &bytes);
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
    #[inline]
    fn extend_arg(&mut self, before_instr: usize, bytes: &[u8]) {
        self.mut_cur_block_codeobj()
            .code
            .insert(before_instr, bytes[2]);
        self.mut_cur_block_codeobj()
            .code
            .insert(before_instr, CommonOpcode::EXTENDED_ARG as u8);
        self.mut_cur_block_codeobj()
            .code
            .insert(before_instr, bytes[1]);
        self.mut_cur_block_codeobj()
            .code
            .insert(before_instr, CommonOpcode::EXTENDED_ARG as u8);
        self.mut_cur_block_codeobj()
            .code
            .insert(before_instr, bytes[0]);
        self.mut_cur_block_codeobj()
            .code
            .insert(before_instr, CommonOpcode::EXTENDED_ARG as u8);
        self.mut_cur_block().lasti += 6;
    }

    fn write_instr<C: Into<u8>>(&mut self, code: C) {
        self.mut_cur_block_codeobj().code.push(code.into());
        self.mut_cur_block().lasti += 1;
        // log!(info "wrote: {}", code);
    }

    fn write_arg(&mut self, code: usize) {
        match u8::try_from(code) {
            Ok(u8code) => {
                self.mut_cur_block_codeobj().code.push(u8code);
                self.mut_cur_block().lasti += 1;
                // log!(info "wrote: {}", code);
            }
            Err(_) => {
                let delta = self.jump_delta(code);
                let bytes = u32::try_from(code + delta).unwrap().to_be_bytes();
                let before_instr = self.lasti().saturating_sub(1);
                self.mut_cur_block_codeobj().code.push(bytes[3]);
                self.mut_cur_block().lasti += 1;
                self.extend_arg(before_instr, &bytes);
            }
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
        let value = cons.into();
        let is_nat = value.is_nat();
        let is_bool = value.is_bool();
        if !self.cfg.no_std {
            if is_bool {
                self.emit_push_null();
                self.emit_load_name_instr(Identifier::public("Bool"));
            } else if is_nat {
                self.emit_push_null();
                self.emit_load_name_instr(Identifier::public("Nat"));
            }
        }
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
        if !self.cfg.no_std && is_nat {
            self.emit_call_instr(1, Name);
            self.stack_dec();
        }
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

    fn local_search(&self, name: &str, _acc_kind: AccessKind) -> Option<Name> {
        let current_is_toplevel = self.cur_block() == self.toplevel_block();
        if let Some(idx) = self
            .cur_block_codeobj()
            .names
            .iter()
            .position(|n| &**n == name)
        {
            Some(Name::local(idx))
        } else if let Some(idx) = self
            .cur_block_codeobj()
            .varnames
            .iter()
            .position(|v| &**v == name)
        {
            if current_is_toplevel {
                Some(Name::local(idx))
            } else {
                Some(Name::fast(idx))
            }
        } else {
            self.cur_block_codeobj()
                .freevars
                .iter()
                .position(|f| &**f == name)
                .map(Name::deref)
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

    fn register_name(&mut self, name: Str) -> Name {
        let current_is_toplevel = self.cur_block() == self.toplevel_block();
        match self.rec_search(&name) {
            Some(st @ (StoreLoadKind::Local | StoreLoadKind::Global)) => {
                let st = if current_is_toplevel {
                    StoreLoadKind::Local
                } else {
                    st
                };
                self.mut_cur_block_codeobj().names.push(name);
                Name::new(st, self.cur_block_codeobj().names.len() - 1)
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
                Attr => LOAD_ATTR as u8,
                Method => LOAD_METHOD as u8,
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
                    Attr => STORE_ATTR as u8,
                    // cannot overwrite methods directly
                    Method => STORE_ATTR as u8,
                }
            }
        }
    }

    fn emit_load_name_instr(&mut self, ident: Identifier) {
        log!(info "entered {}({ident})", fn_name!());
        let escaped = escape_name(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped));
        let instr = self.select_load_instr(name.kind, Name);
        /*let null_idx = self.cur_block_codeobj().code.len() - 2;
        if instr == LOAD_GLOBAL
            && self.cur_block_codeobj().code.get(null_idx) == Some(&(Opcode311::PUSH_NULL as u8))
        {
            self.mut_cur_block_codeobj().code.pop();
            self.mut_cur_block_codeobj().code.pop();
            self.mut_cur_block().lasti -= 2;
        }*/
        self.write_instr(instr);
        self.write_arg(name.idx);
        self.stack_inc();
        if instr == LOAD_GLOBAL as u8 && self.py_version.minor >= Some(11) {
            self.write_bytes(&[0; 2]);
            self.write_bytes(&[0; 8]);
        }
    }

    fn emit_load_global_instr(&mut self, ident: Identifier) {
        log!(info "entered {} ({ident})", fn_name!());
        let escaped = escape_name(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped));
        let instr = LOAD_GLOBAL;
        self.write_instr(instr);
        self.write_arg(name.idx);
        self.stack_inc();
    }

    fn emit_import_name_instr(&mut self, ident: Identifier, items_len: usize) {
        log!(info "entered {}({ident})", fn_name!());
        let escaped = escape_name(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped));
        self.write_instr(IMPORT_NAME);
        self.write_arg(name.idx);
        self.stack_inc_n(items_len);
        self.stack_dec(); // (level + from_list) -> module object
    }

    fn emit_import_from_instr(&mut self, ident: Identifier) {
        log!(info "entered {}", fn_name!());
        let escaped = escape_name(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped));
        self.write_instr(IMPORT_FROM);
        self.write_arg(name.idx);
        // self.stack_inc(); (module object) -> attribute
    }

    fn emit_import_all_instr(&mut self, ident: Identifier) {
        log!(info "entered {}", fn_name!());
        self.emit_load_const(0i32); // escaping to call access `Nat` before importing `Nat`
        self.emit_load_const([Str::ever("*")]);
        let escaped = escape_name(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped));
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
        let escaped = escape_name(ident);
        let name = self
            .local_search(&escaped, Attr)
            .unwrap_or_else(|| self.register_attr(escaped));
        let instr = self.select_load_instr(name.kind, Attr);
        self.write_instr(instr);
        self.write_arg(name.idx);
        if self.py_version.minor >= Some(11) {
            self.write_bytes(&[0; 8]);
        }
    }

    fn emit_load_method_instr(&mut self, ident: Identifier) {
        log!(info "entered {} ({ident})", fn_name!());
        if &ident.inspect()[..] == "__new__" {
            log!("{:?}", ident.vi);
        }
        let escaped = escape_name(ident);
        let name = self
            .local_search(&escaped, Method)
            .unwrap_or_else(|| self.register_method(escaped));
        let instr = self.select_load_instr(name.kind, Method);
        self.write_instr(instr);
        self.write_arg(name.idx);
        if self.py_version.minor >= Some(11) {
            self.stack_inc(); // instead of PUSH_NULL
            self.write_bytes(&[0; 20]);
        }
    }

    fn emit_store_instr(&mut self, ident: Identifier, acc_kind: AccessKind) {
        log!(info "entered {} ({ident})", fn_name!());
        let escaped = escape_name(ident);
        let name = self.local_search(&escaped, acc_kind).unwrap_or_else(|| {
            if acc_kind.is_local() {
                self.register_name(escaped)
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
        }
    }

    /// used for importing Erg builtin objects, etc. normally, this is not used
    // Ergの組み込みオブジェクトをimportするときなどに使う、通常は使わない
    fn emit_store_global_instr(&mut self, ident: Identifier) {
        log!(info "entered {} ({ident})", fn_name!());
        let escaped = escape_name(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped));
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
                self.emit_store_instr(attr.ident, Attr);
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
    fn crash(&mut self, description: &str) -> ! {
        if cfg!(feature = "debug") {
            println!("current block: {}", self.cur_block());
            panic!("internal error: {description}");
        } else {
            process::exit(1);
        }
    }

    fn gen_param_names(&self, params: &Params) -> Vec<Str> {
        params
            .non_defaults
            .iter()
            .map(|p| p.inspect().map(|s| &s[..]).unwrap_or("_"))
            .chain(if let Some(var_args) = &params.var_args {
                vec![var_args.inspect().map(|s| &s[..]).unwrap_or("_")]
            } else {
                vec![]
            })
            .chain(
                params
                    .defaults
                    .iter()
                    .map(|p| p.inspect().map(|s| &s[..]).unwrap_or("_")),
            )
            .map(|s| format!("::{s}"))
            .map(|s| self.get_cached(&s))
            .collect()
    }

    fn emit_acc(&mut self, acc: Accessor) {
        log!(info "entered {} ({acc})", fn_name!());
        match acc {
            Accessor::Ident(ident) => {
                if &ident.inspect()[..] == "#ModuleType" && !self.module_type_loaded {
                    self.load_module_type();
                    self.module_type_loaded = true;
                }
                self.emit_load_name_instr(ident);
            }
            Accessor::Attr(mut a) => {
                // Python's namedtuple, a representation of Record, does not allow attribute names such as `::x`.
                // Since Erg does not allow the coexistence of private and public variables with the same name, there is no problem in this trick.
                let is_record = a.obj.ref_t().is_record();
                if is_record {
                    a.ident.dot = Some(DOT);
                }
                self.emit_expr(*a.obj);
                self.emit_load_attr_instr(a.ident);
            }
        }
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
                AccessKind::Method => self.write_instr(Opcode310::CALL_METHOD),
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
            /*self.write_instr(Opcode311::PRECALL);
            self.write_arg(argc);
            self.write_instr(Opcode311::CALL);
            self.write_arg(argc);*/
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
        let code = self.emit_trait_block(def.def_kind(), &def.sig, def.body.block);
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
        let name = sig.ident().inspect().clone();
        let mut trait_call = enum_unwrap!(block.remove(0), Expr::Call);
        let req = if kind == DefKind::Trait {
            enum_unwrap!(
                trait_call.args.remove_left_or_key("Requirement").unwrap(),
                Expr::Record
            )
        } else {
            todo!()
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
            Str::rc(self.cfg.input.enclosed_name()),
            &name,
            firstlineno,
        ));
        let mod_name = self.toplevel_block_codeobj().name.clone();
        self.emit_load_const(mod_name);
        self.emit_store_instr(Identifier::public("__module__"), Name);
        self.emit_load_const(name);
        self.emit_store_instr(Identifier::public("__qualname__"), Name);
        for def in req.attrs.into_iter() {
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
                    *l += ld as u8;
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
                Str::rc(self.cfg.input.enclosed_name()),
                ident.inspect(),
                ident.ln_begin().unwrap(),
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
                        *l += ld as u8;
                    }
                    self.mut_cur_block().prev_lineno += ld;
                }
            }
            unit.codeobj
        };
        self.emit_load_const(code);
        if self.py_version.minor < Some(11) {
            if let Some(class) = class_name {
                self.emit_load_const(Str::from(format!("{class}.{}", ident.name.inspect())));
            } else {
                self.emit_load_const(ident.name.inspect().clone());
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
        let require_or_sup = class_def.require_or_sup.clone();
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
        let subclasses_len = self.emit_require_type(obj, *require_or_sup);
        self.emit_call_instr(2 + subclasses_len, Name);
        self.stack_dec_n((1 + 2 + subclasses_len) - 1);
        self.emit_store_instr(ident, Name);
        self.stack_dec();
    }

    // NOTE: use `TypeVar`, `Generic` in `typing` module
    // fn emit_poly_type_def(&mut self, sig: SubrSignature, body: DefBody) {}

    /// Y = Inherit X => class Y(X): ...
    fn emit_require_type(&mut self, obj: GenTypeObj, require_or_sup: Expr) -> usize {
        log!(info "entered {} ({obj}, {require_or_sup})", fn_name!());
        match obj {
            GenTypeObj::Class(_) => 0,
            GenTypeObj::Subclass(_) => {
                self.emit_expr(require_or_sup);
                1 // TODO: not always 1
            }
            _ => todo!(),
        }
    }

    fn emit_attr_def(&mut self, attr_def: AttrDef) {
        log!(info "entered {} ({attr_def})", fn_name!());
        self.emit_frameless_block(attr_def.block, vec![]);
        self.store_acc(attr_def.attr);
    }

    fn emit_var_def(&mut self, sig: VarSignature, mut body: DefBody) {
        log!(info "entered {} ({sig} = {})", fn_name!(), body.block);
        if body.block.len() == 1 {
            self.emit_expr(body.block.remove(0));
        } else {
            self.emit_frameless_block(body.block, vec![]);
        }
        self.emit_store_instr(sig.ident, Name);
    }

    fn emit_subr_def(&mut self, class_name: Option<&str>, sig: SubrSignature, body: DefBody) {
        log!(info "entered {} ({sig} = {})", fn_name!(), body.block);
        let name = sig.ident.inspect().clone();
        let mut make_function_flag = 0;
        let params = self.gen_param_names(&sig.params);
        if !sig.params.defaults.is_empty() {
            let defaults_len = sig.params.defaults.len();
            sig.params
                .defaults
                .into_iter()
                .for_each(|default| self.emit_expr(default.default_val));
            self.write_instr(BUILD_TUPLE);
            self.write_arg(defaults_len);
            self.stack_dec_n(defaults_len - 1);
            make_function_flag += MakeFunctionFlags::Defaults as usize;
        }
        let code = self.emit_block(body.block, Some(name.clone()), params);
        // code.flags += CodeObjFlags::Optimized as u32;
        self.register_cellvars(&mut make_function_flag);
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
        // stack_dec: <code obj> + <name> -> <function>
        self.stack_dec();
        if make_function_flag & MakeFunctionFlags::Defaults as usize != 0 {
            self.stack_dec();
        }
        self.emit_store_instr(sig.ident, Name);
    }

    fn emit_lambda(&mut self, lambda: Lambda) {
        log!(info "entered {} ({lambda})", fn_name!());
        let mut make_function_flag = 0;
        let params = self.gen_param_names(&lambda.params);
        if !lambda.params.defaults.is_empty() {
            let defaults_len = lambda.params.defaults.len();
            lambda
                .params
                .defaults
                .into_iter()
                .for_each(|default| self.emit_expr(default.default_val));
            self.write_instr(BUILD_TUPLE);
            self.write_arg(defaults_len);
            self.stack_dec_n(defaults_len - 1);
            make_function_flag += MakeFunctionFlags::Defaults as usize;
        }
        let code = self.emit_block(lambda.body, Some("<lambda>".into()), params);
        self.register_cellvars(&mut make_function_flag);
        self.emit_load_const(code);
        if self.py_version.minor < Some(11) {
            self.emit_load_const("<lambda>");
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
    }

    fn register_cellvars(&mut self, flag: &mut usize) {
        if !self.cur_block_codeobj().cellvars.is_empty() {
            let cellvars_len = self.cur_block_codeobj().cellvars.len();
            for i in 0..cellvars_len {
                if self.py_version.minor >= Some(11) {
                    self.write_instr(Opcode311::MAKE_CELL);
                    self.write_arg(i);
                    self.write_instr(Opcode311::LOAD_CLOSURE);
                } else {
                    self.write_instr(Opcode310::LOAD_CLOSURE);
                }
                self.write_arg(i);
            }
            self.write_instr(BUILD_TUPLE);
            self.write_arg(cellvars_len);
            *flag += MakeFunctionFlags::Closure as usize;
        }
    }

    fn emit_unaryop(&mut self, unary: UnaryOp) {
        log!(info "entered {} ({unary})", fn_name!());
        let tycode = TypeCode::from(unary.lhs_t());
        self.emit_expr(*unary.expr);
        let instr = match &unary.op.kind {
            // TODO:
            TokenKind::PrePlus => UNARY_POSITIVE,
            TokenKind::PreMinus => UNARY_NEGATIVE,
            TokenKind::Mutate => NOP, // ERG_MUTATE,
            _ => {
                CompileError::feature_error(
                    self.cfg.input.clone(),
                    unary.op.loc(),
                    &unary.op.inspect().clone(),
                    String::from(unary.op.content),
                )
                .write_to_stderr();
                NOT_IMPLEMENTED
            }
        };
        self.write_instr(instr);
        self.write_arg(tycode as usize);
    }

    fn emit_binop(&mut self, bin: BinOp) {
        log!(info "entered {} ({bin})", fn_name!());
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
            TokenKind::InOp => {
                // if no-std, always `x in y == True`
                if self.cfg.no_std {
                    self.emit_load_const(true);
                    return;
                }
                if !self.in_op_loaded {
                    self.load_in_op();
                }
                self.emit_push_null();
                self.emit_load_name_instr(Identifier::private("#in_operator"));
            }
            _ => {}
        }
        let type_pair = TypePair::new(bin.lhs_t(), bin.rhs_t());
        self.emit_expr(*bin.lhs);
        self.emit_expr(*bin.rhs);
        if self.py_version.minor >= Some(11) {
            self.emit_binop_instr_311(bin.op, type_pair);
        } else {
            self.emit_binop_instr_310(bin.op, type_pair);
        }
    }

    fn emit_binop_instr_310(&mut self, binop: Token, type_pair: TypePair) {
        let instr = match &binop.kind {
            TokenKind::Plus => Opcode310::BINARY_ADD,
            TokenKind::Minus => Opcode310::BINARY_SUBTRACT,
            TokenKind::Star => Opcode310::BINARY_MULTIPLY,
            TokenKind::Slash => Opcode310::BINARY_TRUE_DIVIDE,
            TokenKind::FloorDiv => Opcode310::BINARY_FLOOR_DIVIDE,
            TokenKind::Pow => Opcode310::BINARY_POWER,
            TokenKind::Mod => Opcode310::BINARY_MODULO,
            TokenKind::AndOp => Opcode310::BINARY_AND,
            TokenKind::OrOp => Opcode310::BINARY_OR,
            TokenKind::Less
            | TokenKind::LessEq
            | TokenKind::DblEq
            | TokenKind::NotEq
            | TokenKind::Gre
            | TokenKind::GreEq => Opcode310::COMPARE_OP,
            TokenKind::LeftOpen
            | TokenKind::RightOpen
            | TokenKind::Closed
            | TokenKind::Open
            | TokenKind::InOp => Opcode310::CALL_FUNCTION, // ERG_BINARY_RANGE,
            _ => {
                CompileError::feature_error(
                    self.cfg.input.clone(),
                    binop.loc(),
                    &binop.inspect().clone(),
                    String::from(binop.content),
                )
                .write_to_stderr();
                Opcode310::NOT_IMPLEMENTED
            }
        };
        let arg = match &binop.kind {
            TokenKind::Less => 0,
            TokenKind::LessEq => 1,
            TokenKind::DblEq => 2,
            TokenKind::NotEq => 3,
            TokenKind::Gre => 4,
            TokenKind::GreEq => 5,
            TokenKind::LeftOpen
            | TokenKind::RightOpen
            | TokenKind::Closed
            | TokenKind::Open
            | TokenKind::InOp => 2,
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
            | TokenKind::InOp => {
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
            | TokenKind::OrOp => Opcode311::BINARY_OP,
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
            | TokenKind::InOp => {
                self.write_instr(Opcode311::PRECALL);
                self.write_arg(2);
                self.write_arg(0);
                self.write_arg(0);
                Opcode311::CALL
            }
            _ => {
                CompileError::feature_error(
                    self.cfg.input.clone(),
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
            TokenKind::AndOp => BinOpCode::And as usize,
            TokenKind::OrOp => BinOpCode::Or as usize,
            TokenKind::Less => 0,
            TokenKind::LessEq => 1,
            TokenKind::DblEq => 2,
            TokenKind::NotEq => 3,
            TokenKind::Gre => 4,
            TokenKind::GreEq => 5,
            TokenKind::LeftOpen
            | TokenKind::RightOpen
            | TokenKind::Closed
            | TokenKind::Open
            | TokenKind::InOp => 2,
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
            | TokenKind::InOp => {
                self.stack_dec();
                if self.py_version.minor >= Some(11) {
                    self.stack_dec();
                }
            }
            _ => {}
        }
    }

    fn emit_del_instr(&mut self, mut args: Args) {
        let ident = enum_unwrap!(args.remove_left_or_key("obj").unwrap(), Expr::Accessor:(Accessor::Ident:(_)));
        log!(info "entered {} ({ident})", fn_name!());
        let escaped = escape_name(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped));
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

    fn emit_if_instr(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        let init_stack_len = self.stack_len();
        let cond = args.remove(0);
        self.emit_expr(cond);
        let idx_pop_jump_if_false = self.lasti();
        self.write_instr(Opcode310::POP_JUMP_IF_FALSE);
        // cannot detect where to jump to at this moment, so put as 0
        self.write_arg(0);
        match args.remove(0) {
            // then block
            Expr::Lambda(lambda) => {
                // let params = self.gen_param_names(&lambda.params);
                self.emit_frameless_block(lambda.body, vec![]);
            }
            other => {
                self.emit_expr(other);
            }
        }
        if args.get(0).is_some() {
            self.write_instr(JUMP_FORWARD); // jump to end
            self.write_arg(0);
            // else block
            let idx_else_begin = self.lasti();
            self.calc_edit_jump(idx_pop_jump_if_false + 1, idx_else_begin);
            match args.remove(0) {
                Expr::Lambda(lambda) => {
                    // let params = self.gen_param_names(&lambda.params);
                    self.emit_frameless_block(lambda.body, vec![]);
                }
                other => {
                    self.emit_expr(other);
                }
            }
            let idx_jump_forward = idx_else_begin - 2;
            let idx_end = self.lasti();
            self.calc_edit_jump(idx_jump_forward + 1, idx_end - idx_jump_forward - 2);
            // FIXME: this is a hack to make sure the stack is balanced
            while self.stack_len() != init_stack_len + 1 {
                self.stack_dec();
            }
        } else {
            // no else block
            let idx_end = if self.py_version.minor >= Some(11) {
                self.lasti() - idx_pop_jump_if_false - 1
            } else {
                self.lasti()
            };
            self.calc_edit_jump(idx_pop_jump_if_false + 1, idx_end);
            self.emit_load_const(ValueObj::None);
            while self.stack_len() != init_stack_len + 1 {
                self.stack_dec();
            }
        }
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    fn emit_for_instr(&mut self, mut args: Args) {
        log!(info "entered {} ({})", fn_name!(), args);
        let _init_stack_len = self.stack_len();
        let iterable = args.remove(0);
        self.emit_expr(iterable);
        self.write_instr(GET_ITER);
        self.write_arg(0);
        let idx_for_iter = self.lasti();
        self.write_instr(FOR_ITER);
        self.stack_inc();
        // FOR_ITER pushes a value onto the stack, but we can't know how many
        // but after executing this instruction, stack_len should be 1
        // cannot detect where to jump to at this moment, so put as 0
        self.write_arg(0);
        let lambda = enum_unwrap!(args.remove(0), Expr::Lambda);
        // If there is nothing on the stack at the start, init_stack_len == 2 (an iterator and the first iterator value)
        let init_stack_len = self.stack_len();
        let params = self.gen_param_names(&lambda.params);
        // store the iterator value, stack_len == 1 or 2 in the end
        self.emit_frameless_block(lambda.body, params);
        if self.stack_len() > init_stack_len - 1 {
            self.emit_pop_top();
        }
        debug_assert_eq!(self.stack_len(), init_stack_len - 1); // the iterator is remained
        match self.py_version.minor {
            Some(11) => {
                self.write_instr(Opcode311::JUMP_BACKWARD);
                self.write_arg((self.lasti() - idx_for_iter + 2) / 2);
            }
            Some(10) => {
                self.write_instr(Opcode310::JUMP_ABSOLUTE);
                self.write_arg(idx_for_iter / 2);
            }
            Some(8) => {
                self.write_instr(Opcode308::JUMP_ABSOLUTE);
                self.write_arg(idx_for_iter);
            }
            _ => todo!(),
        }
        let idx_end = self.lasti();
        self.calc_edit_jump(idx_for_iter + 1, idx_end - idx_for_iter - 2);
        self.stack_dec();
        self.emit_load_const(ValueObj::None);
        debug_assert_eq!(self.stack_len(), _init_stack_len + 1);
    }

    fn emit_while_instr(&mut self, mut args: Args) {
        log!(info "entered {} ({})", fn_name!(), args);
        let _init_stack_len = self.stack_len();
        let cond = args.remove(0);
        self.emit_expr(cond.clone());
        let idx_while = self.lasti();
        self.write_instr(Opcode310::POP_JUMP_IF_FALSE);
        self.write_arg(0);
        self.stack_dec();
        let lambda = enum_unwrap!(args.remove(0), Expr::Lambda);
        let init_stack_len = self.stack_len();
        let params = self.gen_param_names(&lambda.params);
        self.emit_frameless_block(lambda.body, params);
        if self.stack_len() > init_stack_len {
            self.emit_pop_top();
        }
        self.emit_expr(cond);
        let arg = if self.py_version.minor >= Some(11) {
            let arg = self.lasti() - (idx_while + 2);
            self.write_instr(Opcode311::POP_JUMP_BACKWARD_IF_TRUE);
            arg / 2 + 1
        } else {
            self.write_instr(Opcode310::POP_JUMP_IF_TRUE);
            if self.py_version.minor >= Some(10) {
                (idx_while + 2) / 2
            } else {
                idx_while + 2
            }
        };
        self.write_arg(arg);
        self.stack_dec();
        let idx_end = if self.py_version.minor >= Some(11) {
            self.lasti() - idx_while - 1
        } else {
            self.lasti()
        };
        self.calc_edit_jump(idx_while + 1, idx_end);
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
            let mut lambda = enum_unwrap!(expr, Expr::Lambda);
            debug_power_assert!(lambda.params.len(), ==, 1);
            if !lambda.params.defaults.is_empty() {
                todo!("default values in match expression are not supported yet")
            }
            let param = lambda.params.non_defaults.remove(0);
            let pop_jump_points = self.emit_match_pattern(param, args.is_empty());
            self.emit_frameless_block(lambda.body, Vec::new());
            // If we move on to the next arm, the stack size will increase
            // so `self.stack_dec();` for now (+1 at the end).
            self.stack_dec();
            for pop_jump_point in pop_jump_points.into_iter() {
                let idx = if self.py_version.minor >= Some(11) {
                    self.lasti() - pop_jump_point // - 2
                } else {
                    self.lasti() + 2
                };
                self.calc_edit_jump(pop_jump_point + 1, idx); // jump to POP_TOP
                jump_forward_points.push(self.lasti());
                self.write_instr(JUMP_FORWARD); // jump to the end
                self.write_arg(0);
            }
        }
        let lasti = self.lasti();
        for jump_point in jump_forward_points.into_iter() {
            self.calc_edit_jump(jump_point + 1, lasti - jump_point - 1);
        }
        self.stack_inc();
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    fn emit_match_pattern(
        &mut self,
        param: NonDefaultParamSignature,
        is_last_arm: bool,
    ) -> Vec<usize> {
        log!(info "entered {}", fn_name!());
        let mut pop_jump_points = vec![];
        if let Some(t_spec) = param.t_spec.map(|spec| spec.t_spec) {
            // If it's the last arm, there's no need to inspect it
            if !is_last_arm {
                self.emit_match_guard(t_spec, &mut pop_jump_points);
            }
        }
        match param.pat {
            ParamPattern::VarName(name) => {
                let ident = Identifier::bare(None, name);
                self.emit_store_instr(ident, AccessKind::Name);
            }
            ParamPattern::Discard(_) => {
                self.emit_pop_top();
            }
            _other => unreachable!(),
        }
        pop_jump_points
    }

    fn emit_match_guard(&mut self, t_spec: TypeSpec, pop_jump_points: &mut Vec<usize>) {
        #[allow(clippy::single_match)]
        match t_spec {
            TypeSpec::Enum(enum_t) => {
                let elems = enum_t
                    .deconstruct()
                    .0
                    .into_iter()
                    .map(|elem| {
                        let ConstExpr::Lit(lit) = elem.expr else { todo!() };
                        let t = type_from_token_kind(lit.token.kind);
                        ValueObj::from_str(t, lit.token.content).unwrap()
                    })
                    .collect::<Vec<_>>();
                self.emit_load_const(elems);
                self.write_instr(CONTAINS_OP);
                self.write_arg(0);
                self.stack_dec();
                pop_jump_points.push(self.lasti());
                // in 3.11, POP_JUMP_IF_FALSE is replaced with POP_JUMP_FORWARD_IF_FALSE
                // but the numbers are the same, only the way the jumping points are calculated is different.
                self.write_instr(Opcode310::POP_JUMP_IF_FALSE); // jump to the next case
                self.write_arg(0);
                // self.stack_dec();
            }
            TypeSpec::PreDeclTy(PreDeclTypeSpec::Simple(simple)) if simple.args.is_empty() => {
                // arg null
                // ↓ SWAP 1
                // null arg
                // ↓ LOAD_NAME(in_operator)
                // null arg in_operator
                // ↓ SWAP 1
                // null in_operator arg
                // ↓ LOAD_NAME(typ)
                // null in_operator arg typ
                self.emit_push_null();
                self.rot2();
                if !self.in_op_loaded {
                    self.load_in_op();
                }
                self.emit_load_name_instr(Identifier::private("#in_operator"));
                self.rot2();
                // TODO: DOT/not
                let mut typ = Identifier::bare(Some(DOT), simple.ident.name);
                // TODO:
                typ.vi.py_name = match &typ.name.inspect()[..] {
                    "Int" => Some("int".into()),
                    "Float" => Some("float".into()),
                    _ => None,
                };
                self.emit_load_name_instr(typ);
                self.emit_precall_and_call(2);
                self.stack_dec();
                pop_jump_points.push(self.lasti());
                // in 3.11, POP_JUMP_IF_FALSE is replaced with POP_JUMP_FORWARD_IF_FALSE
                // but the numbers are the same, only the way the jumping points are calculated is different.
                self.write_instr(Opcode310::POP_JUMP_IF_FALSE); // jump to the next case
                self.write_arg(0);
                self.stack_dec();
            }
            /*TypeSpec::Interval { op, lhs, rhs } => {
                let binop = BinOp::new(op, lhs.downcast(), rhs.downcast(), VarInfo::default());
                self.emit_binop(binop);
            }*/
            // TODO:
            other => {
                log!(err "{other}")
            }
        }
    }

    fn emit_with_instr_311(&mut self, args: Args) {
        log!(info "entered {}", fn_name!());
        let mut args = args;
        let expr = args.remove(0);
        let lambda = enum_unwrap!(args.remove(0), Expr::Lambda);
        let params = self.gen_param_names(&lambda.params);
        self.emit_expr(expr);
        self.write_instr(Opcode311::BEFORE_WITH);
        self.write_arg(0);
        // push __exit__, __enter__() to the stack
        self.stack_inc_n(2);
        let lambda_line = lambda.body.last().unwrap().ln_begin().unwrap_or(0);
        self.emit_with_block(lambda.body, params);
        let stash = Identifier::private_with_line(Str::from(fresh_varname()), lambda_line);
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
        self.write_instr(Opcode308::WITH_EXCEPT_START);
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

    fn emit_with_instr_310(&mut self, args: Args) {
        log!(info "entered {}", fn_name!());
        let mut args = args;
        let expr = args.remove(0);
        let lambda = enum_unwrap!(args.remove(0), Expr::Lambda);
        let params = self.gen_param_names(&lambda.params);
        self.emit_expr(expr);
        let idx_setup_with = self.lasti();
        self.write_instr(Opcode310::SETUP_WITH);
        self.write_arg(0);
        // push __exit__, __enter__() to the stack
        self.stack_inc_n(2);
        let lambda_line = lambda.body.last().unwrap().ln_begin().unwrap_or(0);
        self.emit_with_block(lambda.body, params);
        let stash = Identifier::private_with_line(Str::from(fresh_varname()), lambda_line);
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
        self.write_instr(JUMP_FORWARD);
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

    fn emit_with_instr_308(&mut self, args: Args) {
        log!(info "entered {}", fn_name!());
        let mut args = args;
        let expr = args.remove(0);
        let lambda = enum_unwrap!(args.remove(0), Expr::Lambda);
        let params = self.gen_param_names(&lambda.params);
        self.emit_expr(expr);
        let idx_setup_with = self.lasti();
        self.write_instr(Opcode308::SETUP_WITH);
        self.write_arg(0);
        // push __exit__, __enter__() to the stack
        // self.stack_inc_n(2);
        let lambda_line = lambda.body.last().unwrap().ln_begin().unwrap_or(0);
        self.emit_with_block(lambda.body, params);
        let stash = Identifier::private_with_line(Str::from(fresh_varname()), lambda_line);
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
                other => {
                    let is_py_api = other.is_py_api();
                    self.emit_push_null();
                    self.emit_expr(other);
                    self.emit_args_311(call.args, Name, is_py_api);
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
                Some(8) => self.emit_with_instr_308(args),
                _ => todo!(),
            },
            // "pyimport" | "py" are here
            _ => {
                let is_py_api = local.is_py_api();
                self.emit_push_null();
                self.emit_load_name_instr(local);
                self.emit_args_311(args, Name, is_py_api);
            }
        }
    }

    fn emit_call_method(&mut self, obj: Expr, method_name: Identifier, args: Args) {
        log!(info "entered {}", fn_name!());
        let class = obj.ref_t().qual_name(); // これは必ずmethodのあるクラスになっている
        if &method_name.inspect()[..] == "update!" {
            if self.py_version.minor >= Some(11) {
                return self.emit_call_update_311(obj, args);
            } else {
                return self.emit_call_update_310(obj, args);
            }
        } else if let Some(func_name) = fake_method_to_func(&class, method_name.inspect()) {
            return self.emit_call_fake_method(obj, func_name, method_name, args);
        }
        let is_py_api = obj.is_py_api();
        self.emit_expr(obj);
        self.emit_load_method_instr(method_name);
        self.emit_args_311(args, Method, is_py_api);
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

    fn emit_var_args_38(&mut self, pos_len: usize, var_args: &PosArg) {
        if pos_len > 0 {
            self.write_instr(BUILD_TUPLE);
            self.write_arg(pos_len);
        }
        self.emit_expr(var_args.expr.clone());
        if pos_len > 0 {
            self.write_instr(Opcode308::BUILD_TUPLE_UNPACK_WITH_CALL);
            self.write_arg(2);
        }
    }

    fn emit_args_311(&mut self, mut args: Args, kind: AccessKind, is_py_api: bool) {
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
                self.emit_var_args_38(pos_len, var_args);
            }
        }
        while let Some(arg) = args.try_remove_kw(0) {
            let kw = if is_py_api {
                arg.keyword.content
            } else {
                Str::from(format!("::{}", arg.keyword.content))
            };
            kws.push(ValueObj::Str(kw));
            self.emit_expr(arg.expr);
        }
        let kwsc = if !kws.is_empty() {
            self.emit_call_kw_instr(argc, kws);
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

    /// X.update! x -> x + 1
    /// X = (x -> x + 1)(X)
    /// X = X + 1
    fn emit_call_update_311(&mut self, obj: Expr, mut args: Args) {
        log!(info "entered {}", fn_name!());
        let acc = enum_unwrap!(obj, Expr::Accessor);
        let func = args.remove_left_or_key("f").unwrap();
        self.emit_push_null();
        self.emit_expr(func);
        self.emit_acc(acc.clone());
        self.emit_precall_and_call(1);
        // (1 (subroutine) + argc) input objects -> 1 return object
        // self.stack_dec_n((1 + 1) - 1);
        self.stack_dec();
        self.store_acc(acc);
        self.emit_load_const(ValueObj::None);
    }

    /// X.update! x -> x + 1
    /// X = (x -> x + 1)(X)
    /// X = X + 1
    fn emit_call_update_310(&mut self, obj: Expr, mut args: Args) {
        log!(info "entered {}", fn_name!());
        let acc = enum_unwrap!(obj, Expr::Accessor);
        let func = args.remove_left_or_key("f").unwrap();
        self.emit_expr(func);
        self.emit_acc(acc.clone());
        self.write_instr(Opcode310::CALL_FUNCTION);
        self.write_arg(1);
        // (1 (subroutine) + argc) input objects -> 1 return object
        self.stack_dec_n((1 + 1) - 1);
        self.store_acc(acc);
        self.emit_load_const(ValueObj::None);
    }

    /// 1.abs() => abs(1)
    fn emit_call_fake_method(
        &mut self,
        obj: Expr,
        func_name: &'static str,
        mut method_name: Identifier,
        mut args: Args,
    ) {
        log!(info "entered {}", fn_name!());
        method_name.dot = None;
        method_name.vi.py_name = Some(Str::ever(func_name));
        self.emit_push_null();
        self.emit_load_name_instr(method_name);
        args.insert_pos(0, PosArg::new(obj));
        self.emit_args_311(args, Name, true);
    }

    // assert takes 1 or 2 arguments (0: cond, 1: message)
    fn emit_assert_instr(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        let init_stack_len = self.stack_len();
        self.emit_expr(args.remove(0));
        let pop_jump_point = self.lasti();
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
            }
        }
        self.write_instr(RAISE_VARARGS);
        self.write_arg(1);
        self.stack_dec();
        let idx = match self.py_version.minor {
            Some(11) => (self.lasti() - pop_jump_point - 2) / 2,
            Some(10) => self.lasti() / 2,
            Some(_) => self.lasti(),
            _ => todo!(),
        };
        self.edit_code(pop_jump_point + 1, idx);
        self.emit_load_const(ValueObj::None);
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    // TODO: list comprehension
    fn emit_array(&mut self, array: Array) {
        let init_stack_len = self.stack_len();
        if !self.cfg.no_std {
            self.emit_push_null();
            self.emit_load_name_instr(Identifier::public("Array"));
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
            Array::WithLength(arr) => {
                self.emit_expr(*arr.elem);
                self.write_instr(BUILD_LIST);
                self.write_arg(1);
                self.emit_expr(*arr.len);
                if self.py_version.minor >= Some(11) {
                    self.write_instr(Opcode311::BINARY_OP);
                    self.write_arg(BinOpCode::Multiply as usize);
                } else {
                    self.write_instr(Opcode310::BINARY_MULTIPLY);
                    self.write_arg(0);
                }
                self.stack_dec();
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
            self.emit_frameless_block(field.body.block, vec![]);
        }
        self.emit_call_instr(attrs_len, Name);
        // (1 (subroutine) + argc + kwsc) input objects -> 1 return object
        self.stack_dec_n((1 + attrs_len + 0) - 1);
        debug_assert_eq!(self.stack_len(), init_stack_len + 1);
    }

    fn get_root(acc: &Accessor) -> Identifier {
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
        let full_name = Str::from(acc.show());
        let name = self
            .local_search(&full_name, Name)
            .unwrap_or_else(|| self.register_name(full_name));
        self.write_instr(IMPORT_NAME);
        self.write_arg(name.idx);
        let root = Self::get_root(&acc);
        self.emit_store_instr(root, Name);
        self.stack_dec();
    }

    fn emit_expr(&mut self, expr: Expr) {
        log!(info "entered {} ({expr})", fn_name!());
        if expr.ln_begin().unwrap_or_else(|| panic!("{expr}")) > self.cur_block().prev_lineno {
            let sd = self.lasti() - self.cur_block().prev_lasti;
            let ld = expr.ln_begin().unwrap() - self.cur_block().prev_lineno;
            if ld != 0 {
                if sd != 0 {
                    self.mut_cur_block_codeobj().lnotab.push(sd as u8);
                    self.mut_cur_block_codeobj().lnotab.push(ld as u8);
                } else {
                    // empty lines
                    if let Some(last_ld) = self.mut_cur_block_codeobj().lnotab.last_mut() {
                        *last_ld += ld as u8;
                    } else {
                        // a block starts with an empty line
                        self.mut_cur_block_codeobj().lnotab.push(0);
                        self.mut_cur_block_codeobj().lnotab.push(ld as u8);
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
        match expr {
            Expr::Lit(lit) => self.emit_load_const(lit.value),
            Expr::Accessor(acc) => self.emit_acc(acc),
            Expr::Def(def) => self.emit_def(def),
            Expr::ClassDef(class) => self.emit_class_def(class),
            Expr::AttrDef(attr) => self.emit_attr_def(attr),
            Expr::Lambda(lambda) => self.emit_lambda(lambda),
            Expr::UnaryOp(unary) => self.emit_unaryop(unary),
            Expr::BinOp(bin) => self.emit_binop(bin),
            Expr::Call(call) => self.emit_call(call),
            Expr::Array(arr) => self.emit_array(arr),
            // TODO: tuple comprehension
            // TODO: tuples can be const
            Expr::Tuple(tup) => match tup {
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
            },
            Expr::Set(set) => match set {
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
            },
            Expr::Dict(dict) => match dict {
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
            },
            Expr::Record(rec) => self.emit_record(rec),
            Expr::Code(code) => {
                let code = self.emit_block(code, None, vec![]);
                self.emit_load_const(code);
            }
            Expr::Compound(chunks) => {
                let is_module_loading_chunks = chunks
                    .get(2)
                    .map(|chunk| {
                        option_enum_unwrap!(chunk, Expr::Call)
                            .map(|call| {
                                call.obj.show_acc().as_ref().map(|s| &s[..]) == Some("exec")
                            })
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);
                if !self.module_type_loaded && is_module_loading_chunks {
                    self.load_module_type();
                    self.module_type_loaded = true;
                }
                let init_stack_len = self.stack_len();
                for expr in chunks.into_iter() {
                    self.emit_expr(expr);
                    if self.stack_len() == init_stack_len + 1 {
                        self.emit_pop_top();
                    }
                }
                self.cancel_if_pop_top();
                /*if is_module_loading_chunks {
                    self.stack_dec_n(0);
                }*/
            }
            Expr::TypeAsc(tasc) => {
                self.emit_expr(*tasc.expr);
            }
            Expr::Import(acc) => self.emit_import(acc),
        }
    }

    /// forブロックなどで使う
    fn emit_frameless_block(&mut self, block: Block, params: Vec<Str>) {
        log!(info "entered {}", fn_name!());
        let line = block.ln_begin().unwrap_or(0);
        for param in params {
            self.emit_store_instr(Identifier::public_with_line(DOT, param, line), Name);
        }
        let init_stack_len = self.stack_len();
        for expr in block.into_iter() {
            self.emit_expr(expr);
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
        for expr in block.into_iter() {
            self.emit_expr(expr);
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
        let firstlineno = match class.methods.get(0).and_then(|def| def.ln_begin()) {
            Some(l) => l,
            None => class.sig.ln_begin().unwrap(),
        };
        self.units.push(PyCodeGenUnit::new(
            self.unit_size,
            self.py_version,
            vec![],
            Str::rc(self.cfg.input.enclosed_name()),
            &name,
            firstlineno,
        ));
        let init_stack_len = self.stack_len();
        let mod_name = self.toplevel_block_codeobj().name.clone();
        self.emit_load_const(mod_name);
        self.emit_store_instr(Identifier::public("__module__"), Name);
        self.emit_load_const(name);
        self.emit_store_instr(Identifier::public("__qualname__"), Name);
        self.emit_init_method(&class.sig, class.__new__.clone());
        if class.need_to_gen_new {
            self.emit_new_func(&class.sig, class.__new__);
        }
        if !class.methods.is_empty() {
            self.emit_frameless_block(class.methods, vec![]);
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
                    *l += ld as u8;
                }
                self.mut_cur_block().prev_lineno += ld;
            }
        }
        unit.codeobj
    }

    fn emit_init_method(&mut self, sig: &Signature, __new__: Type) {
        log!(info "entered {}", fn_name!());
        let line = sig.ln_begin().unwrap();
        let class_name = sig.ident().inspect();
        let mut ident = Identifier::public_with_line(DOT, Str::ever("__init__"), line);
        ident.vi.t = __new__.clone();
        let param_name = fresh_varname();
        let param = VarName::from_str_and_line(Str::from(param_name.clone()), line);
        let param = NonDefaultParamSignature::new(ParamPattern::VarName(param), None);
        let self_param = VarName::from_str_and_line(Str::ever("self"), line);
        let self_param = NonDefaultParamSignature::new(ParamPattern::VarName(self_param), None);
        let params = Params::new(vec![self_param, param], None, vec![], None);
        let subr_sig = SubrSignature::new(ident, params);
        let mut attrs = vec![];
        match __new__.non_default_params().unwrap()[0].typ() {
            // namedtupleは仕様上::xなどの名前を使えない
            // {x = Int; y = Int}
            // => self::x = %x.x; self::y = %x.y
            // {.x = Int; .y = Int}
            // => self.x = %x.x; self.y = %x.y
            Type::Record(rec) => {
                for field in rec.keys() {
                    let obj =
                        Expr::Accessor(Accessor::private_with_line(Str::from(&param_name), line));
                    let expr = obj.attr_expr(Identifier::bare(
                        Some(DOT),
                        VarName::from_str(field.symbol.clone()),
                    ));
                    let obj = Expr::Accessor(Accessor::private_with_line(Str::ever("self"), line));
                    let dot = if field.vis.is_private() {
                        None
                    } else {
                        Some(DOT)
                    };
                    let attr = obj.attr(Identifier::bare(
                        dot,
                        VarName::from_str(field.symbol.clone()),
                    ));
                    let attr_def = AttrDef::new(attr, Block::new(vec![expr]));
                    attrs.push(Expr::AttrDef(attr_def));
                }
                let none = Token::new(TokenKind::NoneLit, "None", line, 0);
                attrs.push(Expr::Lit(Literal::try_from(none).unwrap()));
            }
            other => todo!("{other}"),
        }
        let block = Block::new(attrs);
        let body = DefBody::new(EQUAL, block, DefId(0));
        self.emit_subr_def(Some(class_name), subr_sig, body);
    }

    /// ```python
    /// class C:
    ///     # __new__ => __call__
    ///     def new(x): return C.__call__(x)
    /// ```
    fn emit_new_func(&mut self, sig: &Signature, __new__: Type) {
        log!(info "entered {}", fn_name!());
        let class_ident = sig.ident();
        let line = sig.ln_begin().unwrap();
        let mut ident = Identifier::public_with_line(DOT, Str::ever("new"), line);
        ident.vi.t = __new__;
        let param_name = fresh_varname();
        let param = VarName::from_str_and_line(Str::from(param_name.clone()), line);
        let param = NonDefaultParamSignature::new(ParamPattern::VarName(param), None);
        let sig = SubrSignature::new(ident, Params::new(vec![param], None, vec![], None));
        let arg = PosArg::new(Expr::Accessor(Accessor::private_with_line(
            Str::from(param_name),
            line,
        )));
        let class = Expr::Accessor(Accessor::Ident(class_ident.clone()));
        let mut new_ident =
            Identifier::bare(None, VarName::from_str_and_line(Str::ever("__new__"), line));
        new_ident.vi.py_name = Some(Str::ever("__call__"));
        let class_new = class.attr_expr(new_ident);
        let call = class_new.call_expr(Args::new(vec![arg], None, vec![], None));
        let block = Block::new(vec![call]);
        let body = DefBody::new(EQUAL, block, DefId(0));
        self.emit_subr_def(Some(class_ident.inspect()), sig, body);
    }

    fn emit_block(&mut self, block: Block, opt_name: Option<Str>, params: Vec<Str>) -> CodeObj {
        log!(info "entered {}", fn_name!());
        self.unit_size += 1;
        let name = if let Some(name) = opt_name {
            name
        } else {
            "<block>".into()
        };
        let firstlineno = block
            .first()
            .and_then(|first| first.ln_begin())
            .unwrap_or(0);
        self.units.push(PyCodeGenUnit::new(
            self.unit_size,
            self.py_version,
            params,
            Str::rc(self.cfg.input.enclosed_name()),
            &name,
            firstlineno,
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
        let init_stack_len = self.stack_len();
        for expr in block.into_iter() {
            self.emit_expr(expr);
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
            self.edit_code(idx_copy_free_vars + 1, freevars_len);
        } else if self.py_version.minor >= Some(11) {
            let code = self.cur_block_codeobj().code.get(idx_copy_free_vars);
            debug_assert_eq!(code, Some(&(Opcode311::COPY_FREE_VARS as u8)));
            self.edit_code(idx_copy_free_vars, CommonOpcode::NOP as usize);
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
                    *l += ld as u8;
                }
                self.mut_cur_block().prev_lineno += ld;
            }
        }
        unit.codeobj
    }

    fn load_prelude(&mut self) {
        self.load_record_type();
        self.load_prelude_py();
        self.prelude_loaded = true;
        self.record_type_loaded = true;
    }

    fn load_in_op(&mut self) {
        let mod_name = if self.py_version.minor >= Some(10) {
            Identifier::public("_erg_std_prelude")
        } else {
            Identifier::public("_erg_std_prelude_old")
        };
        self.emit_global_import_items(
            mod_name,
            vec![(
                Identifier::public("in_operator"),
                Some(Identifier::private("#in_operator")),
            )],
        );
        self.in_op_loaded = true;
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
        self.emit_load_const(erg_std_path().to_str().unwrap());
        self.emit_call_instr(1, Method);
        self.stack_dec();
        self.emit_pop_top();
        let erg_std_mod = if self.py_version.minor >= Some(10) {
            Identifier::public("_erg_std_prelude")
        } else {
            Identifier::public("_erg_std_prelude_old")
        };
        // escaping
        self.emit_global_import_items(
            erg_std_mod.clone(),
            vec![(
                Identifier::public("in_operator"),
                Some(Identifier::private("#in_operator")),
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

    fn load_module_type(&mut self) {
        self.emit_global_import_items(
            Identifier::public("types"),
            vec![(
                Identifier::public("ModuleType"),
                Some(Identifier::private("#ModuleType")),
            )],
        );
    }

    pub fn emit(&mut self, hir: HIR) -> CodeObj {
        log!(info "the code-generating process has started.{RESET}");
        self.unit_size += 1;
        self.units.push(PyCodeGenUnit::new(
            self.unit_size,
            self.py_version,
            vec![],
            Str::rc(self.cfg.input.enclosed_name()),
            "<module>",
            1,
        ));
        if self.py_version.minor >= Some(11) {
            self.write_instr(Opcode311::RESUME);
            self.write_arg(0);
        }
        if !self.cfg.no_std && !self.prelude_loaded {
            self.load_prelude();
        }
        for expr in hir.module.into_iter() {
            self.emit_expr(expr);
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
                    *l += ld as u8;
                }
                self.mut_cur_block().prev_lineno += ld;
            }
        }
        log!(info "the code-generating process has completed.{RESET}");
        unit.codeobj
    }
}
