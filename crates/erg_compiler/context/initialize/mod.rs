//! defines type information for builtin objects (in `Context`)
//!
//! 組み込みオブジェクトの型情報を(Contextに)定義
#![allow(non_snake_case)]

mod classes;
pub mod const_func;
mod funcs;
mod patches;
mod procs;
mod traits;

use std::path::PathBuf;

use erg_common::config::ErgConfig;
use erg_common::consts::{DEBUG_MODE, ERG_MODE, PYTHON_MODE};
use erg_common::dict;
use erg_common::error::Location;
use erg_common::fresh::fresh_varname;
#[allow(unused_imports)]
use erg_common::log;
use erg_common::Str;
use erg_common::{set, unique_in_place};

use erg_parser::ast::VarName;

use crate::context::initialize::const_func::*;
use crate::context::instantiate_spec::ConstTemplate;
use crate::context::{
    ClassDefType, Context, ContextKind, MethodPair, ModuleContext, ParamSpec, TraitImpl,
};
use crate::module::SharedCompilerResource;
use crate::ty::constructors::*;
use crate::ty::free::Constraint;
use crate::ty::value::ValueObj;
use crate::ty::{BuiltinConstSubr, ConstSubr, ParamTy, Predicate, TyParam, Type, Visibility};
use crate::varinfo::{AbsLocation, Mutability, VarInfo, VarKind};
use Mutability::*;
use ParamSpec as PS;
use Type::*;
use VarKind::*;

const NUM: &str = "Num";

