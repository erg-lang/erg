//! generates `CodeObj` (equivalent to PyCodeObject of CPython) from `AST`.
//!
//! ASTからPythonバイトコード(コードオブジェクト)を生成する
use std::fmt;
use std::process;

use erg_common::astr::AtomicStr;
use erg_common::cache::CacheSet;
use erg_common::config::{ErgConfig, Input};
use erg_common::error::{ErrorDisplay, Location};
use erg_common::opcode::Opcode;
use erg_common::traits::{Locational, Stream};
use erg_common::Str;
use erg_common::{
    debug_power_assert, enum_unwrap, fn_name, fn_name_full, impl_stream_for_wrapper, log,
    switch_unreachable,
};
use erg_parser::ast::DefId;
use erg_parser::ast::DefKind;
use erg_type::codeobj::{CodeObj, CodeObjFlags};
use Opcode::*;

use erg_parser::ast::{ParamPattern, ParamSignature, Params, VarName};
use erg_parser::token::{Token, TokenKind};

use erg_type::free::fresh_varname;
use erg_type::value::TypeKind;
use erg_type::value::ValueObj;
use erg_type::{HasType, Type, TypeCode, TypePair};

use crate::compile::{AccessKind, Name, StoreLoadKind};
use crate::context::eval::eval_lit;
use crate::error::CompileError;
use crate::hir::{
    Accessor, Args, Array, AttrDef, Attribute, BinOp, Block, Call, ClassDef, Def, DefBody, Expr,
    Identifier, Lambda, Literal, PosArg, Record, Signature, SubrSignature, Tuple, UnaryOp,
    VarSignature, HIR,
};
use AccessKind::*;

fn is_python_special(name: &str) -> bool {
    matches!(name, "__call__" | "__init__")
}

fn is_python_global(name: &str) -> bool {
    matches!(
        name,
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
            | "exit"
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
            | "zip"
    )
}

fn is_fake_method(class: &str, name: &str) -> bool {
    matches!(
        (class, name),
        (
            "Complex" | "Float" | "Ratio" | "Int" | "Nat" | "Bool",
            "abs"
        ) | (_, "iter")
            | (_, "map")
    )
}

fn convert_to_python_attr(class: &str, uniq_obj_name: Option<&str>, name: Str) -> Str {
    match (class, uniq_obj_name, &name[..]) {
        ("Array!", _, "push!") => Str::ever("append"),
        ("Set!", _, "add") => Str::ever("add"),
        ("Complex" | "Float" | "Ratio" | "Int" | "Nat" | "Bool", _, "Real") => Str::ever("real"),
        ("Complex" | "Float" | "Ratio" | "Int" | "Nat" | "Bool", _, "Imag") => Str::ever("imag"),
        ("File!", _, "read!") => Str::ever("read"),
        (_, _, "__new__") => Str::ever("__call__"),
        (_, _, "to_str") => Str::ever("__str__"),
        ("StringIO!", _, "getvalue!") => Str::ever("getvalue"),
        ("Module", Some("importlib"), "reload!") => Str::ever("reload"),
        ("Module", Some("random"), "randint!") => Str::ever("randint"),
        ("Module", Some("random"), "choice!") => Str::ever("choice"),
        ("Module", Some("sys"), "setrecurtionlimit!") => Str::ever("setrecurtionlimit"),
        ("Module", Some("time"), "sleep!") => Str::ever("sleep"),
        ("Module", Some("time"), "time!") => Str::ever("time"),
        _ => name,
    }
}

fn escape_attr(class: &str, uniq_obj_name: Option<&str>, ident: Identifier) -> Str {
    let vis = ident.vis();
    let mut name =
        convert_to_python_attr(class, uniq_obj_name, ident.name.into_token().content).to_string();
    name = name.replace('!', "__erg_proc__");
    name = name.replace('$', "__erg_shared__");
    if vis.is_public() || is_python_global(&name) || is_python_special(&name) {
        Str::from(name)
    } else {
        Str::from("::".to_string() + &name)
    }
}

fn convert_to_python_name(name: Str) -> Str {
    match &name[..] {
        "abs" => Str::ever("abs"),
        // assert is implemented in bytecode
        "classof" => Str::ever("type"),
        "compile" => Str::ever("compile"),
        "dir!" => Str::ever("dir"),
        // discard is implemented in bytecode
        // for is implemented in bytecode
        "id!" => Str::ever("id"),
        // if is implemented in bytecode
        "import" => Str::ever("__import__"),
        "input!" => Str::ever("input"),
        "log" => Str::ever("print"), // TODO: log != print (prints after executing)
        "open!" => Str::ever("open"),
        "panic" => Str::ever("exit"),
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
        "Set" | "Set!" => Str::ever("set"),
        _ => name,
    }
}

