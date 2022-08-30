//! generates `CodeObj` (equivalent to PyCodeObject of CPython) from `AST`.
//!
//! ASTからPythonバイトコード(コードオブジェクト)を生成する
use std::fmt;
use std::process;

use erg_common::cache::CacheSet;
use erg_common::config::{ErgConfig, Input};
use erg_common::error::{Location, MultiErrorDisplay};
use erg_common::opcode::Opcode;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{
    debug_power_assert, enum_unwrap, fn_name_full, impl_stream_for_wrapper, log, switch_unreachable,
};
use erg_type::codeobj::{CodeObj, CodeObjFlags};
use Opcode::*;

use erg_parser::ast::{Identifier, ParamPattern, Params, VarName};
use erg_parser::token::{Token, TokenKind};

use erg_type::value::ValueObj;
use erg_type::{HasType, TypeCode, TypePair};

use crate::compile::{AccessKind, Name, StoreLoadKind};
use crate::error::{CompileError, CompileErrors, CompileResult};
use crate::eval::eval_lit;
use crate::hir::{
    Accessor, Args, Array, Block, DefBody, Expr, Signature, SubrSignature, Tuple, VarSignature, HIR,
};
use AccessKind::*;

fn is_python_global(name: &str) -> bool {
    match name {
        "ArithmeticError"
        | "AssertionError"
        | "AttributeError"
        | "BaseException"
        | "BlockingIOError"
        | "BrokenPipeError"
        | "BufferError"
        | "BytesWarning"
        | "ChildProcessError"
        | "ConnectionAbortedError"
        | "ConnectionError"
        | "ConnectionRefusedError"
        | "ConnectionResetError"
        | "DeprecationWarning"
        | "EOFError"
        | "Ellipsis"
        | "EncodingWarning"
        | "EnvironmentError"
        | "Exception"
        | "False"
        | "FileExistsError"
        | "FileNotFoundError"
        | "FloatingPointError"
        | "FutureWarning"
        | "GeneratorExit"
        | "IOError"
        | "ImportError"
        | "ImportWarning"
        | "IndentationError"
        | "IndexError"
        | "InterruptedError"
        | "IsADirectoryError"
        | "KeyError"
        | "KeyboardInterrupt"
        | "LookupError"
        | "MemoryError"
        | "ModuleNotFoundError"
        | "NameError"
        | "None"
        | "NotADirectoryError"
        | "NotImplemented"
        | "NotImplementedError"
        | "OSError"
        | "OverflowError"
        | "PendingDeprecationWarning"
        | "PermissionError"
        | "ProcessLookupError"
        | "RecursionError"
        | "ReferenceError"
        | "ResourceWarning"
        | "RuntimeError"
        | "RuntimeWarning"
        | "StopAsyncIteration"
        | "StopIteration"
        | "SyntaxError"
        | "SyntaxWarning"
        | "SystemError"
        | "SystemExit"
        | "TabError"
        | "TimeoutError"
        | "True"
        | "TypeError"
        | "UnboundLocalError"
        | "UnicodeDecodeError"
        | "UnicodeEncodeError"
        | "UnicodeError"
        | "UnicodeTranslateError"
        | "UnicodeWarning"
        | "UserWarning"
        | "ValueError"
        | "Warning"
        | "WindowsError"
        | "ZeroDivisionError"
        | "__build__class__"
        | "__debug__"
        | "__doc__"
        | "__import__"
        | "__loader__"
        | "__name__"
        | "__package__"
        | "__spec__"
        | "__annotations__"
        | "__builtins__"
        | "abs"
        | "aiter"
        | "all"
        | "any"
        | "anext"
        | "ascii"
        | "bin"
        | "bool"
        | "breakpoint"
        | "bytearray"
        | "bytes"
        | "callable"
        | "chr"
        | "classmethod"
        | "compile"
        | "complex"
        | "delattr"
        | "dict"
        | "dir"
        | "divmod"
        | "enumerate"
        | "eval"
        | "exec"
        | "filter"
        | "float"
        | "format"
        | "frozenset"
        | "getattr"
        | "globals"
        | "hasattr"
        | "hash"
        | "help"
        | "hex"
        | "id"
        | "input"
        | "int"
        | "isinstance"
        | "issubclass"
        | "iter"
        | "len"
        | "list"
        | "locals"
        | "map"
        | "max"
        | "memoryview"
        | "min"
        | "next"
        | "object"
        | "oct"
        | "open"
        | "ord"
        | "pow"
        | "print"
        | "property"
        | "quit"
        | "range"
        | "repr"
        | "reversed"
        | "round"
        | "set"
        | "setattr"
        | "slice"
        | "sorted"
        | "staticmethod"
        | "str"
        | "sum"
        | "super"
        | "tuple"
        | "type"
        | "vars"
        | "zip" => true,
        _ => false,
    }
}

fn convert_to_python_attr(class: &str, uniq_obj_name: Option<&str>, name: Str) -> Str {
    match (class, uniq_obj_name, &name[..]) {
        ("Array!", _, "push!") => Str::ever("append"),
        ("Complex" | "Real" | "Int" | "Nat" | "Float", _, "Real") => Str::ever("real"),
        ("Complex" | "Real" | "Int" | "Nat" | "Float", _, "Imag") => Str::ever("imag"),
        ("Module", Some("random"), "randint!") => Str::ever("randint"),
        ("Module", Some("random"), "choice!") => Str::ever("choice"),
        _ => name,
    }
}

fn escape_attr(class: &str, uniq_obj_name: Option<&str>, name: Str) -> Str {
    let mut name = convert_to_python_attr(class, uniq_obj_name, name).to_string();
    name = name.replace('!', "__erg_proc__");
    name = name.replace('$', "__erg_shared__");
    Str::rc(&name)
}