const UNPACK: &str = "Unpack";
const INHERITABLE_TYPE: &str = "InheritableType";
const NAMED: &str = "Named";
const SIZED: &str = "Sized";
const MUTABLE: &str = "Mutable";
const SELF: &str = "Self";
const IMMUTIZABLE: &str = "Immutizable";
const IMMUT_TYPE: &str = "ImmutType";
const PROC_UPDATE: &str = "update!";
const MUTIZABLE: &str = "Mutizable";
const MUTABLE_MUT_TYPE: &str = "MutType!";
const PATH_LIKE: &str = "PathLike";
const MUTABLE_READABLE: &str = "Readable!";
const FUNC_READ: &str = "read";
const PROC_READ: &str = "read!";
const FUNC_READABLE: &str = "readable";
const PROC_READLINE: &str = "readline!";
const FUNC_READLINE: &str = "readline";
const FUNC_READLINES: &str = "readlines";
const PROC_READLINES: &str = "readlines!";
const MUTABLE_IO: &str = "IO!";
const FUNC_MODE: &str = "mode";
const FUNC_NAME: &str = "name";
const FUNC_CLOSE: &str = "close";
const PROC_CLOSE: &str = "close!";
const FUNC_CLOSED: &str = "closed";
const FUNC_FILENO: &str = "fileno";
const PROC_FLUSH: &str = "flush!";
const FUNC_FLUSH: &str = "flush";
const FUNC_ISATTY: &str = "isatty";
const FUNC_SEEK: &str = "seek";
const PROC_SEEK: &str = "seek!";
const FUNC_SEEKABLE: &str = "seekable";
const FUNC_TELL: &str = "tell";
const MUTABLE_WRITABLE: &str = "Writable!";
const WRITE: &str = "Write";
const FUNC_WRITE: &str = "write";
const PROC_WRITE: &str = "write!";
const FILE_LIKE: &str = "FileLike";
const MUTABLE_FILE_LIKE: &str = "FileLike!";
const SHOW: &str = "Show";
const INPUT: &str = "Input";
const OUTPUT: &str = "Output";
const POW_OUTPUT: &str = "PowOutput";
const MOD_OUTPUT: &str = "ModOutput";
const IN: &str = "In";
const CONTAINER: &str = "Container";
const COLLECTION: &str = "Collection";
const INDEXABLE: &str = "Indexable";
const MAPPING: &str = "Mapping";
const MUTABLE_MAPPING: &str = "Mapping!";
const EQ: &str = "Eq";
const ORD: &str = "Ord";
const TO_STR: &str = "to_str";
const ORDERING: &str = "Ordering";
const SEQUENCE: &str = "Sequence";
const MUTABLE_SEQUENCE: &str = "Sequence!";
const FUNC_LEN: &str = "len";
const FUNC_GET: &str = "get";
const ITERABLE: &str = "Iterable";
const ITERATOR: &str = "Iterator";
const STR_ITERATOR: &str = "StrIterator";
const FUNC_ITER: &str = "iter";
const ITER: &str = "Iter";
const CONTEXT_MANAGER: &str = "ContextManager";
const EXC_TYPE: &str = "exc_type";
const EXC_VALUE: &str = "exc_value";
const TRACEBACK: &str = "traceback";
const ADD: &str = "Add";
const SUB: &str = "Sub";
const MUL: &str = "Mul";
const DIV: &str = "Div";
const FLOOR_DIV: &str = "FloorDiv";
const POS: &str = "Pos";
const NEG: &str = "Neg";
const NEVER: &str = "Never";
const OBJ: &str = "Obj";
const MUTABLE_OBJ: &str = "Obj!";
const FUNC_CLONE: &str = "clone";
const BYTES: &str = "Bytes";
const BYTEARRAY: &str = "ByteArray!";
const FLOAT: &str = "Float";
const MUT_FLOAT: &str = "Float!";
const EPSILON: &str = "EPSILON";
const REAL: &str = "real";
const IMAG: &str = "imag";
const FUNC_AS_INTEGER_RATIO: &str = "as_integer_ratio";
const FUNC_CONJUGATE: &str = "conjugate";
const FUNC_IS_INTEGER: &str = "is_integer";
const FUNC_HEX: &str = "hex";
const FUNC_FROMHEX: &str = "fromhex";
const COMPLEX: &str = "Complex";
const INT: &str = "Int";
const MUT_INT: &str = "Int!";
const RATIO: &str = "Ratio";
const MUT_RATIO: &str = "Raltio!";
const FUNC_ABS: &str = "abs";
const FUNC_SUCC: &str = "succ";
const FUNC_PRED: &str = "pred";
const FUNC_BIT_LENGTH: &str = "bit_length";
const FUNC_BIT_COUNT: &str = "bit_count";
const FUNC_BYTEORDER: &str = "byteorder";
const TOKEN_BIG_ENDIAN: &str = "big";
const TOKEN_LITTLE_ENDIAN: &str = "little";
const FUNC_FROM_BYTES: &str = "from_bytes";
const FUNC_TO_BYTES: &str = "to_bytes";
const NAT: &str = "Nat";
const MUT_NAT: &str = "Nat!";
const PROC_TIMES: &str = "times!";
const FUNC_TIMES: &str = "times";
const BOOL: &str = "Bool";
const MUT_BOOL: &str = "Bool!";
const STR: &str = "Str";
const MUT_STR: &str = "Str!";
const FUNC_REPLACE: &str = "replace";
const FUNC_ENCODE: &str = "encode";
const FUNC_FORMAT: &str = "format";
const FUNC_LOWER: &str = "lower";
const FUNC_UPPER: &str = "upper";
const FUNC_TO_INT: &str = "to_int";
const FUNC_STARTSWITH: &str = "startswith";
const FUNC_ENDSWITH: &str = "endswith";
const FUNC_CAPITALIZE: &str = "capitalize";
const FUNC_CONTAINS: &str = "contains";
const FUNC_SPLIT: &str = "split";
const FUNC_SPLITLINES: &str = "splitlines";
const FUNC_JOIN: &str = "join";
const FUNC_FIND: &str = "find";
const FUNC_RFIND: &str = "rfind";
const FUNC_INDEX: &str = "index";
const FUNC_RINDEX: &str = "rindex";
const FUNC_COUNT: &str = "count";
const NONE_TYPE: &str = "NoneType";
const TYPE: &str = "Type";
const CLASS: &str = "Class";
const CLASS_TYPE: &str = "ClassType";
const TRAIT: &str = "Trait";
const TRAIT_TYPE: &str = "TraitType";
const CODE: &str = "Code";
const FUNC_MRO: &str = "mro";
const FUNC_CO_ARGCOUNT: &str = "co_argcount";
const FUNC_CO_VARNAMES: &str = "co_varnames";
const FUNC_CO_CONSTS: &str = "co_consts";
const FUNC_CO_NAMES: &str = "co_names";
const FUNC_CO_FREEVARS: &str = "co_freevars";
const FUNC_CO_CELLVARS: &str = "co_cellvars";
const FUNC_CO_FILENAME: &str = "co_filename";
const FUNC_CO_NAME: &str = "co_name";
const FUNC_CO_FIRSTLINENO: &str = "co_firstlineno";
const FUNC_CO_STACKSIZE: &str = "co_stacksize";
const FUNC_CO_FLAGS: &str = "co_flags";
const FUNC_CO_CODE: &str = "co_code";
const FUNC_CO_LNOTAB: &str = "co_lnotab";
const FUNC_CO_NLOCALS: &str = "co_nlocals";
const FUNC_CO_KWONLYARGCOUNT: &str = "co_kwonlyargcount";
const FUNC_CO_POSONLYARGCOUNT: &str = "co_posonlyargcount";
const FUNC_MODULE: &str = "module";
const FUNC_GLOBAL: &str = "global";
const GENERIC_MODULE: &str = "GenericModule";
const PATH: &str = "Path";
const MODULE: &str = "Module";
const PY_MODULE: &str = "PyModule";
const GENERIC_ARRAY: &str = "GenericArray";
const ARRAY: &str = "Array";
const MUT_ARRAY: &str = "Array!";
const FUNC_PARTITION: &str = "partition";
const FUNC_DEDUP: &str = "dedup";
const FUNC_CONCAT: &str = "concat";
const FUNC_PUSH: &str = "push";
const PROC_PUSH: &str = "push!";
const ARRAY_ITERATOR: &str = "ArrayIterator";
const GENERIC_SET: &str = "GenericSet";
const SET: &str = "Set";
const MUT_SET: &str = "Set!";
const SET_ITERATOR: &str = "SetIterator";
const MUT_DICT: &str = "Dict!";
const GENERIC_DICT: &str = "GenericDict";
const DICT: &str = "Dict";
const FUNC_DECODE: &str = "decode";
const GENERIC_TUPLE: &str = "GenericTuple";
const TUPLE: &str = "Tuple";
const TUPLE_ITERATOR: &str = "TupleIterator";
const RECORD: &str = "Record";
const OR: &str = "Or";
const RANGE_ITERATOR: &str = "RangeIterator";
const ENUMERATE: &str = "Enumerate";
const FILTER: &str = "Filter";
const MAP: &str = "Map";
const REVERSED: &str = "Reversed";
const ZIP: &str = "Zip";
const FROZENSET: &str = "FrozenSet";
const COPY: &str = "copy";
const DIFFERENCE: &str = "difference";
const INTERSECTION: &str = "intersection";
const ISDISJOINT: &str = "isdisjoint";
const ISSUBSET: &str = "issubset";
const ISSUPERSET: &str = "issuperset";
const SYMMETRIC_DIFFERENCE: &str = "symmetric_difference";
const MEMORYVIEW: &str = "MemoryView";
const UNION_FUNC: &str = "union";
const FUNC_INC: &str = "inc";
const PROC_INC: &str = "inc!";
const FUNC_DEC: &str = "dec";
const PROC_DEC: &str = "dec!";
const MUT_FILE: &str = "File!";
const MUT_READABLE: &str = "Readable!";
const MUT_WRITABLE: &str = "Writable!";
const MUT_FILE_LIKE: &str = "FileLike!";
const FUNC_APPEND: &str = "append";
const FUNC_EXTEND: &str = "extend";
const PROC_EXTEND: &str = "extend!";
const FUNC_INSERT: &str = "insert";
const PROC_INSERT: &str = "insert!";
const FUNC_REMOVE: &str = "remove";
const PROC_REMOVE: &str = "remove!";
const FUNC_POP: &str = "pop";
const PROC_POP: &str = "pop!";
const FUNC_CLEAR: &str = "clear";
const PROC_CLEAR: &str = "clear!";
const FUNC_SORT: &str = "sort";
const PROC_SORT: &str = "sort!";
const FUNC_REVERSE: &str = "reverse";
const PROC_REVERSE: &str = "reverse!";
const PROC_STRICT_MAP: &str = "strict_map!";
const FUNC_ADD: &str = "add";
const PROC_ADD: &str = "add!";
const FUNC_INVERT: &str = "invert";
const PROC_INVERT: &str = "invert!";
const RANGE: &str = "Range";
const GENERIC_CALLABLE: &str = "GenericCallable";
const GENERIC_GENERATOR: &str = "GenericGenerator";
const FUNC_RETURN: &str = "return";
const FUNC_YIELD: &str = "yield";
const PROC: &str = "Proc";
const NAMED_PROC: &str = "NamedProc";
const NAMED_FUNC: &str = "NamedFunc";
const FUNC: &str = "Func";
const QUANTIFIED: &str = "Quantified";
const QUANTIFIED_FUNC: &str = "QuantifiedFunc";
const FUNC_OBJECT: &str = "object";
const FUNC_INT: &str = "int";
const FUNC_INT__: &str = "int__";
const FUNC_FLOAT: &str = "float";
const FUNC_BOOL: &str = "bool";
const FUNC_STR: &str = "str";
const FUNC_STR__: &str = "str__";
const FUNC_TYPE: &str = "type";
const CODE_TYPE: &str = "CodeType";
const MODULE_TYPE: &str = "ModuleType";
const FUNC_LIST: &str = "list";
const _FUNC_SET: &str = "set";
const FUNC_DICT: &str = "dict";
const FUNC_TUPLE: &str = "tuple";
const UNION: &str = "Union";
const FUNC_STR_ITERATOR: &str = "str_iterator";
const FUNC_ARRAY_ITERATOR: &str = "array_iterator";
const FUNC_SET_ITERATOR: &str = "set_iterator";
const FUNC_TUPLE_ITERATOR: &str = "tuple_iterator";
const FUNC_ENUMERATE: &str = "enumerate";
const FUNC_FILTER: &str = "filter";
const FUNC_FROZENSET: &str = "frozenset";
const FUNC_MAP: &str = "map";
const FUNC_MEMORYVIEW: &str = "memoryview";
const FUNC_REVERSED: &str = "reversed";
const FUNC_ZIP: &str = "zip";
const FILE: &str = "File";
const CALLABLE: &str = "Callable";
const GENERATOR: &str = "Generator";
const FUNC_RANGE: &str = "range";
const FUNC_ALL: &str = "all";
const FUNC_ANY: &str = "any";
const FUNC_ASCII: &str = "ascii";
const FUNC_ASSERT: &str = "assert";
const FUNC_BIN: &str = "bin";
const FUNC_BYTES: &str = "bytes";
const FUNC_CHR: &str = "chr";
const FUNC_CLASSOF: &str = "classof";
const FUNC_COMPILE: &str = "compile";
const FUNC_EXIT: &str = "exit";
const FUNC_ISINSTANCE: &str = "isinstance";
const FUNC_ISSUBCLASS: &str = "issubclass";
const FUNC_MAX: &str = "max";
const FUNC_MIN: &str = "min";
const FUNC_NOT: &str = "not";
const FUNC_OCT: &str = "oct";
const FUNC_ORD: &str = "ord";
const FUNC_POW: &str = "pow";
const FUNC_QUIT: &str = "quit";
const FUNC_REPR: &str = "repr";
const FUNC_ROUND: &str = "round";
const FUNC_SORTED: &str = "sorted";
const FUNC_SUM: &str = "sum";
const FUNC_IF: &str = "if";
const FUNC_IF__: &str = "if__";
const FUNC_DISCARD: &str = "discard";
const FUNC_DISCARD__: &str = "discard__";
const FUNC_IMPORT: &str = "import";
const FUNC_LOG: &str = "log";
const FUNC_PRINT: &str = "print";
const FUNC_NAT: &str = "nat";
const FUNC_NAT__: &str = "nat__";
const FUNC_PANIC: &str = "panic";
const FUNC_UNREACHABLE: &str = "unreachable";
const SUBSUME: &str = "Subsume";
const INHERIT: &str = "Inherit";
const INHERITABLE: &str = "Inheritable";
const DEL: &str = "Del";
const PATCH: &str = "Patch";
const STRUCTURAL: &str = "Structural";
const KEYS: &str = "keys";
const VALUES: &str = "values";
const ITEMS: &str = "items";
const DICT_KEYS: &str = "DictKeys";
const DICT_VALUES: &str = "DictValues";
const DICT_ITEMS: &str = "DictItems";
const FUNC_DICT_KEYS: &str = "dict_keys";
const FUNC_DICT_VALUES: &str = "dict_values";
const FUNC_DICT_ITEMS: &str = "dict_items";
const FUNC_HASATTR: &str = "hasattr";
const FUNC_GETATTR: &str = "getattr";
const FUNC_SETATTR: &str = "setattr";
const FUNC_DELATTR: &str = "delattr";