fn escape_name(ident: Identifier) -> Str {
    let vis = ident.vis();
    let mut name = convert_to_python_name(ident.name.into_token().content).to_string();
    name = name.replace('!', "__erg_proc__");
    name = name.replace('$', "__erg_shared__");
    if vis.is_public() || is_python_global(&name) || is_python_special(&name) {
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
    prelude_loaded: bool,
    record_type_loaded: bool,
    module_type_loaded: bool,
    abc_loaded: bool,
    unit_size: usize,
    units: CodeGenStack,
}

impl CodeGenerator {
    pub fn new(cfg: ErgConfig) -> Self {
        Self {
            cfg,
            str_cache: CacheSet::new(),
            prelude_loaded: false,
            record_type_loaded: false,
            module_type_loaded: false,
            abc_loaded: false,
            unit_size: 0,
            units: CodeGenStack::empty(),
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

    /// NOTE: For example, an operation that increases the stack by 2 and decreases it by 1 should be `stack_inc_n(2); stack_dec();` not `stack_inc(1);`.
    /// This is because the stack size will not increase correctly.
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
                // cellvarsのpushはrec_search()で行われる
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

    fn register_attr(&mut self, name: Str) -> Name {
        self.mut_cur_block_codeobj().names.push(name);
        Name::local(self.cur_block_codeobj().names.len() - 1)
    }

    fn register_method(&mut self, name: Str) -> Name {
        self.mut_cur_block_codeobj().names.push(name);
        Name::local(self.cur_block_codeobj().names.len() - 1)
    }

    fn emit_load_name_instr(&mut self, ident: Identifier) {
        log!(info "entered {}", fn_name!());
        let escaped = escape_name(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped));
        let instr = match name.kind {
            StoreLoadKind::Fast | StoreLoadKind::FastConst => LOAD_FAST,
            StoreLoadKind::Global | StoreLoadKind::GlobalConst => LOAD_GLOBAL,
            StoreLoadKind::Deref | StoreLoadKind::DerefConst => LOAD_DEREF,
            StoreLoadKind::Local | StoreLoadKind::LocalConst => LOAD_NAME,
        };
        self.write_instr(instr);
        self.write_arg(name.idx as u8);
        self.stack_inc();
    }

    fn emit_import_name_instr(&mut self, ident: Identifier, items_len: usize) {
        log!(info "entered {}", fn_name!());
        let escaped = escape_name(ident);
        let name = self
            .local_search(&escaped, Name)
            .unwrap_or_else(|| self.register_name(escaped));
        self.write_instr(IMPORT_NAME);
        self.write_arg(name.idx as u8);
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
        self.write_arg(name.idx as u8);
        // self.stack_inc(); (module object) -> attribute
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

    fn emit_load_attr_instr(
        &mut self,
        class: &str,
        uniq_obj_name: Option<&str>,
        ident: Identifier,
    ) {
        log!(info "entered {} ({class}{ident})", fn_name!());
        let escaped = escape_attr(class, uniq_obj_name, ident);
        let name = self
            .local_search(&escaped, Attr)
            .unwrap_or_else(|| self.register_attr(escaped));
        let instr = match name.kind {
            StoreLoadKind::Fast | StoreLoadKind::FastConst => LOAD_FAST,
            StoreLoadKind::Global | StoreLoadKind::GlobalConst => LOAD_GLOBAL,
            StoreLoadKind::Deref | StoreLoadKind::DerefConst => LOAD_DEREF,
            StoreLoadKind::Local | StoreLoadKind::LocalConst => LOAD_ATTR,
        };
        self.write_instr(instr);
        self.write_arg(name.idx as u8);
    }

    fn emit_load_method_instr(
        &mut self,
        class: &str,
        uniq_obj_name: Option<&str>,
        ident: Identifier,
    ) {
        log!(info "entered {} ({class}{ident})", fn_name!());
        let escaped = escape_attr(class, uniq_obj_name, ident);
        let name = self
            .local_search(&escaped, Method)
            .unwrap_or_else(|| self.register_method(escaped));
        let instr = match name.kind {
            StoreLoadKind::Fast | StoreLoadKind::FastConst => LOAD_FAST,
            StoreLoadKind::Global | StoreLoadKind::GlobalConst => LOAD_GLOBAL,
            StoreLoadKind::Deref | StoreLoadKind::DerefConst => LOAD_DEREF,
            StoreLoadKind::Local | StoreLoadKind::LocalConst => LOAD_METHOD,
        };
        self.write_instr(instr);
        self.write_arg(name.idx as u8);
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
        let instr = match name.kind {
            StoreLoadKind::Fast => STORE_FAST,
            StoreLoadKind::FastConst => ERG_STORE_FAST_IMMUT,
            // NOTE: First-time variables are treated as GLOBAL, but they are always first-time variables when assigned, so they are just NAME
            // NOTE: 初見の変数はGLOBAL扱いになるが、代入時は必ず初見であるので単なるNAME
            StoreLoadKind::Global | StoreLoadKind::GlobalConst => STORE_NAME,
            StoreLoadKind::Deref | StoreLoadKind::DerefConst => STORE_DEREF,
            StoreLoadKind::Local | StoreLoadKind::LocalConst => {
                match acc_kind {
                    AccessKind::Name => STORE_NAME,
                    AccessKind::Attr => STORE_ATTR,
                    // cannot overwrite methods directly
                    AccessKind::Method => STORE_ATTR,
                }
            }
        };
        self.write_instr(instr);
        self.write_arg(name.idx as u8);
        self.stack_dec();
        if instr == Opcode::STORE_ATTR {
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
        self.write_arg(name.idx as u8);
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
            acc => todo!("store: {acc}"),
        }
    }

    fn emit_pop_top(&mut self) {
        self.write_instr(Opcode::POP_TOP);
        self.write_arg(0u8);
        self.stack_dec();
    }

    fn cancel_pop_top(&mut self) {
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
    fn crash(&mut self, description: &'static str) -> ! {
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
                self.emit_load_name_instr(ident);
            }
            Accessor::Attr(a) => {
                let class = a.obj.ref_t().name();
                let uniq_obj_name = a.obj.__name__().map(Str::rc);
                // C = Class ...
                // C.
                //     a = C.x
                // ↓
                // class C:
                //     a = x
                if Some(&self.cur_block_codeobj().name[..]) == a.obj.__name__()
                    && &self.cur_block_codeobj().name[..] != "<module>"
                {
                    self.emit_load_name_instr(a.ident);
                } else {
                    self.emit_expr(*a.obj);
                    self.emit_load_attr_instr(
                        &class,
                        uniq_obj_name.as_ref().map(|s| &s[..]),
                        a.ident,
                    );
                }
            }
            Accessor::TupleAttr(t_attr) => {
                self.emit_expr(*t_attr.obj);
                self.emit_load_const(t_attr.index.value);
                self.write_instr(BINARY_SUBSCR);
                self.write_arg(0);
                self.stack_dec();
            }
            Accessor::Subscr(subscr) => {
                self.emit_expr(*subscr.obj);
                self.emit_expr(*subscr.index);
                self.write_instr(BINARY_SUBSCR);
                self.write_arg(0);
                self.stack_dec();
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

    fn emit_trait_def(&mut self, def: Def) {
        if !self.abc_loaded {
            self.load_abc();
            self.abc_loaded = true;
        }
        self.write_instr(LOAD_BUILD_CLASS);
        self.write_arg(0);
        self.stack_inc();
        let code = self.emit_trait_block(def.def_kind(), &def.sig, def.body.block);
        self.emit_load_const(code);
        self.emit_load_const(def.sig.ident().inspect().clone());
        self.write_instr(MAKE_FUNCTION);
        self.write_arg(0);
        self.emit_load_const(def.sig.ident().inspect().clone());
        self.emit_load_name_instr(Identifier::private(Str::ever("#ABCMeta")));
        self.emit_load_const(vec![ValueObj::from("metaclass")]);
        let subclasses_len = 1;
        self.write_instr(Opcode::CALL_FUNCTION_KW);
        self.write_arg(2 + subclasses_len as u8);
        self.stack_dec_n((1 + 2 + 1 + subclasses_len) - 1);
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
        self.units.push(CodeGenUnit::new(
            self.unit_size,
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
                Some(Identifier::private(Str::ever("#abstractmethod"))),
            );
        }
        self.emit_load_const(ValueObj::None);
        self.write_instr(RETURN_VALUE);
        self.write_arg(0u8);
        if self.cur_block().stack_len > 1 {
            let block_id = self.cur_block().id;
            let stack_len = self.cur_block().stack_len;
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
        let deco_is_some = deco.is_some();
        if let Some(deco) = deco {
            self.emit_load_name_instr(deco);
        }
        let code = {
            self.unit_size += 1;
            self.units.push(CodeGenUnit::new(
                self.unit_size,
                vec![],
                Str::rc(self.cfg.input.enclosed_name()),
                ident.inspect(),
                ident.ln_begin().unwrap(),
            ));
            self.emit_load_const(ValueObj::None);
            self.write_instr(RETURN_VALUE);
            self.write_arg(0u8);
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
        if let Some(class) = class_name {
            self.emit_load_const(Str::from(format!("{class}.{}", ident.name.inspect())));
        } else {
            self.emit_load_const(ident.name.inspect().clone());
        }
        self.write_instr(MAKE_FUNCTION);
        self.write_arg(0);
        if deco_is_some {
            self.write_instr(CALL_FUNCTION);
            self.write_arg(1);
            self.stack_dec();
        }
        // stack_dec: (<abstractmethod>) + <code obj> + <name> -> <function>
        self.stack_dec();
        self.emit_store_instr(ident, Name);
    }

    fn emit_class_def(&mut self, class_def: ClassDef) {
        log!(info "entered {} ({})", fn_name!(), class_def.sig);
        let ident = class_def.sig.ident().clone();
        let kind = class_def.kind;
        let require_or_sup = class_def.require_or_sup.clone();
        self.write_instr(LOAD_BUILD_CLASS);
        self.write_arg(0);
        self.stack_inc();
        let code = self.emit_class_block(class_def);
        self.emit_load_const(code);
        self.emit_load_const(ident.inspect().clone());
        self.write_instr(MAKE_FUNCTION);
        self.write_arg(0);
        self.emit_load_const(ident.inspect().clone());
        // LOAD subclasses
        let subclasses_len = self.emit_require_type(kind, *require_or_sup);
        self.write_instr(CALL_FUNCTION);
        self.write_arg(2 + subclasses_len as u8);
        self.stack_dec_n((1 + 2 + subclasses_len) - 1);
        self.emit_store_instr(ident, Name);
        self.stack_dec();
    }

    // NOTE: use `TypeVar`, `Generic` in `typing` module
    // fn emit_poly_type_def(&mut self, sig: SubrSignature, body: DefBody) {}

    /// Y = Inherit X => class Y(X): ...
    fn emit_require_type(&mut self, kind: TypeKind, require_or_sup: Expr) -> usize {
        log!(info "entered {} ({kind:?}, {require_or_sup})", fn_name!());
        match kind {
            TypeKind::Class => 0,
            TypeKind::Subclass => {
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
        let mut make_function_flag = 0u8;
        let params = self.gen_param_names(&sig.params);
        let code = self.emit_block(body.block, Some(name.clone()), params);
        if !self.cur_block_codeobj().cellvars.is_empty() {
            let cellvars_len = self.cur_block_codeobj().cellvars.len() as u8;
            for i in 0..cellvars_len {
                self.write_instr(LOAD_CLOSURE);
                self.write_arg(i);
            }
            self.write_instr(BUILD_TUPLE);
            self.write_arg(cellvars_len);
            make_function_flag += 8;
        }
        self.emit_load_const(code);
        if let Some(class) = class_name {
            self.emit_load_const(Str::from(format!("{class}.{name}")));
        } else {
            self.emit_load_const(name);
        }
        self.write_instr(MAKE_FUNCTION);
        self.write_arg(make_function_flag);
        // stack_dec: <code obj> + <name> -> <function>
        self.stack_dec();
        self.emit_store_instr(sig.ident, Name);
    }

    fn emit_lambda(&mut self, lambda: Lambda) {
        log!(info "entered {} ({lambda})", fn_name!());
        let mut make_function_flag = 0u8;
        let params = self.gen_param_names(&lambda.params);
        let code = self.emit_block(lambda.body, Some("<lambda>".into()), params);
        if !self.cur_block_codeobj().cellvars.is_empty() {
            let cellvars_len = self.cur_block_codeobj().cellvars.len() as u8;
            for i in 0..cellvars_len {
                self.write_instr(LOAD_CLOSURE);
                self.write_arg(i);
            }
            self.write_instr(BUILD_TUPLE);
            self.write_arg(cellvars_len);
            make_function_flag += 8;
        }
        self.emit_load_const(code);
        self.emit_load_const("<lambda>");
        self.write_instr(MAKE_FUNCTION);
        self.write_arg(make_function_flag);
        // stack_dec: <lambda code obj> + <name "<lambda>"> -> <function>
        self.stack_dec();
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
                    "",
                    AtomicStr::from(unary.op.content),
                )
                .write_to_stderr();
                NOT_IMPLEMENTED
            }
        };
        self.write_instr(instr);
        self.write_arg(tycode as u8);
    }

    fn emit_binop(&mut self, bin: BinOp) {
        log!(info "entered {} ({bin})", fn_name!());
        // TODO: and/orのプリミティブ命令の実装
        // Range operators are not operators in Python
        match &bin.op.kind {
            // l..<r == range(l, r)
            TokenKind::RightOpen => {
                self.emit_load_name_instr(Identifier::public("range"));
            }
            TokenKind::LeftOpen | TokenKind::Closed | TokenKind::Open => todo!(),
            _ => {}
        }
        let type_pair = TypePair::new(bin.lhs_t(), bin.rhs_t());
        self.emit_expr(*bin.lhs);
        self.emit_expr(*bin.rhs);
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
            TokenKind::LeftOpen | TokenKind::RightOpen | TokenKind::Closed | TokenKind::Open => {
                CALL_FUNCTION
            } // ERG_BINARY_RANGE,
            _ => {
                CompileError::feature_error(
                    self.cfg.input.clone(),
                    bin.op.loc(),
                    "",
                    AtomicStr::from(bin.op.content),
                )
                .write_to_stderr();
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
            TokenKind::LeftOpen | TokenKind::RightOpen | TokenKind::Closed | TokenKind::Open => 2,
            _ => type_pair as u8,
        };
        self.write_instr(instr);
        self.write_arg(arg);
        self.stack_dec();
        match &bin.op.kind {
            TokenKind::LeftOpen | TokenKind::RightOpen | TokenKind::Open | TokenKind::Closed => {
                self.stack_dec();
            }
            _ => {}
        }
    }

    fn emit_discard_instr(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        while let Some(arg) = args.try_remove(0) {
            self.emit_expr(arg);
            self.emit_pop_top();
        }
    }

    fn emit_if_instr(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        let cond = args.remove(0);
        self.emit_expr(cond);
        let idx_pop_jump_if_false = self.cur_block().lasti;
        self.write_instr(POP_JUMP_IF_FALSE);
        // cannot detect where to jump to at this moment, so put as 0
        self.write_arg(0_u8);
        match args.remove(0) {
            // then block
            Expr::Lambda(lambda) => {
                let params = self.gen_param_names(&lambda.params);
                self.emit_frameless_block(lambda.body, params);
            }
            other => {
                self.emit_expr(other);
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
                    self.emit_frameless_block(lambda.body, params);
                }
                other => {
                    self.emit_expr(other);
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
    }

    fn emit_for_instr(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        let iterable = args.remove(0);
        self.emit_expr(iterable);
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
        self.emit_frameless_block(lambda.body, params); // ここでPOPされる
        self.write_instr(JUMP_ABSOLUTE);
        self.write_arg((idx_for_iter / 2) as u8);
        let idx_end = self.cur_block().lasti;
        self.edit_code(idx_for_iter + 1, (idx_end - idx_for_iter - 2) / 2);
        self.emit_load_const(ValueObj::None);
    }

    fn emit_match_instr(&mut self, mut args: Args, _use_erg_specific: bool) {
        log!(info "entered {}", fn_name!());
        let expr = args.remove(0);
        self.emit_expr(expr);
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
            let pop_jump_points = self.emit_match_pattern(pat);
            self.emit_frameless_block(lambda.body, Vec::new());
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
    }

    fn emit_match_pattern(&mut self, pat: ParamPattern) -> Vec<usize> {
        log!(info "entered {}", fn_name!());
        let mut pop_jump_points = vec![];
        match pat {
            ParamPattern::VarName(name) => {
                let ident = Identifier::bare(None, name);
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
                    pop_jump_points.append(&mut self.emit_match_pattern(elem.pat));
                }
                if !arr.elems.defaults.is_empty() {
                    todo!("default values in match are not supported yet")
                }
            }
            _other => {
                todo!()
            }
        }
        pop_jump_points
    }

    fn emit_with_instr(&mut self, args: Args) {
        log!(info "entered {}", fn_name!());
        let mut args = args;
        let expr = args.remove(0);
        let lambda = enum_unwrap!(args.remove(0), Expr::Lambda);
        let params = self.gen_param_names(&lambda.params);
        self.emit_expr(expr);
        let idx_setup_with = self.cur_block().lasti;
        self.write_instr(SETUP_WITH);
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
        self.write_instr(DUP_TOP);
        self.write_arg(0);
        self.stack_inc();
        self.write_instr(DUP_TOP);
        self.write_arg(0);
        self.stack_inc();
        self.write_instr(CALL_FUNCTION);
        self.write_arg(3);
        self.stack_dec_n((1 + 3) - 1);
        self.emit_pop_top();
        let idx_jump_forward = self.cur_block().lasti;
        self.write_instr(JUMP_FORWARD);
        self.write_arg(0);
        self.edit_code(
            idx_setup_with + 1,
            (self.cur_block().lasti - idx_setup_with - 2) / 2,
        );
        self.write_instr(WITH_EXCEPT_START);
        self.write_arg(0);
        let idx_pop_jump_if_true = self.cur_block().lasti;
        self.write_instr(POP_JUMP_IF_TRUE);
        self.write_arg(0);
        self.write_instr(RERAISE);
        self.write_arg(1);
        self.edit_code(idx_pop_jump_if_true + 1, self.cur_block().lasti / 2);
        // self.emit_pop_top();
        // self.emit_pop_top();
        self.emit_pop_top();
        self.write_instr(POP_EXCEPT);
        self.write_arg(0);
        let idx_end = self.cur_block().lasti;
        self.edit_code(idx_jump_forward + 1, (idx_end - idx_jump_forward - 2) / 2);
        self.emit_load_name_instr(stash);
    }

    fn emit_call(&mut self, call: Call) {
        log!(info "entered {} ({call})", fn_name!());
        if let Some(method_name) = call.method_name {
            self.emit_call_method(*call.obj, method_name, call.args);
        } else {
            match *call.obj {
                Expr::Accessor(Accessor::Ident(ident)) if ident.vis().is_private() => {
                    self.emit_call_local(ident, call.args)
                }
                other => {
                    self.emit_expr(other);
                    self.emit_args(call.args, Name);
                }
            }
        }
    }

    fn emit_call_local(&mut self, local: Identifier, args: Args) {
        log!(info "entered {}", fn_name!());
        match &local.inspect()[..] {
            "assert" => self.emit_assert_instr(args),
            "discard" => self.emit_discard_instr(args),
            "for" | "for!" => self.emit_for_instr(args),
            "if" | "if!" => self.emit_if_instr(args),
            "match" | "match!" => self.emit_match_instr(args, true),
            "with!" => self.emit_with_instr(args),
            "import" => {
                if !self.module_type_loaded {
                    self.load_module_type();
                    self.module_type_loaded = true;
                }
                self.emit_load_name_instr(local);
                self.emit_args(args, Name);
            }
            _ => {
                self.emit_load_name_instr(local);
                self.emit_args(args, Name);
            }
        }
    }

    fn emit_call_method(&mut self, obj: Expr, method_name: Identifier, args: Args) {
        log!(info "entered {}", fn_name!());
        let class = obj.ref_t().name(); // これは必ずmethodのあるクラスになっている
        let uniq_obj_name = obj.__name__().map(Str::rc);
        if &method_name.inspect()[..] == "update!" {
            return self.emit_call_update(obj, args);
        } else if is_fake_method(&class, method_name.inspect()) {
            return self.emit_call_fake_method(obj, method_name, args);
        }
        self.emit_expr(obj);
        self.emit_load_method_instr(&class, uniq_obj_name.as_ref().map(|s| &s[..]), method_name);
        self.emit_args(args, Method);
    }

    fn emit_args(&mut self, mut args: Args, kind: AccessKind) {
        let argc = args.len();
        let pos_len = args.pos_args.len();
        let mut kws = Vec::with_capacity(args.kw_len());
        while let Some(arg) = args.try_remove_pos(0) {
            self.emit_expr(arg.expr);
        }
        if let Some(var_args) = &args.var_args {
            if pos_len > 0 {
                self.write_instr(BUILD_LIST);
                self.write_arg(pos_len as u8);
            }
            self.emit_expr(var_args.expr.clone());
            if pos_len > 0 {
                self.write_instr(LIST_EXTEND);
                self.write_arg(1);
                self.write_instr(LIST_TO_TUPLE);
                self.write_arg(0);
            }
        }
        while let Some(arg) = args.try_remove_kw(0) {
            kws.push(ValueObj::Str(arg.keyword.content.clone()));
            self.emit_expr(arg.expr);
        }
        let kwsc = if !kws.is_empty() {
            let kws_tuple = ValueObj::from(kws);
            self.emit_load_const(kws_tuple);
            self.write_instr(CALL_FUNCTION_KW);
            self.write_arg(argc as u8);
            1
        } else {
            if args.var_args.is_some() {
                self.write_instr(CALL_FUNCTION_EX);
                if kws.is_empty() {
                    self.write_arg(0);
                } else {
                    self.write_arg(1);
                }
            } else {
                if kind.is_method() {
                    self.write_instr(CALL_METHOD);
                } else {
                    self.write_instr(CALL_FUNCTION);
                }
                self.write_arg(argc as u8);
            }
            0
        };
        // (1 (subroutine) + argc + kwsc) input objects -> 1 return object
        self.stack_dec_n((1 + argc + kwsc) - 1);
    }

    /// X.update! x -> x + 1
    /// X = (x -> x + 1)(X)
    /// X = X + 1
    fn emit_call_update(&mut self, obj: Expr, mut args: Args) {
        log!(info "entered {}", fn_name!());
        let acc = enum_unwrap!(obj, Expr::Accessor);
        let func = args.remove_left_or_key("f").unwrap();
        self.emit_expr(func);
        self.emit_acc(acc.clone());
        self.write_instr(CALL_FUNCTION);
        self.write_arg(1);
        // (1 (subroutine) + argc) input objects -> 1 return object
        self.stack_dec_n((1 + 1) - 1);
        self.store_acc(acc);
    }

    /// 1.abs() => abs(1)
    fn emit_call_fake_method(&mut self, obj: Expr, mut method_name: Identifier, mut args: Args) {
        log!(info "entered {}", fn_name!());
        method_name.dot = None;
        self.emit_load_name_instr(method_name);
        args.insert_pos(0, PosArg::new(obj));
        self.emit_args(args, Name);
    }

    // assert takes 1 or 2 arguments (0: cond, 1: message)
    fn emit_assert_instr(&mut self, mut args: Args) {
        log!(info "entered {}", fn_name!());
        self.emit_expr(args.remove(0));
        let pop_jump_point = self.cur_block().lasti;
        self.write_instr(Opcode::POP_JUMP_IF_TRUE);
        self.write_arg(0);
        self.stack_dec();
        self.write_instr(Opcode::LOAD_ASSERTION_ERROR);
        self.write_arg(0);
        if let Some(expr) = args.try_remove(0) {
            self.emit_expr(expr);
            self.write_instr(Opcode::CALL_FUNCTION);
            self.write_arg(1);
        }
        self.write_instr(Opcode::RAISE_VARARGS);
        self.write_arg(1);
        let idx = self.cur_block().lasti;
        self.edit_code(pop_jump_point + 1, idx / 2); // jump to POP_TOP
    }

    #[allow(clippy::identity_op)]
    fn emit_record(&mut self, rec: Record) {
        log!(info "entered {} ({rec})", fn_name!());
        let attrs_len = rec.attrs.len();
        // making record type
        let ident = Identifier::private(Str::ever("#NamedTuple"));
        self.emit_load_name_instr(ident);
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
        self.emit_load_name_instr(ident);
        for field in rec.attrs.into_iter() {
            self.emit_frameless_block(field.body.block, vec![]);
        }
        self.write_instr(CALL_FUNCTION);
        self.write_arg(attrs_len as u8);
        // (1 (subroutine) + argc + kwsc) input objects -> 1 return object
        self.stack_dec_n((1 + attrs_len + 0) - 1);
    }

    fn emit_expr(&mut self, expr: Expr) {
        log!(info "entered {} ({expr})", fn_name!());
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
            // TODO: list comprehension
            Expr::Array(arr) => match arr {
                Array::Normal(mut arr) => {
                    let len = arr.elems.len();
                    while let Some(arg) = arr.elems.try_remove_pos(0) {
                        self.emit_expr(arg.expr);
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
                        self.emit_expr(arg.expr);
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
            Expr::Set(set) => match set {
                crate::hir::Set::Normal(mut set) => {
                    let len = set.elems.len();
                    while let Some(arg) = set.elems.try_remove_pos(0) {
                        self.emit_expr(arg.expr);
                    }
                    self.write_instr(BUILD_SET);
                    self.write_arg(len as u8);
                    if len == 0 {
                        self.stack_inc();
                    } else {
                        self.stack_dec_n(len - 1);
                    }
                }
                crate::hir::Set::WithLength(_) => todo!(),
            },
            Expr::Record(rec) => self.emit_record(rec),
            Expr::Code(code) => {
                let code = self.emit_block(code, None, vec![]);
                self.emit_load_const(code);
            }
            Expr::Compound(chunks) => {
                if !self.module_type_loaded {
                    self.load_module_type();
                    self.module_type_loaded = true;
                }
                self.emit_frameless_block(chunks, vec![]);
            }
            Expr::TypeAsc(tasc) => {
                self.emit_expr(*tasc.expr);
            }
            // Dict,
            other => {
                CompileError::feature_error(self.cfg.input.clone(), other.loc(), "???", "".into())
                    .write_to_stderr();
                self.crash("cannot compile this expression at this time");
            }
        }
    }

    /// forブロックなどで使う
    fn emit_frameless_block(&mut self, block: Block, params: Vec<Str>) {
        log!(info "entered {}", fn_name!());
        let line = block.ln_begin().unwrap_or(0);
        for param in params {
            self.emit_store_instr(
                Identifier::public_with_line(Token::dummy(), param, line),
                Name,
            );
        }
        for expr in block.into_iter() {
            self.emit_expr(expr);
            // TODO: discard
            // 最終的に帳尻を合わせる(コード生成の順番的にスタックの整合性が一時的に崩れる場合がある)
            if self.cur_block().stack_len == 1 {
                self.emit_pop_top();
            }
        }
        self.cancel_pop_top();
    }

    fn emit_with_block(&mut self, block: Block, params: Vec<Str>) {
        log!(info "entered {}", fn_name!());
        let line = block.ln_begin().unwrap_or(0);
        for param in params {
            self.emit_store_instr(
                Identifier::public_with_line(Token::dummy(), param, line),
                Name,
            );
        }
        for expr in block.into_iter() {
            self.emit_expr(expr);
            if self.cur_block().stack_len != 0 {
                self.emit_pop_top();
            }
        }
        self.cancel_pop_top();
    }

    fn emit_class_block(&mut self, class: ClassDef) -> CodeObj {
        log!(info "entered {}", fn_name!());
        let name = class.sig.ident().inspect().clone();
        self.unit_size += 1;
        let firstlineno = match class.methods.get(0).and_then(|def| def.ln_begin()) {
            Some(l) => l,
            None => class.sig.ln_begin().unwrap(),
        };
        self.units.push(CodeGenUnit::new(
            self.unit_size,
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
        self.emit_init_method(&class.sig, class.__new__.clone());
        if class.need_to_gen_new {
            self.emit_new_func(&class.sig, class.__new__);
        }
        if !class.methods.is_empty() {
            self.emit_frameless_block(class.methods, vec![]);
        }
        if self.cur_block().stack_len == 0 {
            self.emit_load_const(ValueObj::None);
        }
        self.write_instr(RETURN_VALUE);
        self.write_arg(0u8);
        if self.cur_block().stack_len > 1 {
            let block_id = self.cur_block().id;
            let stack_len = self.cur_block().stack_len;
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
        let ident = Identifier::public_with_line(Token::dummy(), Str::ever("__init__"), line);
        let param_name = fresh_varname();
        let param = VarName::from_str_and_line(Str::from(param_name.clone()), line);
        let param = ParamSignature::new(ParamPattern::VarName(param), None, None);
        let self_param = VarName::from_str_and_line(Str::ever("self"), line);
        let self_param = ParamSignature::new(ParamPattern::VarName(self_param), None, None);
        let params = Params::new(vec![self_param, param], None, vec![], None);
        let subr_sig = SubrSignature::new(ident, params, __new__.clone());
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
                    let expr = Expr::Accessor(Accessor::Attr(Attribute::new(
                        obj,
                        Identifier::bare(
                            Some(Token::dummy()),
                            VarName::from_str(field.symbol.clone()),
                        ),
                        Type::Failure,
                    )));
                    let obj = Expr::Accessor(Accessor::private_with_line(Str::ever("self"), line));
                    let dot = if field.vis.is_private() {
                        None
                    } else {
                        Some(Token::dummy())
                    };
                    let attr = Accessor::Attr(Attribute::new(
                        obj,
                        Identifier::bare(dot, VarName::from_str(field.symbol.clone())),
                        Type::Failure,
                    ));
                    let attr_def = AttrDef::new(attr, Block::new(vec![expr]));
                    attrs.push(Expr::AttrDef(attr_def));
                }
                let none = Token::new(TokenKind::NoneLit, "None", line, 0);
                attrs.push(Expr::Lit(Literal::from(none)));
            }
            other => todo!("{other}"),
        }
        let block = Block::new(attrs);
        let body = DefBody::new(Token::dummy(), block, DefId(0));
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
        let ident = Identifier::public_with_line(Token::dummy(), Str::ever("new"), line);
        let param_name = fresh_varname();
        let param = VarName::from_str_and_line(Str::from(param_name.clone()), line);
        let param = ParamSignature::new(ParamPattern::VarName(param), None, None);
        let sig = SubrSignature::new(ident, Params::new(vec![param], None, vec![], None), __new__);
        let arg = PosArg::new(Expr::Accessor(Accessor::private_with_line(
            Str::from(param_name),
            line,
        )));
        let class = Expr::Accessor(Accessor::Ident(class_ident.clone()));
        let class_new = Expr::Accessor(Accessor::attr(
            class,
            Identifier::bare(None, VarName::from_str_and_line(Str::ever("__new__"), line)),
            Type::Failure,
        ));
        let call = Expr::Call(Call::new(
            class_new,
            None,
            Args::new(vec![arg], None, vec![], None),
            Type::Failure,
        ));
        let block = Block::new(vec![call]);
        let body = DefBody::new(Token::dummy(), block, DefId(0));
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
        self.units.push(CodeGenUnit::new(
            self.unit_size,
            params,
            Str::rc(self.cfg.input.enclosed_name()),
            &name,
            firstlineno,
        ));
        for expr in block.into_iter() {
            self.emit_expr(expr);
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
        self.write_arg(0u8);
        // flagging
        if !self.cur_block_codeobj().varnames.is_empty() {
            self.mut_cur_block_codeobj().flags += CodeObjFlags::NewLocals as u32;
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
        self.record_type_loaded = true;
    }

    fn load_record_type(&mut self) {
        self.emit_global_import_items(
            Identifier::public("collections"),
            vec![(
                Identifier::public("namedtuple"),
                Some(Identifier::private(Str::ever("#NamedTuple"))),
            )],
        );
    }

    fn load_abc(&mut self) {
        self.emit_global_import_items(
            Identifier::public("abc"),
            vec![
                (
                    Identifier::public("ABCMeta"),
                    Some(Identifier::private(Str::ever("#ABCMeta"))),
                ),
                (
                    Identifier::public("abstractmethod"),
                    Some(Identifier::private(Str::ever("#abstractmethod"))),
                ),
            ],
        );
    }

    fn load_module_type(&mut self) {
        self.emit_global_import_items(
            Identifier::public("types"),
            vec![(
                Identifier::public("ModuleType"),
                Some(Identifier::private(Str::ever("#ModuleType"))),
            )],
        );
    }

    pub fn emit(&mut self, hir: HIR) -> CodeObj {
        log!(info "the code-generating process has started.{RESET}");
        self.unit_size += 1;
        self.units.push(CodeGenUnit::new(
            self.unit_size,
            vec![],
            Str::rc(self.cfg.input.enclosed_name()),
            "<module>",
            1,
        ));
        if !self.prelude_loaded {
            self.load_prelude();
            self.prelude_loaded = true;
        }
        let mut print_point = 0;
        if self.input().is_repl() {
            print_point = self.cur_block().lasti;
            self.emit_load_name_instr(Identifier::public("print"));
            // Consistency will be taken later (when NOP replacing)
            // 後で(NOP書き換え時)整合性を取る
            self.stack_dec();
        }
        for expr in hir.module.into_iter() {
            self.emit_expr(expr);
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
            CompileError::stack_bug(
                self.input().clone(),
                Location::Unknown,
                stack_len,
                block_id,
                fn_name_full!(),
            )
            .write_to_stderr();
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