fn convert_to_python_name(name: Str) -> Str {
    match &name[..] {
        "abs" => Str::ever("abs"),
        // assert is implemented in bytecode
        "classof" => Str::ever("type"),
        "compile" => Str::ever("compile"),
        // discard is implemented in bytecode
        // for is implemented in bytecode
        "id" => Str::ever("id"),
        // if is implemented in bytecode
        "import" => Str::ever("__import__"),
        "input!" => Str::ever("input"),
        "log" => Str::ever("print"), // TODO: log != print (prints after executing)
        "print!" => Str::ever("print"),
        "py" | "pyimport" => Str::ever("__import__"),
        "quit" | "exit" => Str::ever("quit"),
        "Nat" | "Nat!" => Str::ever("int"),
        "Int" | "Int!" => Str::ever("int"),
        "Float" | "Float!" => Str::ever("float"),
        "Ratio" | "Ratio!" => Str::ever("float"),
        "Complex" => Str::ever("complex"),
        "Str" | "Str!" => Str::ever("str"),
        "Bool" | "Bool!" => Str::ever("bool"),
        "Array" | "Array!" => Str::ever("list"),
        _ => name,
    }
}

fn escape_name(ident: Identifier) -> Str {
    let vis = ident.vis();
    let mut name = convert_to_python_name(ident.name.into_token().content).to_string();
    name = name.replace('!', "__erg_proc__");
    name = name.replace('$', "__erg_shared__");
    if vis.is_public() || is_python_global(&name) {
        Str::from(name)
    } else {
        Str::from("::".to_string() + &name)
    }
}

#[derive(Debug, Clone)]
pub struct CodeGenUnit {
    pub(crate) id: usize,
    pub(crate) codeobj: CodeObj,
    pub(crate) stack_len: u32, // the maximum stack size
    pub(crate) prev_lineno: usize,
    pub(crate) lasti: usize,
    pub(crate) prev_lasti: usize,
    pub(crate) _refs: Vec<ValueObj>, // ref-counted objects
}

impl PartialEq for CodeGenUnit {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Display for CodeGenUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CompilerUnit{{\nid: {}\ncode:\n{}\n}}",
            self.id,
            self.codeobj.code_info()
        )
    }
}