const OP_IN: &str = "__in__";
const OP_NOT_IN: &str = "__notin__";
const OP_EQ: &str = "__eq__";
const OP_NE: &str = "__ne__";
const OP_CMP: &str = "__cmp__";
const OP_LT: &str = "__lt__";
const OP_LE: &str = "__le__";
const OP_GT: &str = "__gt__";
const OP_GE: &str = "__ge__";
const OP_ADD: &str = "__add__";
const OP_SUB: &str = "__sub__";
const OP_MUL: &str = "__mul__";
const OP_DIV: &str = "__div__";
const OP_FLOOR_DIV: &str = "__floordiv__";
const OP_ABS: &str = "__abs__";
const OP_PARTIAL_CMP: &str = "__partial_cmp__";
const OP_AND: &str = "__and__";
const OP_OR: &str = "__or__";
const OP_POW: &str = "__pow__";
const OP_MOD: &str = "__mod__";
const OP_IS: &str = "__is__!";
const OP_IS_NOT: &str = "__isnot__!";
const OP_RNG: &str = "__rng__";
const OP_LORNG: &str = "__lorng__";
const OP_RORNG: &str = "__rorng__";
const OP_ORNG: &str = "__orng__";
const OP_MUTATE: &str = "__mutate__";
const OP_POS: &str = "__pos__";
const OP_NEG: &str = "__neg__";

const FUNDAMENTAL_LEN: &str = "__len__";
const FUNDAMENTAL_CONTAINS: &str = "__contains__";
const FUNDAMENTAL_CALL: &str = "__call__";
const FUNDAMENTAL_NAME: &str = "__name__";
const FUNDAMENTAL_STR: &str = "__str__";
const FUNDAMENTAL_HASH: &str = "__hash__";
const FUNDAMENTAL_INT: &str = "__int__";
const FUNDAMENTAL_ITER: &str = "__iter__";
const FUNDAMENTAL_NEXT: &str = "__next__";
const FUNDAMENTAL_MODULE: &str = "__module__";
const FUNDAMENTAL_SIZEOF: &str = "__sizeof__";
const FUNDAMENTAL_REPR: &str = "__repr__";
const FUNDAMENTAL_DICT: &str = "__dict__";
const FUNDAMENTAL_BYTES: &str = "__bytes__";
const FUNDAMENTAL_GETITEM: &str = "__getitem__";
const FUNDAMENTAL_TUPLE_GETITEM: &str = "__Tuple_getitem__";
const FUNDAMENTAL_SETITEM: &str = "__setitem__";
const PROC_FUNDAMENTAL_SETITEM: &str = "__setitem__!";
const PROC_FUNDAMENTAL_DELITEM: &str = "__delitem__!";
const FUNDAMENTAL_IMPORT: &str = "__import__";
const FUNDAMENTAL_ENTER: &str = "__enter__";
const FUNDAMENTAL_EXIT: &str = "__exit__";