impl CodeGenUnit {
    pub fn new<S: Into<Str>, T: Into<Str>>(
        id: usize,
        params: Vec<Str>,
        filename: S,
        name: T,
        firstlineno: usize,
    ) -> Self {
        Self {
            id,
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
pub struct CodeGenStack(Vec<CodeGenUnit>);

impl_stream_for_wrapper!(CodeGenStack, CodeGenUnit);

#[derive(Debug)]
pub struct CodeGenerator {
    cfg: ErgConfig,
    str_cache: CacheSet<str>,
    namedtuple_loaded: bool,
    unit_size: usize,
    units: CodeGenStack,
    pub(crate) errs: CompileErrors,
}

impl CodeGenerator {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            cfg,
            str_cache: CacheSet::new(),
            namedtuple_loaded: false,
            unit_size: 0,
            units: CodeGenStack::empty(),
            errs: CompileErrors::empty(),
        }
    }

    pub fn clear(&mut self) {
        self.units.clear();
        self.errs.clear();
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
    fn toplevel_block(&self) -> &CodeGenUnit {
        self.units.first().unwrap()
    }

    #[inline]
    fn cur_block(&self) -> &CodeGenUnit {
        self.units.last().unwrap()
    }

    #[inline]
    fn mut_cur_block(&mut self) -> &mut CodeGenUnit {
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
    fn edit_code(&mut self, idx: usize, code: usize) {
        *self.mut_cur_block_codeobj().code.get_mut(idx).unwrap() = code as u8;
    }

    fn write_instr(&mut self, code: Opcode) {
        self.mut_cur_block_codeobj().code.push(code as u8);
        self.mut_cur_block().lasti += 1;
        // log!(info "wrote: {}", code);
    }

    fn write_arg(&mut self, code: u8) {
        self.mut_cur_block_codeobj().code.push(code);
        self.mut_cur_block().lasti += 1;
        // log!(info "wrote: {}", code);
    }

    fn stack_inc(&mut self) {
        self.mut_cur_block().stack_len += 1;
        if self.cur_block().stack_len > self.cur_block_codeobj().stacksize {
            self.mut_cur_block_codeobj().stacksize = self.cur_block().stack_len;
        }
    }

    fn stack_dec(&mut self) {
        if self.cur_block().stack_len == 0 {
            println!("current block: {}", self.cur_block());
            self.crash("the stack size becomes -1");
        } else {
            self.mut_cur_block().stack_len -= 1;
        }
    }

    fn stack_inc_n(&mut self, n: usize) {
        self.mut_cur_block().stack_len += n as u32;
        if self.cur_block().stack_len > self.cur_block_codeobj().stacksize {
            self.mut_cur_block_codeobj().stacksize = self.cur_block().stack_len;
        }
    }

    fn stack_dec_n(&mut self, n: usize) {
        if n > 0 && self.cur_block().stack_len == 0 {
            self.crash("the stack size becomes -1");
        } else {
            self.mut_cur_block().stack_len -= n as u32;
        }
    }

    fn emit_load_const<C: Into<ValueObj>>(&mut self, cons: C) {
        let cons = cons.into();
        let idx = self
            .mut_cur_block_codeobj()
            .consts
            .iter()
            .position(|c| c == &cons)
            .unwrap_or_else(|| {
                self.mut_cur_block_codeobj().consts.push(cons);
                self.mut_cur_block_codeobj().consts.len() - 1
            });
        self.write_instr(Opcode::LOAD_CONST);
        self.write_arg(idx as u8);
        self.stack_inc();
    }

    fn local_search(&self, name: &str, acc_kind: AccessKind) -> Option<Name> {
        let current_is_toplevel = self.cur_block() == self.toplevel_block();
        if let Some(idx) = self
            .cur_block_codeobj()
            .names
            .iter()
            .position(|n| &**n == name)
        {
            if current_is_toplevel || !acc_kind.is_local() {
                Some(Name::local(idx))
            } else {
                Some(Name::global(idx))
            }
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

    fn register_name(&mut self, ident: Identifier) -> Name {
        let current_is_toplevel = self.cur_block() == self.toplevel_block();
        let name = escape_name(ident);
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
                self.mut_cur_block_codeobj().freevars.push(name);
                Name::deref(self.cur_block_codeobj().freevars.len() - 1)
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

    fn register_attr(&mut self, class: &str, uniq_obj_name: Option<&str>, name: Str) -> Name {
        let name = Str::rc(name.split('.').last().unwrap());
        let name = escape_attr(class, uniq_obj_name, name);
        self.mut_cur_block_codeobj().names.push(name);
        Name::local(self.cur_block_codeobj().names.len() - 1)
    }

    fn register_method(&mut self, class: &str, uniq_obj_name: Option<&str>, name: Str) -> Name {
        let name = Str::rc(name.split('.').last().unwrap());
        let name = escape_attr(class, uniq_obj_name, name);
        self.mut_cur_block_codeobj().names.push(name);
        Name::local(self.cur_block_codeobj().names.len() - 1)
    }

    fn emit_load_name_instr(&mut self, ident: Identifier) -> CompileResult<()> {
        let name = self
            .local_search(ident.inspect(), Name)
            .unwrap_or_else(|| self.register_name(ident));
        let instr = match name.kind {
            StoreLoadKind::Fast | StoreLoadKind::FastConst => Opcode::LOAD_FAST,
            StoreLoadKind::Global | StoreLoadKind::GlobalConst => Opcode::LOAD_GLOBAL,
            StoreLoadKind::Deref | StoreLoadKind::DerefConst => Opcode::LOAD_DEREF,
            StoreLoadKind::Local | StoreLoadKind::LocalConst => Opcode::LOAD_NAME,
        };
        self.write_instr(instr);
        self.write_arg(name.idx as u8);
        self.stack_inc();
        Ok(())
    }

    fn emit_import_name_instr(&mut self, ident: Identifier) -> CompileResult<()> {
        let name = self
            .local_search(ident.inspect(), Name)
            .unwrap_or_else(|| self.register_name(ident));
        self.write_instr(IMPORT_NAME);
        self.write_arg(name.idx as u8);
        self.stack_dec(); // (level + from_list) -> module object
        Ok(())
    }

    fn emit_import_from_instr(&mut self, ident: Identifier) -> CompileResult<()> {
        let name = self
            .local_search(ident.inspect(), Name)
            .unwrap_or_else(|| self.register_name(ident));
        self.write_instr(IMPORT_FROM);
        self.write_arg(name.idx as u8);
        // self.stack_inc(); (module object) -> attribute
        Ok(())
    }

    fn emit_load_attr_instr(
        &mut self,
        class: &str,
        uniq_obj_name: Option<&str>,
        name: Str,
    ) -> CompileResult<()> {
        let name = self
            .local_search(&name, Attr)
            .unwrap_or_else(|| self.register_attr(class, uniq_obj_name, name));
        let instr = match name.kind {
            StoreLoadKind::Fast | StoreLoadKind::FastConst => Opcode::LOAD_FAST,
            StoreLoadKind::Global | StoreLoadKind::GlobalConst => Opcode::LOAD_GLOBAL,
            StoreLoadKind::Deref | StoreLoadKind::DerefConst => Opcode::LOAD_DEREF,
            StoreLoadKind::Local | StoreLoadKind::LocalConst => Opcode::LOAD_ATTR,
        };
        self.write_instr(instr);
        self.write_arg(name.idx as u8);
        Ok(())
    }

    fn emit_load_method_instr(
        &mut self,
        class: &str,
        uniq_obj_name: Option<&str>,
        name: Str,
    ) -> CompileResult<()> {
        let name = self
            .local_search(&name, Method)
            .unwrap_or_else(|| self.register_method(class, uniq_obj_name, name));
        let instr = match name.kind {
            StoreLoadKind::Fast | StoreLoadKind::FastConst => Opcode::LOAD_FAST,
            StoreLoadKind::Global | StoreLoadKind::GlobalConst => Opcode::LOAD_GLOBAL,
            StoreLoadKind::Deref | StoreLoadKind::DerefConst => Opcode::LOAD_DEREF,
            StoreLoadKind::Local | StoreLoadKind::LocalConst => Opcode::LOAD_METHOD,
        };
        self.write_instr(instr);
        self.write_arg(name.idx as u8);
        Ok(())
    }

    fn emit_store_instr(&mut self, ident: Identifier, acc_kind: AccessKind) {
        let name = self
            .local_search(ident.inspect(), acc_kind)
            .unwrap_or_else(|| self.register_name(ident));
        let instr = match name.kind {
            StoreLoadKind::Fast => Opcode::STORE_FAST,
            StoreLoadKind::FastConst => Opcode::ERG_STORE_FAST_IMMUT,
            StoreLoadKind::Global | StoreLoadKind::GlobalConst => Opcode::STORE_GLOBAL,
            StoreLoadKind::Deref | StoreLoadKind::DerefConst => Opcode::STORE_DEREF,
            StoreLoadKind::Local | StoreLoadKind::LocalConst => {
                match acc_kind {
                    AccessKind::Name => Opcode::STORE_NAME,
                    AccessKind::Attr => Opcode::STORE_ATTR,
                    // cannot overwrite methods directly
                    AccessKind::Method => Opcode::STORE_ATTR,
                }
            }
        };
        self.write_instr(instr);
        self.write_arg(name.idx as u8);
        self.stack_dec();
    }

    fn emit_pop_top(&mut self) {
        self.write_instr(Opcode::POP_TOP);
        self.write_arg(0u8);
        self.stack_dec();
    }

    fn cancel_pop_top(&mut self) {
        let lasop_t_idx = self.cur_block_codeobj().code.len() - 2;
        if self.cur_block_codeobj().code.get(lasop_t_idx) == Some(&(Opcode::POP_TOP as u8)) {
            self.mut_cur_block_codeobj().code.pop();
            self.mut_cur_block_codeobj().code.pop();
            self.mut_cur_block().lasti -= 2;
            self.stack_inc();
        }
    }

    /// Compileが継続不能になった際呼び出す
    /// 極力使わないこと
    fn crash(&mut self, description: &'static str) -> ! {
        self.errs.fmt_all_stderr();
        if cfg!(feature = "debug") {
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
            .chain(
                params
                    .defaults
                    .iter()
                    .map(|p| p.inspect().map(|s| &s[..]).unwrap_or("_")),
            )
            .map(|s| self.get_cached(s))
            .collect()
    }

    fn emit_mono_type_def(&mut self, sig: VarSignature, body: DefBody) {
        self.write_instr(Opcode::LOAD_BUILD_CLASS);
        self.write_arg(0);
        self.stack_inc();
        let code = self.codegen_typedef_block(sig.inspect().clone(), body.block);
        self.emit_load_const(code);
        self.emit_load_const(sig.inspect().clone());
        self.write_instr(Opcode::MAKE_FUNCTION);
        self.write_arg(0);
        self.emit_load_const(sig.inspect().clone());
        self.write_instr(Opcode::CALL_FUNCTION);
        self.write_arg(2);
        self.stack_dec_n((1 + 2) - 1);
        self.emit_store_instr(sig.ident, Name);
    }

    fn emit_var_def(&mut self, sig: VarSignature, mut body: DefBody) {
        if body.is_type() {
            return self.emit_mono_type_def(sig, body);
        }
        if body.block.len() == 1 {
            self.codegen_expr(body.block.remove(0));
        } else {
            self.codegen_frameless_block(body.block, vec![]);
        }
        self.emit_store_instr(sig.ident, Name);
    }

    fn emit_subr_def(&mut self, sig: SubrSignature, body: DefBody) {
        let name = sig.ident.inspect().clone();
        let mut opcode_flag = 0u8;
        let params = self.gen_param_names(&sig.params);
        let code = self.codegen_block(body.block, Some(name.clone()), params);
        self.emit_load_const(code);
        if !self.cur_block_codeobj().cellvars.is_empty() {
            let cellvars_len = self.cur_block_codeobj().cellvars.len() as u8;
            for i in 0..cellvars_len {
                self.write_instr(LOAD_CLOSURE);
                self.write_arg(i);
            }
            self.write_instr(BUILD_TUPLE);
            self.write_arg(cellvars_len);
            opcode_flag += 8;
        }
        self.emit_load_const(name.clone());
        self.write_instr(MAKE_FUNCTION);
        self.write_arg(opcode_flag);
        // stack_dec: <code obj> + <name> -> <function>
        self.stack_dec();
        self.emit_store_instr(sig.ident, Name);
    }

    fn emit_discard_instr(&mut self, mut args: Args) -> CompileResult<()> {
        while let Some(arg) = args.try_remove(0) {
            self.codegen_expr(arg);
            self.emit_pop_top();
        }
        Ok(())
    }

    fn emit_if_instr(&mut self, mut args: Args) -> CompileResult<()> {
        let cond = args.remove(0);
        self.codegen_expr(cond);
        let idx_pop_jump_if_false = self.cur_block().lasti;
        self.write_instr(POP_JUMP_IF_FALSE);
        // cannot detect where to jump to at this moment, so put as 0
        self.write_arg(0_u8);
        match args.remove(0) {
            // then block
            Expr::Lambda(lambda) => {
                let params = self.gen_param_names(&lambda.params);
                self.codegen_frameless_block(lambda.body, params);
            }
            other => {
                self.codegen_expr(other);
            }
        }
        if args.get(0).is_some() {
            self.write_instr(JUMP_FORWARD); // jump to end
            self.write_arg(0_u8);
            // else block
            let idx_else_begin = self.cur_block().lasti;
            self.edit_code(idx_pop_jump_if_false + 1, idx_else_begin / 2);
            match args.remove(0) {
                Expr::Lambda(lambda) => {
                    let params = self.gen_param_names(&lambda.params);
                    self.codegen_frameless_block(lambda.body, params);
                }
                other => {
                    self.codegen_expr(other);
                }
            }
            let idx_jump_forward = idx_else_begin - 2;
            let idx_end = self.cur_block().lasti;
            self.edit_code(idx_jump_forward + 1, (idx_end - idx_jump_forward - 2) / 2);
            self.stack_dec();
            self.stack_dec();
        } else {
            // no else block
            let idx_end = self.cur_block().lasti;
            self.edit_code(idx_pop_jump_if_false + 1, idx_end / 2);
            self.emit_load_const(ValueObj::None);
            self.stack_dec();
            self.stack_dec();
        }
        Ok(())
    }

    fn emit_for_instr(&mut self, mut args: Args) -> CompileResult<()> {
        let iterable = args.remove(0);
        self.codegen_expr(iterable);
        self.write_instr(GET_ITER);
        self.write_arg(0);
        let idx_for_iter = self.cur_block().lasti;
        self.write_instr(FOR_ITER);
        // FOR_ITER pushes a value onto the stack, but we can't know how many
        // but after executing this instruction, stack_len should be 1
        // cannot detect where to jump to at this moment, so put as 0
        self.write_arg(0);
        let lambda = enum_unwrap!(args.remove(0), Expr::Lambda);
        let params = self.gen_param_names(&lambda.params);
        self.codegen_frameless_block(lambda.body, params); // ここでPOPされる
        self.write_instr(JUMP_ABSOLUTE);
        self.write_arg((idx_for_iter / 2) as u8);
        let idx_end = self.cur_block().lasti;
        self.edit_code(idx_for_iter + 1, (idx_end - idx_for_iter - 2) / 2);
        self.emit_load_const(ValueObj::None);
        Ok(())
    }

    fn emit_match_instr(&mut self, mut args: Args, _use_erg_specific: bool) -> CompileResult<()> {
        let expr = args.remove(0);
        self.codegen_expr(expr);
        let len = args.len();
        let mut absolute_jump_points = vec![];
        while let Some(expr) = args.try_remove(0) {
            // パターンが複数ある場合引数を複製する、ただし最後はしない
            if len > 1 && !args.is_empty() {
                self.write_instr(Opcode::DUP_TOP);
                self.write_arg(0);
                self.stack_inc();
            }
            // compilerで型チェック済み(可読性が下がるため、matchでNamedは使えない)
            let mut lambda = enum_unwrap!(expr, Expr::Lambda);
            debug_power_assert!(lambda.params.len(), ==, 1);
            if !lambda.params.defaults.is_empty() {
                todo!("default values in match expression are not supported yet")
            }
            let pat = lambda.params.non_defaults.remove(0).pat;
            let pop_jump_points = self.emit_match_pattern(pat)?;
            self.codegen_frameless_block(lambda.body, Vec::new());
            for pop_jump_point in pop_jump_points.into_iter() {
                let idx = self.cur_block().lasti + 2;
                self.edit_code(pop_jump_point + 1, idx / 2); // jump to POP_TOP
                absolute_jump_points.push(self.cur_block().lasti);
                self.write_instr(Opcode::JUMP_ABSOLUTE); // jump to the end
                self.write_arg(0);
            }
        }
        let lasti = self.cur_block().lasti;
        for absolute_jump_point in absolute_jump_points.into_iter() {
            self.edit_code(absolute_jump_point + 1, lasti / 2);
        }
        Ok(())
    }

    fn emit_match_pattern(&mut self, pat: ParamPattern) -> CompileResult<Vec<usize>> {
        let mut pop_jump_points = vec![];
        match pat {
            ParamPattern::VarName(name) => {
                let ident = Identifier::new(None, name);
                self.emit_store_instr(ident, AccessKind::Name);
            }
            ParamPattern::Lit(lit) => {
                self.emit_load_const(eval_lit(&lit));
                self.write_instr(Opcode::COMPARE_OP);
                self.write_arg(2); // ==
                self.stack_dec();
                pop_jump_points.push(self.cur_block().lasti);
                self.write_instr(Opcode::POP_JUMP_IF_FALSE); // jump to the next case
                self.write_arg(0);
                self.emit_pop_top();
                self.stack_dec();
            }
            ParamPattern::Array(arr) => {
                let len = arr.len();
                self.write_instr(Opcode::MATCH_SEQUENCE);
                self.write_arg(0);
                pop_jump_points.push(self.cur_block().lasti);
                self.write_instr(Opcode::POP_JUMP_IF_FALSE);
                self.write_arg(0);
                self.stack_dec();
                self.write_instr(Opcode::GET_LEN);
                self.write_arg(0);
                self.emit_load_const(len);
                self.write_instr(Opcode::COMPARE_OP);
                self.write_arg(2); // ==
                self.stack_dec();
                pop_jump_points.push(self.cur_block().lasti);
                self.write_instr(Opcode::POP_JUMP_IF_FALSE);
                self.write_arg(0);
                self.stack_dec();
                self.write_instr(Opcode::UNPACK_SEQUENCE);
                self.write_arg(len as u8);
                self.stack_inc_n(len - 1);
                for elem in arr.elems.non_defaults {
                    pop_jump_points.append(&mut self.emit_match_pattern(elem.pat)?);
                }
                if !arr.elems.defaults.is_empty() {
                    todo!("default values in match are not supported yet")
                }
            }
            _other => {
                todo!()
            }
        }
        Ok(pop_jump_points)
    }

    fn emit_call_special(&mut self, name: VarName, mut args: Args) -> CompileResult<()> {
        match &name.inspect()[..] {
            "assert" => self.emit_assert_instr(args),
            "discard" => self.emit_discard_instr(args),
            "for" | "for!" => self.emit_for_instr(args),
            "if" | "if!" => self.emit_if_instr(args),
            "match" | "match!" => self.emit_match_instr(args, true),
            _ => {
                let ident = Identifier::new(None, name);
                self.emit_load_name_instr(ident).unwrap_or_else(|e| {
                    self.errs.push(e);
                });
                let argc = args.len();
                let mut kws = Vec::with_capacity(args.kw_len());
                while let Some(arg) = args.try_remove_pos(0) {
                    self.codegen_expr(arg.expr);
                }
                while let Some(arg) = args.try_remove_kw(0) {
                    kws.push(ValueObj::Str(arg.keyword.content.clone()));
                    self.codegen_expr(arg.expr);
                }
                let kwsc = if !kws.is_empty() {
                    let kws_tuple = ValueObj::from(kws);
                    self.emit_load_const(kws_tuple);
                    self.write_instr(CALL_FUNCTION_KW);
                    1
                } else {
                    self.write_instr(CALL_FUNCTION);
                    0
                };
                self.write_arg(argc as u8);
                // (1 (subroutine) + argc + kwsc) input objects -> 1 return object
                self.stack_dec_n((1 + argc + kwsc) - 1);
                Ok(())
            }
        }
    }

    fn emit_call(&mut self, obj: Expr, method_name: Option<Token>, mut args: Args) {
        let class = obj.ref_t().name(); // これは必ずmethodのあるクラスになっている
        let uniq_obj_name = obj.__name__().map(Str::rc);
        self.codegen_expr(obj);
        if let Some(method_name) = method_name {
            self.emit_load_method_instr(
                &class,
                uniq_obj_name.as_ref().map(|s| &s[..]),
                method_name.content,
            )
            .unwrap_or_else(|err| {
                self.errs.push(err);
            });
        }
        let argc = args.len();
        let mut kws = Vec::with_capacity(args.kw_len());
        while let Some(arg) = args.try_remove_pos(0) {
            self.codegen_expr(arg.expr);
        }
        while let Some(arg) = args.try_remove_kw(0) {
            kws.push(ValueObj::Str(arg.keyword.content.clone()));
            self.codegen_expr(arg.expr);
        }
        let kwsc = if !kws.is_empty() {
            let kws_tuple = ValueObj::from(kws);
            self.emit_load_const(kws_tuple);
            self.write_instr(CALL_FUNCTION_KW);
            1
        } else {
            self.write_instr(CALL_METHOD);
            0
        };
        self.write_arg(argc as u8);
        // (1 (method) + argc + kwsc) input objects -> 1 return object
        self.stack_dec_n((1 + argc + kwsc) - 1);
    }

    // assert takes 1 or 2 arguments (0: cond, 1: message)
    fn emit_assert_instr(&mut self, mut args: Args) -> CompileResult<()> {
        self.codegen_expr(args.remove(0));
        let pop_jump_point = self.cur_block().lasti;
        self.write_instr(Opcode::POP_JUMP_IF_TRUE);
        self.write_arg(0);
        self.stack_dec();
        self.write_instr(Opcode::LOAD_ASSERTION_ERROR);
        self.write_arg(0);
        if let Some(expr) = args.try_remove(0) {
            self.codegen_expr(expr);
            self.write_instr(Opcode::CALL_FUNCTION);
            self.write_arg(1);
        }
        self.write_instr(Opcode::RAISE_VARARGS);
        self.write_arg(1);
        let idx = self.cur_block().lasti;
        self.edit_code(pop_jump_point + 1, idx / 2); // jump to POP_TOP
        Ok(())
    }

    fn codegen_expr(&mut self, expr: Expr) {
        if expr.ln_begin().unwrap_or_else(|| panic!("{expr}")) > self.cur_block().prev_lineno {
            let sd = self.cur_block().lasti - self.cur_block().prev_lasti;
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
                self.mut_cur_block().prev_lasti = self.cur_block().lasti;
            } else {
                self.errs.push(CompileError::compiler_bug(
                    0,
                    self.cfg.input.clone(),
                    expr.loc(),
                    fn_name_full!(),
                    line!(),
                ));
                self.crash("codegen failed: invalid bytecode format");
            }
        }
        match expr {
            Expr::Lit(lit) => {
                self.emit_load_const(lit.value);
            }
            Expr::Accessor(acc) => self.codegen_acc(acc),
            Expr::Def(def) => match def.sig {
                Signature::Subr(sig) => self.emit_subr_def(sig, def.body),
                Signature::Var(sig) => self.emit_var_def(sig, def.body),
            },
            // TODO:
            Expr::Lambda(lambda) => {
                let params = self.gen_param_names(&lambda.params);
                self.codegen_block(lambda.body, Some("<lambda>".into()), params);
                self.emit_load_const("<lambda>");
                self.write_instr(MAKE_FUNCTION);
                self.write_arg(0u8);
                // stack_dec: <lambda code obj> + <name "<lambda>"> -> <function>
                self.stack_dec();
            }
            Expr::UnaryOp(unary) => {
                let tycode = TypeCode::from(unary.lhs_t());
                self.codegen_expr(*unary.expr);
                let instr = match &unary.op.kind {
                    // TODO:
                    TokenKind::PrePlus => UNARY_POSITIVE,
                    TokenKind::PreMinus => UNARY_NEGATIVE,
                    TokenKind::Mutate => NOP, // ERG_MUTATE,
                    _ => {
                        self.errs.push(CompileError::feature_error(
                            self.cfg.input.clone(),
                            unary.op.loc(),
                            "",
                            unary.op.content.clone(),
                        ));
                        NOT_IMPLEMENTED
                    }
                };
                self.write_instr(instr);
                self.write_arg(tycode as u8);
            }
            Expr::BinOp(bin) => {
                // TODO: and/orのプリミティブ命令の実装
                // Range operators are not operators in Python
                match &bin.op.kind {
                    // l..<r == range(l, r)
                    TokenKind::RightOpen => {
                        self.emit_load_name_instr(Identifier::public("range"))
                            .unwrap();
                    }
                    TokenKind::LeftOpen | TokenKind::Closed | TokenKind::Open => todo!(),
                    _ => {}
                }
                let type_pair = TypePair::new(bin.lhs_t(), bin.rhs_t());
                self.codegen_expr(*bin.lhs);
                self.codegen_expr(*bin.rhs);
                let instr = match &bin.op.kind {
                    TokenKind::Plus => BINARY_ADD,
                    TokenKind::Minus => BINARY_SUBTRACT,
                    TokenKind::Star => BINARY_MULTIPLY,
                    TokenKind::Slash => BINARY_TRUE_DIVIDE,
                    TokenKind::Pow => BINARY_POWER,
                    TokenKind::Mod => BINARY_MODULO,
                    TokenKind::AndOp => BINARY_AND,
                    TokenKind::OrOp => BINARY_OR,
                    TokenKind::Less
                    | TokenKind::LessEq
                    | TokenKind::DblEq
                    | TokenKind::NotEq
                    | TokenKind::Gre
                    | TokenKind::GreEq => COMPARE_OP,
                    TokenKind::LeftOpen
                    | TokenKind::RightOpen
                    | TokenKind::Closed
                    | TokenKind::Open => CALL_FUNCTION, // ERG_BINARY_RANGE,
                    _ => {
                        self.errs.push(CompileError::feature_error(
                            self.cfg.input.clone(),
                            bin.op.loc(),
                            "",
                            bin.op.content.clone(),
                        ));
                        NOT_IMPLEMENTED
                    }
                };
                let arg = match &bin.op.kind {
                    TokenKind::Less => 0,
                    TokenKind::LessEq => 1,
                    TokenKind::DblEq => 2,
                    TokenKind::NotEq => 3,
                    TokenKind::Gre => 4,
                    TokenKind::GreEq => 5,
                    TokenKind::LeftOpen
                    | TokenKind::RightOpen
                    | TokenKind::Closed
                    | TokenKind::Open => 2,
                    _ => type_pair as u8,
                };
                self.write_instr(instr);
                self.write_arg(arg);
                self.stack_dec();
                match &bin.op.kind {
                    TokenKind::LeftOpen
                    | TokenKind::RightOpen
                    | TokenKind::Open
                    | TokenKind::Closed => {
                        self.stack_dec();
                    }
                    _ => {}
                }
            }
            Expr::Call(call) => match *call.obj {
                Expr::Accessor(Accessor::Local(local)) => {
                    let name = VarName::new(local.name);
                    self.emit_call_special(name, call.args).unwrap();
                }
                obj => {
                    self.emit_call(obj, call.method_name, call.args);
                }
            },
            // TODO: list comprehension
            Expr::Array(arr) => match arr {
                Array::Normal(mut arr) => {
                    let len = arr.elems.len();
                    while let Some(arg) = arr.elems.try_remove_pos(0) {
                        self.codegen_expr(arg.expr);
                    }
                    self.write_instr(BUILD_LIST);
                    self.write_arg(len as u8);
                    if len == 0 {
                        self.stack_inc();
                    } else {
                        self.stack_dec_n(len - 1);
                    }
                }
                other => todo!("{other}"),
            },
            // TODO: tuple comprehension
            // TODO: tuples can be const
            Expr::Tuple(tup) => match tup {
                Tuple::Normal(mut tup) => {
                    let len = tup.elems.len();
                    while let Some(arg) = tup.elems.try_remove_pos(0) {
                        self.codegen_expr(arg.expr);
                    }
                    self.write_instr(BUILD_TUPLE);
                    self.write_arg(len as u8);
                    if len == 0 {
                        self.stack_inc();
                    } else {
                        self.stack_dec_n(len - 1);
                    }
                }
            },
            Expr::Record(rec) => {
                let attrs_len = rec.attrs.len();
                // importing namedtuple
                if !self.namedtuple_loaded {
                    self.emit_load_const(0);
                    self.emit_load_const(ValueObj::Tuple(std::rc::Rc::from([ValueObj::Str(
                        Str::ever("namedtuple"),
                    )])));
                    let ident = Identifier::public("collections");
                    self.emit_import_name_instr(ident).unwrap();
                    let ident = Identifier::public("namedtuple");
                    self.emit_import_from_instr(ident).unwrap();
                    let ident = Identifier::private(Str::ever("#NamedTuple"));
                    self.emit_store_instr(ident, Name);
                    self.namedtuple_loaded = true;
                }
                // making record type
                let ident = Identifier::private(Str::ever("#NamedTuple"));
                self.emit_load_name_instr(ident).unwrap();
                // record name, let it be anonymous
                self.emit_load_const("Record");
                for field in rec.attrs.iter() {
                    self.emit_load_const(ValueObj::Str(field.sig.ident().inspect().clone()));
                }
                self.write_instr(BUILD_LIST);
                self.write_arg(attrs_len as u8);
                if attrs_len == 0 {
                    self.stack_inc();
                } else {
                    self.stack_dec_n(attrs_len - 1);
                }
                self.write_instr(CALL_FUNCTION);
                self.write_arg(2);
                // (1 (subroutine) + argc + kwsc) input objects -> 1 return object
                self.stack_dec_n((1 + 2 + 0) - 1);
                let ident = Identifier::private(Str::ever("#rec"));
                self.emit_store_instr(ident, Name);
                // making record instance
                let ident = Identifier::private(Str::ever("#rec"));
                self.emit_load_name_instr(ident).unwrap();
                for field in rec.attrs.into_iter() {
                    self.codegen_frameless_block(field.body.block, vec![]);
                }
                self.write_instr(CALL_FUNCTION);
                self.write_arg(attrs_len as u8);
                // (1 (subroutine) + argc + kwsc) input objects -> 1 return object
                self.stack_dec_n((1 + attrs_len + 0) - 1);
            }
            other => {
                self.errs.push(CompileError::feature_error(
                    self.cfg.input.clone(),
                    other.loc(),
                    "???",
                    "".into(),
                ));
                self.crash("cannot compile this expression at this time");
            }
        }
    }

    fn codegen_acc(&mut self, acc: Accessor) {
        match acc {
            Accessor::Local(local) => {
                self.emit_load_name_instr(Identifier::new(None, VarName::new(local.name)))
                    .unwrap_or_else(|err| {
                        self.errs.push(err);
                    });
            }
            Accessor::Public(public) => {
                let ident = Identifier::new(Some(public.dot), VarName::new(public.name));
                self.emit_load_name_instr(ident).unwrap_or_else(|err| {
                    self.errs.push(err);
                });
            }
            Accessor::Attr(a) => {
                let class = a.obj.ref_t().name();
                let uniq_obj_name = a.obj.__name__().map(Str::rc);
                self.codegen_expr(*a.obj);
                self.emit_load_attr_instr(
                    &class,
                    uniq_obj_name.as_ref().map(|s| &s[..]),
                    a.name.content.clone(),
                )
                .unwrap_or_else(|err| {
                    self.errs.push(err);
                });
            }
            Accessor::TupleAttr(t_attr) => {
                self.codegen_expr(*t_attr.obj);
                self.emit_load_const(t_attr.index.value);
                self.write_instr(BINARY_SUBSCR);
                self.write_arg(0);
                self.stack_dec();
            }
            Accessor::Subscr(subscr) => {
                self.codegen_expr(*subscr.obj);
                self.codegen_expr(*subscr.index);
                self.write_instr(BINARY_SUBSCR);
                self.write_arg(0);
                self.stack_dec();
            }
        }
    }

    /// forブロックなどで使う
    fn codegen_frameless_block(&mut self, block: Block, params: Vec<Str>) {
        for param in params {
            self.emit_store_instr(Identifier::private(param), Name);
        }
        for expr in block.into_iter() {
            self.codegen_expr(expr);
            // TODO: discard
            // 最終的に帳尻を合わせる(コード生成の順番的にスタックの整合性が一時的に崩れる場合がある)
            if self.cur_block().stack_len == 1 {
                self.emit_pop_top();
            }
        }
        self.cancel_pop_top();
    }

    fn codegen_typedef_block(&mut self, name: Str, block: Block) -> CodeObj {
        self.unit_size += 1;
        self.units.push(CodeGenUnit::new(
            self.unit_size,
            vec![],
            Str::rc(self.cfg.input.enclosed_name()),
            &name,
            block[0].ln_begin().unwrap(),
        ));
        let mod_name = self.toplevel_block_codeobj().name.clone();
        self.emit_load_const(mod_name);
        self.emit_store_instr(Identifier::public("__module__"), Attr);
        self.emit_load_const(name);
        self.emit_store_instr(Identifier::public("__qualname__"), Attr);
        // TODO: サブルーチンはT.subという書式でSTORE
        for expr in block.into_iter() {
            self.codegen_expr(expr);
            // TODO: discard
            if self.cur_block().stack_len == 1 {
                self.emit_pop_top();
            }
        }
        self.emit_load_const(ValueObj::None);
        self.write_instr(RETURN_VALUE);
        self.write_arg(0u8);
        if self.cur_block().stack_len > 1 {
            let block_id = self.cur_block().id;
            let stack_len = self.cur_block().stack_len;
            self.errs.push(CompileError::stack_bug(
                self.input().clone(),
                Location::Unknown,
                stack_len,
                block_id,
                fn_name_full!(),
            ));
            self.crash("error in codegen_typedef_block: invalid stack size");
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

    fn codegen_block(&mut self, block: Block, opt_name: Option<Str>, params: Vec<Str>) -> CodeObj {
        self.unit_size += 1;
        let name = if let Some(name) = opt_name {
            name
        } else {
            "<block>".into()
        };
        let firstlineno = block.first().unwrap().ln_begin().unwrap();
        self.units.push(CodeGenUnit::new(
            self.unit_size,
            params,
            Str::rc(self.cfg.input.enclosed_name()),
            &name,
            firstlineno,
        ));
        for expr in block.into_iter() {
            self.codegen_expr(expr);
            // NOTE: 各行のトップレベルでは0個または1個のオブジェクトが残っている
            // Pythonの場合使わなかったオブジェクトはそのまま捨てられるが、Ergではdiscardを使う必要がある
            // TODO: discard
            if self.cur_block().stack_len == 1 {
                self.emit_pop_top();
            }
        }
        self.cancel_pop_top(); // 最後の値は戻り値として取っておく
        if self.cur_block().stack_len == 0 {
            self.emit_load_const(ValueObj::None);
        } else if self.cur_block().stack_len > 1 {
            let block_id = self.cur_block().id;
            let stack_len = self.cur_block().stack_len;
            self.errs.push(CompileError::stack_bug(
                self.input().clone(),
                Location::Unknown,
                stack_len,
                block_id,
                fn_name_full!(),
            ));
            self.crash("error in codegen_block: invalid stack size");
        }
        self.write_instr(RETURN_VALUE);
        self.write_arg(0u8);
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

    pub fn codegen(&mut self, hir: HIR) -> CodeObj {
        log!(info "the code-generating process has started.{RESET}");
        self.unit_size += 1;
        self.units.push(CodeGenUnit::new(
            self.unit_size,
            vec![],
            Str::rc(self.cfg.input.enclosed_name()),
            "<module>",
            1,
        ));
        let mut print_point = 0;
        if self.input().is_repl() {
            print_point = self.cur_block().lasti;
            self.emit_load_name_instr(Identifier::public("print"))
                .unwrap();
            // Consistency will be taken later (when NOP replacing)
            // 後で(NOP書き換え時)整合性を取る
            self.stack_dec();
        }
        for expr in hir.module.into_iter() {
            self.codegen_expr(expr);
            // TODO: discard
            if self.cur_block().stack_len == 1 {
                self.emit_pop_top();
            }
        }
        self.cancel_pop_top(); // 最後の値は戻り値として取っておく
        if self.input().is_repl() {
            if self.cur_block().stack_len == 0 {
                // remains `print`, nothing to be printed
                self.edit_code(print_point, Opcode::NOP as usize);
            } else {
                self.stack_inc();
                self.write_instr(CALL_FUNCTION);
                self.write_arg(1_u8);
            }
            self.stack_dec_n(self.cur_block().stack_len as usize);
        }
        if self.cur_block().stack_len == 0 {
            self.emit_load_const(ValueObj::None);
        } else if self.cur_block().stack_len > 1 {
            let block_id = self.cur_block().id;
            let stack_len = self.cur_block().stack_len;
            self.errs.push(CompileError::stack_bug(
                self.input().clone(),
                Location::Unknown,
                stack_len,
                block_id,
                fn_name_full!(),
            ));
            self.crash("error in codegen: invalid stack size");
        }
        self.write_instr(RETURN_VALUE);
        self.write_arg(0u8);
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