const LICENSE: &str = "license";
const CREDITS: &str = "credits";
const COPYRIGHT: &str = "copyright";
const TRUE: &str = "True";
const FALSE: &str = "False";
const NONE: &str = "None";
const NOT_IMPLEMENTED: &str = "NotImplemented";
const ELLIPSIS: &str = "Ellipsis";
const SITEBUILTINS_PRINTER: &str = "_sitebuiltins._Printer";
const PY: &str = "py";
const PYIMPORT: &str = "pyimport";
const PYCOMPILE: &str = "pycompile";

const TY_A: &str = "A";
const TY_B: &str = "B";
const TY_D: &str = "D";
const TY_E: &str = "E";
const TY_T: &str = "T";
const TY_TS: &str = "Ts";
const TY_I: &str = "I";
const TY_P: &str = "P";
const TY_R: &str = "R";
const TY_U: &str = "U";
const TY_L: &str = "L";
const TY_N: &str = "N";
const TY_M: &str = "M";
const TY_O: &str = "O";
const TY_K: &str = "K";
const TY_V: &str = "V";

const KW_OLD: &str = "old";
const KW_B: &str = "b";
const KW_C: &str = "c";
const KW_N: &str = "n";
const KW_S: &str = "s";
const KW_X: &str = "X";
const KW_SELF: &str = "self";
const KW_LENGTH: &str = "length";
const KW_PROC: &str = "proc!";
const KW_PAT: &str = "pat";
const KW_INTO: &str = "into";
const KW_ENCODING: &str = "encoding";
const KW_ERRORS: &str = "errors";
const KW_ARGS: &str = "args";
const KW_IDX: &str = "idx";
const KW_LHS: &str = "lhs";
const KW_RHS: &str = "rhs";
const KW_ELEM: &str = "elem";
const KW_FUNC: &str = "func";
const KW_ITERABLE: &str = "iterable";
const KW_INDEX: &str = "index";
const KW_KEY: &str = "key";
const KW_VALUE: &str = "value";
const KW_KEEPENDS: &str = "keepends";
const KW_OBJECT: &str = "object";
const KW_OBJECTS: &str = "objects";
const KW_TEST: &str = "test";
const KW_MSG: &str = "msg";
const KW_STR: &str = "str";
const KW_I: &str = "i";
const KW_SRC: &str = "src";
const KW_THEN: &str = "then";
const KW_ELSE: &str = "else";
const KW_OBJ: &str = "obj";
const KW_NAME: &str = "name";
const KW_DEFAULT: &str = "default";
const KW_START: &str = "start";
const KW_COND: &str = "cond";
const KW_CLASSINFO: &str = "classinfo";
const KW_SUBCLASS: &str = "subclass";
const KW_SEP: &str = "sep";
const KW_END: &str = "end";
const KW_FILE: &str = "file";
const KW_FLUSH: &str = "flush";
const KW_BASE: &str = "base";
const KW_EXP: &str = "exp";
const KW_FILENAME: &str = "filename";
const KW_MODE: &str = "mode";
const KW_SEQ: &str = "seq";
const KW_NUMBER: &str = "number";
const KW_ITERABLE1: &str = "iterable1";
const KW_ITERABLE2: &str = "iterable2";
const KW_CODE: &str = "code";
const KW_STOP: &str = "stop";
const KW_STEP: &str = "step";
const KW_REQUIREMENT: &str = "Requirement";
const KW_IMPL: &str = "Impl";
const KW_ADDITIONAL: &str = "Additional";
const KW_SUPER: &str = "Super";
const KW_MAXSPLIT: &str = "maxsplit";
const KW_SUB: &str = "sub";
const KW_OFFSET: &str = "offset";
const KW_WHENCE: &str = "whence";

pub fn builtins_path() -> PathBuf {
    erg_common::env::erg_pystd_path().join("builtins.d.er")
}

pub fn std_decl_path() -> PathBuf {
    erg_common::env::erg_std_decl_path()
}

impl Context {
    fn register_builtin_decl(
        &mut self,
        name: &'static str,
        t: Type,
        vis: Visibility,
        py_name: Option<&'static str>,
    ) {
        if DEBUG_MODE {
            if let Type::Subr(subr) = &t {
                if subr.has_qvar() {
                    panic!("not quantified subr: {subr}");
                }
            }
        }
        let impl_of = if let ContextKind::MethodDefs(Some(tr)) = &self.kind {
            Some(tr.clone())
        } else {
            None
        };
        let name = if PYTHON_MODE {
            if let Some(py_name) = py_name {
                VarName::from_static(py_name)
            } else {
                VarName::from_static(name)
            }
        } else {
            VarName::from_static(name)
        };
        if self.decls.get(&name).is_some() {
            panic!("already registered: {} {name}", self.name);
        } else {
            let vi = VarInfo::new(
                t,
                Immutable,
                vis,
                Builtin,
                None,
                impl_of,
                py_name.map(Str::ever),
                AbsLocation::unknown(),
            );
            self.decls.insert(name, vi);
        }
    }

    fn register_builtin_erg_decl(&mut self, name: &'static str, t: Type, vis: Visibility) {
        self.register_builtin_decl(name, t, vis, None);
    }

    fn register_builtin_impl(
        &mut self,
        name: VarName,
        t: Type,
        muty: Mutability,
        vis: Visibility,
        py_name: Option<&'static str>,
        loc: AbsLocation,
    ) {
        if cfg!(feature = "debug") {
            if let Type::Subr(subr) = &t {
                if subr.has_qvar() {
                    panic!("not quantified subr: {subr}");
                }
            }
        }
        let impl_of = if let ContextKind::MethodDefs(Some(tr)) = &self.kind {
            Some(tr.clone())
        } else {
            None
        };
        let vi = VarInfo::new(
            t,
            muty,
            vis,
            Builtin,
            None,
            impl_of,
            py_name.map(Str::ever),
            loc,
        );
        if let Some(_vi) = self.locals.get(&name) {
            if _vi != &vi {
                unreachable!("already registered: {} {name}", self.name);
            }
        } else {
            self.locals.insert(name, vi);
        }
    }

    fn register_builtin_erg_impl(
        &mut self,
        name: &'static str,
        t: Type,
        muty: Mutability,
        vis: Visibility,
    ) {
        let name = VarName::from_static(name);
        self.register_builtin_impl(name, t, muty, vis, None, AbsLocation::unknown());
    }

    // TODO: replace with `register_py_builtin`
    fn register_builtin_py_impl(
        &mut self,
        name: &'static str,
        t: Type,
        muty: Mutability,
        vis: Visibility,
        py_name: Option<&'static str>,
    ) {
        let name = if PYTHON_MODE {
            if let Some(py_name) = py_name {
                VarName::from_static(py_name)
            } else {
                VarName::from_static(name)
            }
        } else {
            VarName::from_static(name)
        };
        self.register_builtin_impl(name, t, muty, vis, py_name, AbsLocation::unknown());
    }

    pub(crate) fn register_py_builtin(
        &mut self,
        name: &'static str,
        t: Type,
        py_name: Option<&'static str>,
        lineno: u32,
    ) {
        let name = if PYTHON_MODE {
            if let Some(py_name) = py_name {
                VarName::from_static(py_name)
            } else {
                VarName::from_static(name)
            }
        } else {
            VarName::from_static(name)
        };
        let vis = if PYTHON_MODE || &self.name[..] != "<builtins>" {
            Visibility::BUILTIN_PUBLIC
        } else {
            Visibility::BUILTIN_PRIVATE
        };
        let muty = Immutable;
        let loc = Location::range(lineno, 0, lineno, name.inspect().len() as u32);
        let module = if &self.name[..] == "<builtins>" {
            builtins_path()
        } else {
            std_decl_path().join(format!("{}.d.er", self.name))
        };
        let abs_loc = AbsLocation::new(Some(module), loc);
        self.register_builtin_impl(name, t, muty, vis, py_name, abs_loc);
    }

    fn register_builtin_const(&mut self, name: &str, vis: Visibility, obj: ValueObj) {
        if self.rec_get_const_obj(name).is_some() {
            panic!("already registered: {} {name}", self.name);
        } else {
            let impl_of = if let ContextKind::MethodDefs(Some(tr)) = &self.kind {
                Some(tr.clone())
            } else {
                None
            };
            // TODO: not all value objects are comparable
            let vi = VarInfo::new(
                v_enum(set! {obj.clone()}),
                Const,
                vis,
                Builtin,
                None,
                impl_of,
                None,
                AbsLocation::unknown(),
            );
            self.consts.insert(VarName::from_str(Str::rc(name)), obj);
            self.locals.insert(VarName::from_str(Str::rc(name)), vi);
        }
    }

    fn register_const_param_defaults(&mut self, name: &'static str, params: Vec<ConstTemplate>) {
        if self.const_param_defaults.get(name).is_some() {
            panic!("already registered: {} {name}", self.name);
        } else {
            self.const_param_defaults.insert(Str::ever(name), params);
        }
    }

    /// FIXME: トレイトの汎化型を指定するのにも使っているので、この名前は適当でない
    pub(crate) fn register_superclass(&mut self, sup: Type, sup_ctx: &Context) {
        self.super_classes.push(sup);
        self.super_classes.extend(sup_ctx.super_classes.clone());
        self.super_traits.extend(sup_ctx.super_traits.clone());
        unique_in_place(&mut self.super_classes);
        unique_in_place(&mut self.super_traits);
    }

    pub(crate) fn register_supertrait(&mut self, sup: Type, sup_ctx: &Context) {
        self.super_traits.push(sup);
        self.super_traits.extend(sup_ctx.super_traits.clone());
        unique_in_place(&mut self.super_traits);
    }

    fn register_builtin_type(
        &mut self,
        t: Type,
        ctx: Self,
        vis: Visibility,
        muty: Mutability,
        py_name: Option<&'static str>,
    ) {
        if t.typarams_len().is_none() {
            self.register_mono_type(t, ctx, vis, muty, py_name);
        } else {
            self.register_poly_type(t, ctx, vis, muty, py_name);
        }
    }

    fn register_mono_type(
        &mut self,
        t: Type,
        ctx: Self,
        vis: Visibility,
        muty: Mutability,
        py_name: Option<&'static str>,
    ) {
        if self.rec_local_get_mono_type(&t.local_name()).is_some() {
            panic!("{} has already been registered", t.local_name());
        } else if self.rec_get_const_obj(&t.local_name()).is_some() {
            panic!("{} has already been registered as const", t.local_name());
        } else {
            let val = match ctx.kind {
                ContextKind::Class => ValueObj::builtin_class(t.clone()),
                ContextKind::Trait => ValueObj::builtin_trait(t.clone()),
                _ => ValueObj::builtin_type(t.clone()),
            };
            let name = VarName::from_str(t.local_name());
            let meta_t = v_enum(set! { val.clone() });
            self.locals.insert(
                name.clone(),
                VarInfo::new(
                    meta_t,
                    muty,
                    vis,
                    Builtin,
                    None,
                    None,
                    py_name.map(Str::ever),
                    AbsLocation::unknown(),
                ),
            );
            self.consts.insert(name.clone(), val);
            for impl_trait in ctx.super_traits.iter() {
                if let Some(impls) = self.trait_impls().get_mut(&impl_trait.qual_name()) {
                    impls.insert(TraitImpl::new(t.clone(), impl_trait.clone()));
                } else {
                    self.trait_impls().register(
                        impl_trait.qual_name(),
                        set![TraitImpl::new(t.clone(), impl_trait.clone())],
                    );
                }
            }
            for (trait_method, vi) in ctx.decls.iter() {
                if let Some(types) = self.method_to_traits.get_mut(trait_method.inspect()) {
                    types.push(MethodPair::new(t.clone(), vi.clone()));
                } else {
                    self.method_to_traits.insert(
                        trait_method.inspect().clone(),
                        vec![MethodPair::new(t.clone(), vi.clone())],
                    );
                }
            }
            for (class_method, vi) in ctx.locals.iter() {
                if let Some(types) = self.method_to_classes.get_mut(class_method.inspect()) {
                    types.push(MethodPair::new(t.clone(), vi.clone()));
                } else {
                    self.method_to_classes.insert(
                        class_method.inspect().clone(),
                        vec![MethodPair::new(t.clone(), vi.clone())],
                    );
                }
            }
            self.mono_types.insert(name, (t, ctx));
        }
    }

    // FIXME: MethodDefsと再代入は違う
    fn register_poly_type(
        &mut self,
        t: Type,
        ctx: Self,
        vis: Visibility,
        muty: Mutability,
        py_name: Option<&'static str>,
    ) {
        // FIXME: panic
        if let Some((_, root_ctx)) = self.poly_types.get_mut(&t.local_name()) {
            root_ctx.methods_list.push((ClassDefType::Simple(t), ctx));
        } else {
            let val = match ctx.kind {
                ContextKind::Class => ValueObj::builtin_class(t.clone()),
                ContextKind::Trait => ValueObj::builtin_trait(t.clone()),
                _ => ValueObj::builtin_type(t.clone()),
            };
            let name = VarName::from_str(t.local_name());
            // e.g Array!: |T, N|(_: {T}, _: {N}) -> {Array!(T, N)}
            let params = t
                .typarams()
                .into_iter()
                .map(|tp| ParamTy::Pos(tp_enum(self.get_tp_t(&tp).unwrap_or(Obj), set! { tp })))
                .collect();
            let meta_t = func(params, None, vec![], v_enum(set! { val.clone() })).quantify();
            if ERG_MODE {
                self.locals.insert(
                    name.clone(),
                    VarInfo::new(
                        meta_t,
                        muty,
                        vis,
                        Builtin,
                        None,
                        None,
                        py_name.map(Str::ever),
                        AbsLocation::unknown(),
                    ),
                );
            }
            self.consts.insert(name.clone(), val);
            for impl_trait in ctx.super_traits.iter() {
                if let Some(impls) = self.trait_impls().get_mut(&impl_trait.qual_name()) {
                    impls.insert(TraitImpl::new(t.clone(), impl_trait.clone()));
                } else {
                    self.trait_impls().register(
                        impl_trait.qual_name(),
                        set![TraitImpl::new(t.clone(), impl_trait.clone())],
                    );
                }
            }
            for (trait_method, vi) in ctx.decls.iter() {
                if let Some(traits) = self.method_to_traits.get_mut(trait_method.inspect()) {
                    traits.push(MethodPair::new(t.clone(), vi.clone()));
                } else {
                    self.method_to_traits.insert(
                        trait_method.inspect().clone(),
                        vec![MethodPair::new(t.clone(), vi.clone())],
                    );
                }
            }
            for (class_method, vi) in ctx.locals.iter() {
                if let Some(types) = self.method_to_classes.get_mut(class_method.inspect()) {
                    types.push(MethodPair::new(t.clone(), vi.clone()));
                } else {
                    self.method_to_classes.insert(
                        class_method.inspect().clone(),
                        vec![MethodPair::new(t.clone(), vi.clone())],
                    );
                }
            }
            self.poly_types.insert(name, (t, ctx));
        }
    }

    fn register_builtin_patch(
        &mut self,
        name: &'static str,
        ctx: Self,
        vis: Visibility,
        muty: Mutability,
    ) {
        if self.patches.contains_key(name) {
            panic!("{name} has already been registered");
        } else {
            let name = VarName::from_static(name);
            let vi = VarInfo::new(
                Patch,
                muty,
                vis,
                Builtin,
                None,
                None,
                None,
                AbsLocation::unknown(),
            );
            self.locals.insert(name.clone(), vi);
            for method_name in ctx.locals.keys() {
                if let Some(patches) = self.method_impl_patches.get_mut(method_name) {
                    patches.push(name.clone());
                } else {
                    self.method_impl_patches
                        .insert(method_name.clone(), vec![name.clone()]);
                }
            }
            if let ContextKind::GluePatch(tr_impl) = &ctx.kind {
                if let Some(impls) = self.trait_impls().get_mut(&tr_impl.sup_trait.qual_name()) {
                    impls.insert(tr_impl.clone());
                } else {
                    self.trait_impls()
                        .register(tr_impl.sup_trait.qual_name(), set![tr_impl.clone()]);
                }
            }
            self.patches.insert(name, ctx);
        }
    }

    fn init_builtin_consts(&mut self) {
        let vis = if PYTHON_MODE {
            Visibility::BUILTIN_PUBLIC
        } else {
            Visibility::BUILTIN_PRIVATE
        };
        self.register_builtin_py_impl(
            LICENSE,
            mono(SITEBUILTINS_PRINTER),
            Immutable,
            vis.clone(),
            Some(LICENSE),
        );
        self.register_builtin_py_impl(
            CREDITS,
            mono(SITEBUILTINS_PRINTER),
            Immutable,
            vis.clone(),
            Some(CREDITS),
        );
        self.register_builtin_py_impl(
            COPYRIGHT,
            mono(SITEBUILTINS_PRINTER),
            Immutable,
            vis.clone(),
            Some(COPYRIGHT),
        );
        self.register_builtin_py_impl(
            NOT_IMPLEMENTED,
            NotImplementedType,
            Const,
            vis.clone(),
            Some(NOT_IMPLEMENTED),
        );
        self.register_builtin_py_impl(ELLIPSIS, Ellipsis, Const, vis.clone(), Some(ELLIPSIS));
        self.register_builtin_py_impl(TRUE, Bool, Const, Visibility::BUILTIN_PRIVATE, Some(TRUE));
        self.register_builtin_py_impl(FALSE, Bool, Const, Visibility::BUILTIN_PRIVATE, Some(FALSE));
        self.register_builtin_py_impl(
            NONE,
            NoneType,
            Const,
            Visibility::BUILTIN_PRIVATE,
            Some(NONE),
        );
        if ERG_MODE {
            self.register_builtin_py_impl(
                FUNC_GLOBAL,
                module(TyParam::value("<builtins>")),
                Immutable,
                vis,
                None,
            );
        }
    }

    fn init_module_consts(&mut self) {
        let vis = if PYTHON_MODE {
            Visibility::BUILTIN_PUBLIC
        } else {
            Visibility::BUILTIN_PRIVATE
        };
        self.register_builtin_py_impl(
            FUNDAMENTAL_NAME,
            Str,
            Immutable,
            vis.clone(),
            Some(FUNDAMENTAL_NAME),
        );
        if ERG_MODE {
            self.register_builtin_py_impl(
                FUNC_MODULE,
                module(TyParam::value(self.get_module().unwrap().name.clone())),
                Immutable,
                vis,
                None,
            );
        }
    }

    pub(crate) fn init_builtins(cfg: ErgConfig, shared: SharedCompilerResource) {
        let mut ctx = Context::builtin_module("<builtins>", cfg, shared.clone(), 100);
        ctx.init_builtin_consts();
        ctx.init_builtin_funcs();
        ctx.init_builtin_const_funcs();
        ctx.init_builtin_procs();
        if PYTHON_MODE {
            ctx.init_builtin_py_specific_funcs();
            ctx.init_py_compat_builtin_operators();
        } else {
            ctx.init_builtin_operators();
        }
        ctx.init_builtin_traits();
        ctx.init_builtin_classes();
        ctx.init_builtin_patches();
        let module = ModuleContext::new(ctx, dict! {});
        shared
            .mod_cache
            .register(PathBuf::from("<builtins>"), None, module);
    }

    pub fn new_module<S: Into<Str>>(
        name: S,
        cfg: ErgConfig,
        shared: SharedCompilerResource,
    ) -> Self {
        let mut ctx = Context::new(
            name.into(),
            cfg,
            ContextKind::Module,
            vec![],
            None,
            Some(shared),
            Context::TOP_LEVEL,
        );
        ctx.init_module_consts();
        ctx
    }
}
