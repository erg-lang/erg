use std::path::Path;

use lsp_types::CompletionResponse;
use serde_json::Value;

use erg_common::config::ErgConfig;
use erg_common::consts::PYTHON_MODE;
use erg_common::dict::Dict;
use erg_common::env::erg_pystd_path;
use erg_common::impl_u8_enum;
use erg_common::io::Input;
use erg_common::python_util::{BUILTIN_PYTHON_MODS, EXT_COMMON_ALIAS, EXT_PYTHON_MODS};
use erg_common::set::Set;
use erg_common::shared::{MappedRwLockReadGuard, RwLockReadGuard, Shared};
use erg_common::spawn::spawn_new_thread;
use erg_common::traits::Locational;

use erg_compiler::artifact::{BuildRunnable, Buildable};
use erg_compiler::build_package::PackageBuilder;
use erg_compiler::context::Context;
use erg_compiler::erg_parser::parse::Parsable;
use erg_compiler::erg_parser::token::TokenKind;
use erg_compiler::hir::Expr;
use erg_compiler::module::SharedCompilerResource;
use erg_compiler::ty::{HasType, ParamTy, Type};
use erg_compiler::varinfo::{AbsLocation, VarInfo};
use TokenKind::*;

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, Documentation, MarkedString,
    MarkupContent, MarkupKind, Position, Range, TextEdit,
};

use crate::_log;
use crate::server::{ELSResult, Flags, RedirectableStdout, Server};
use crate::util::{self, loc_to_pos, NormalizedUrl};

fn comp_item_kind(vi: &VarInfo) -> CompletionItemKind {
    match &vi.t {
        Type::Subr(subr) if subr.self_t().is_some() => CompletionItemKind::METHOD,
        Type::Quantified(quant) if quant.self_t().is_some() => CompletionItemKind::METHOD,
        Type::Subr(_) | Type::Quantified(_) => CompletionItemKind::FUNCTION,
        Type::ClassType => CompletionItemKind::CLASS,
        Type::TraitType => CompletionItemKind::INTERFACE,
        t if matches!(&t.qual_name()[..], "Module" | "PyModule" | "GenericModule") => {
            CompletionItemKind::MODULE
        }
        _ if vi.muty.is_const() => CompletionItemKind::CONSTANT,
        _ => CompletionItemKind::VARIABLE,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CompletionKind {
    RetriggerLocal,
    Local,
    LParen,
    Method,
    RetriggerMethod,
    // Colon, // :, Type ascription or private access `::`
}

impl CompletionKind {
    pub const fn should_be_local(&self) -> bool {
        matches!(self, Self::RetriggerLocal | Self::Local | Self::LParen)
    }

    pub const fn should_be_method(&self) -> bool {
        matches!(self, Self::Method | Self::RetriggerMethod)
    }

    pub const fn _is_lparen(&self) -> bool {
        matches!(self, Self::LParen)
    }
}

fn mark_to_string(mark: MarkedString) -> String {
    match mark {
        MarkedString::String(s) => s,
        MarkedString::LanguageString(ls) => format!("```{}\n{}\n```", ls.language, ls.value),
    }
}

fn markdown_order(block: &str) -> usize {
    if block.starts_with("```") {
        usize::MAX
    } else {
        0
    }
}

impl_u8_enum! { CompletionOrder; i32;
    TypeMatched = -32,
    NameMatched = -8,
    ReturnTypeMatched = -2,
    Normal = 1000000,
    Builtin = 1,
    OtherNamespace = 2,
    Escaped = 32,
    DoubleEscaped = 64,
}

impl CompletionOrder {
    pub const STD_ITEM: char = match char::from_u32(
        CompletionOrder::Normal as u32
            + CompletionOrder::Builtin as u32
            + CompletionOrder::OtherNamespace as u32,
    ) {
        Some(c) => c,
        None => unreachable!(),
    };
}

pub struct CompletionOrderSetter<'b> {
    vi: &'b VarInfo,
    arg_pt: Option<&'b ParamTy>,
    mod_ctx: &'b Context, // for subtype judgement, not for variable lookup
    label: String,
}

impl<'b> CompletionOrderSetter<'b> {
    pub fn new(
        vi: &'b VarInfo,
        arg_pt: Option<&'b ParamTy>,
        mod_ctx: &'b Context,
        label: String,
    ) -> Self {
        Self {
            vi,
            arg_pt,
            mod_ctx,
            label,
        }
    }

    pub fn score(&self) -> i32 {
        let mut orders = vec![CompletionOrder::Normal];
        if self.label.starts_with("__") {
            orders.push(CompletionOrder::DoubleEscaped);
        } else if self.label.starts_with('_') {
            orders.push(CompletionOrder::Escaped);
        }
        if self.vi.kind.is_builtin() {
            orders.push(CompletionOrder::Builtin);
        }
        if self
            .arg_pt
            .map_or(false, |pt| pt.name().map(|s| &s[..]) == Some(&self.label))
        {
            orders.push(CompletionOrder::NameMatched);
        }
        #[allow(clippy::blocks_in_conditions)]
        if self
            .arg_pt
            .map_or(false, |pt| self.mod_ctx.subtype_of(&self.vi.t, pt.typ()))
        {
            orders.push(CompletionOrder::TypeMatched);
        } else if self.arg_pt.map_or(false, |pt| {
            let Some(return_t) = self.vi.t.return_t() else {
                return false;
            };
            if return_t.has_qvar() {
                return false;
            }
            self.mod_ctx.subtype_of(return_t, pt.typ())
        }) {
            orders.push(CompletionOrder::ReturnTypeMatched);
        }
        orders.into_iter().map(i32::from).sum()
    }

    pub fn mangle(&self) -> String {
        let score = self.score();
        format!(
            "{}_{}",
            char::from_u32(score as u32).unwrap_or(CompletionOrder::STD_ITEM),
            self.label
        )
    }

    fn set(&self, item: &mut CompletionItem) {
        item.sort_text = Some(self.mangle());
    }
}

type Cache = Shared<Dict<String, Vec<CompletionItem>>>;

#[derive(Debug, Clone)]
pub struct CompletionCache {
    cache: Cache,
}

fn external_item(name: &str, vi: &VarInfo, mod_name: &str) -> CompletionItem {
    #[cfg(feature = "py_compat")]
    let mod_name = mod_name.replace('/', ".");
    let mut item =
        CompletionItem::new_simple(format!("{name} (import from {mod_name})"), vi.t.to_string());
    item.sort_text = Some(format!("{}_{}", CompletionOrder::STD_ITEM, item.label));
    item.kind = Some(comp_item_kind(vi));
    let import = if PYTHON_MODE {
        format!("from {mod_name} import {name}\n")
    } else {
        format!("{{{name};}} = pyimport \"{mod_name}\"\n")
    };
    item.additional_text_edits = Some(vec![TextEdit {
        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        new_text: import,
    }]);
    item.insert_text = Some(name.trim_end_matches('\0').to_string());
    item.filter_text = Some(name.to_string());
    item
}

fn module_item(name: &str, mistype: bool, insert: Option<u32>) -> CompletionItem {
    let mut item =
        CompletionItem::new_simple(format!("{name} (magic completion)"), "Module".to_string());
    item.kind = Some(CompletionItemKind::MODULE);
    // `import datetime`
    // => `datetime = pyimport "datetime"`
    if let Some(line) = insert {
        let prefix = if mistype { "py" } else { "" };
        let import = format!("{} = {prefix}", name.split('/').last().unwrap_or("module"));
        item.additional_text_edits = Some(vec![TextEdit {
            range: Range::new(Position::new(line - 1, 0), Position::new(line - 1, 0)),
            new_text: import,
        }]);
    }
    item.sort_text = mistype.then(|| format!("{}_{}", CompletionOrder::STD_ITEM, item.label));
    item.insert_text = Some(format!("\"{name}\""));
    item.filter_text = Some(name.to_string());
    item
}

fn module_completions() -> Vec<CompletionItem> {
    let mut comps = Vec::with_capacity(BUILTIN_PYTHON_MODS.len());
    for mod_name in BUILTIN_PYTHON_MODS.into_iter() {
        let mut item = CompletionItem::new_simple(
            format!("{mod_name} (import from std)"),
            "PyModule".to_string(),
        );
        item.sort_text = Some(format!("{}_{}", CompletionOrder::STD_ITEM, item.label));
        item.kind = Some(CompletionItemKind::MODULE);
        let import = if PYTHON_MODE {
            format!("import {mod_name}\n")
        } else {
            format!("{mod_name} = pyimport \"{mod_name}\"\n")
        };
        item.additional_text_edits = Some(vec![TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            new_text: import,
        }]);
        item.insert_text = Some(mod_name.trim_end_matches('\0').to_string());
        item.filter_text = Some(mod_name.to_string());
        comps.push(item);
    }
    #[cfg(not(feature = "py_compat"))]
    for mod_name in erg_common::erg_util::BUILTIN_ERG_MODS {
        let mut item = CompletionItem::new_simple(
            format!("{mod_name} (import from std)"),
            "Module".to_string(),
        );
        item.sort_text = Some(format!("{}_{}", CompletionOrder::STD_ITEM, item.label));
        item.kind = Some(CompletionItemKind::MODULE);
        let import = format!("{mod_name} = import \"{mod_name}\"\n");
        item.additional_text_edits = Some(vec![TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            new_text: import,
        }]);
        item.insert_text = Some(mod_name.to_string());
        item.filter_text = Some(mod_name.to_string());
        comps.push(item);
    }
    for (mod_name, alias) in EXT_PYTHON_MODS.into_iter().zip(EXT_COMMON_ALIAS) {
        let mut item = CompletionItem::new_simple(
            format!("{mod_name} (external library)"),
            "PyModule".to_string(),
        );
        item.sort_text = Some(format!("{}_{}", CompletionOrder::STD_ITEM, item.label));
        item.kind = Some(CompletionItemKind::MODULE);
        let import = if PYTHON_MODE {
            format!("import {mod_name}\n")
        } else {
            format!("{mod_name} = pyimport \"{mod_name}\"\n")
        };
        item.additional_text_edits = Some(vec![TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            new_text: import,
        }]);
        item.insert_text = Some(mod_name.trim_end_matches('\0').to_string());
        item.filter_text = Some(mod_name.to_string());
        comps.push(item);
        if mod_name != alias {
            let mut item = CompletionItem::new_simple(
                format!("{alias} (external library, alias of {mod_name})"),
                "PyModule".to_string(),
            );
            item.sort_text = Some(format!("{}_{}", CompletionOrder::STD_ITEM, item.label));
            item.kind = Some(CompletionItemKind::MODULE);
            let import = if PYTHON_MODE {
                format!("import {mod_name} as {alias}\n")
            } else {
                format!("{alias} = pyimport \"{mod_name}\"\n")
            };
            item.additional_text_edits = Some(vec![TextEdit {
                range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                new_text: import,
            }]);
            item.insert_text = Some(alias.trim_end_matches('\0').to_string());
            item.filter_text = Some(mod_name.to_string());
            comps.push(item);
        }
    }
    comps
}

fn load_modules<'a>(
    cfg: ErgConfig,
    cache: Cache,
    root: &Path,
    mods: impl Iterator<Item = &'a str>,
) {
    let src = mods.fold("".to_string(), |acc, module| {
        acc + &format!("_ = pyimport \"{module}\"\n")
    });
    let cfg = ErgConfig {
        input: Input::str(src.clone()),
        ..cfg
    };
    let shared = SharedCompilerResource::new(cfg.clone());
    let mut checker = PackageBuilder::inherit(cfg, shared.clone());
    let _res = checker.build(src, "exec");
    let mut cache = cache.borrow_mut();
    if cache.get("<module>").is_none() {
        cache.insert("<module>".into(), module_completions());
    }
    let std_path = root.display().to_string().replace('\\', "/");
    for (path, entry) in shared.py_mod_cache.ref_inner().iter() {
        let dir = entry.module.context.local_dir();
        let mod_name = path.display().to_string().replace('\\', "/");
        let mod_name = mod_name
            .trim_start_matches(&std_path)
            .trim_start_matches('/')
            .trim_end_matches("/__init__.d.er")
            .trim_end_matches(".d.er")
            .replace(".d", "");
        let items = dir
            .into_iter()
            .filter(|(name, _)| !name.inspect().starts_with('%'))
            .map(|(name, vi)| external_item(name.inspect(), vi, &mod_name));
        cache.get_mut("<module>").unwrap().extend(items)
    }
}

impl CompletionCache {
    pub fn new(cfg: ErgConfig, flags: Flags) -> Self {
        let cache = Shared::new(Dict::default());
        let clone = cache.clone();
        spawn_new_thread(
            move || {
                // crate::_log!("load_modules");
                let major_mods = [
                    "argparse",
                    "array",
                    "asyncio",
                    "base64",
                    "datetime",
                    "decimal",
                    "fraction",
                    "glob",
                    "html",
                    "http",
                    "http/client",
                    "http/server",
                    "io",
                    "json",
                    "logging",
                    "math",
                    "os",
                    "os/path",
                    "pathlib",
                    "platform",
                    "random",
                    "re",
                    "shutil",
                    "socket",
                    "sqlite3",
                    "ssl",
                    "string",
                    "subprocess",
                    "sys",
                    "tempfile",
                    "time",
                    "timeit",
                    "unittest",
                    "urllib",
                    "zipfile",
                ];
                #[cfg(feature = "py_compat")]
                let py_specific_mods = ["dataclasses", "typing", "collections/abc"];
                #[cfg(not(feature = "py_compat"))]
                let py_specific_mods = [];
                load_modules(
                    cfg.clone(),
                    clone.clone(),
                    erg_pystd_path(),
                    major_mods.into_iter().chain(py_specific_mods),
                );
                // TODO: load modules from site-packages
                flags
                    .builtin_modules_loaded
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            },
            "load_modules",
        );
        Self { cache }
    }

    pub fn get(&self, namespace: &str) -> Option<MappedRwLockReadGuard<Vec<CompletionItem>>> {
        if self.cache.borrow().get(namespace).is_none() {
            None
        } else {
            Some(RwLockReadGuard::map(self.cache.borrow(), |cache| {
                cache.get(namespace).unwrap()
            }))
        }
    }

    pub fn insert(&self, namespace: String, items: Vec<CompletionItem>) {
        self.cache.borrow_mut().insert(namespace, items);
    }

    pub fn clear(&self) {
        self.cache.borrow_mut().clear();
    }

    pub fn _append(&self, cache: Dict<String, Vec<CompletionItem>>) {
        for (k, v) in cache {
            if let Some(comps) = self.cache.borrow_mut().get_mut(&k) {
                comps.extend(v);
            } else {
                self.cache.borrow_mut().insert(k, v);
            }
        }
    }
}

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    /// Returns completion candidates from modules in the same directory
    fn neighbor_completion(
        &self,
        uri: &NormalizedUrl,
        arg_pt: Option<ParamTy>,
        already_appeared: &mut Set<String>,
    ) -> Vec<CompletionItem> {
        let mut comps = vec![];
        for mod_ctx in self.get_neighbor_ctxs(uri) {
            for (name, vi) in mod_ctx.local_dir() {
                if vi.vis.is_private() {
                    continue;
                }
                let Some(path) = vi.def_loc.module.as_ref() else {
                    continue;
                };
                let path = path.file_stem().unwrap().to_string_lossy();
                let label = format!("{name} (import from {path})");
                if already_appeared.contains(&label[..]) {
                    continue;
                }
                let mut item = CompletionItem::new_simple(label, vi.t.to_string());
                CompletionOrderSetter::new(vi, arg_pt.as_ref(), mod_ctx, item.label.clone())
                    .set(&mut item);
                // item.sort_text = Some(format!("{}_{}", CompletionOrder::OtherNamespace, item.label));
                item.kind = Some(comp_item_kind(vi));
                item.data = Some(Value::String(vi.def_loc.to_string()));
                let import = if PYTHON_MODE {
                    format!("from {path} import {name}\n")
                } else {
                    format!("{{{name};}} = import \"{path}\"\n")
                };
                item.additional_text_edits = Some(vec![TextEdit {
                    range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                    new_text: import,
                }]);
                item.insert_text = Some(name.inspect().trim_end_matches('\0').to_string());
                item.filter_text = Some(name.inspect().to_string());
                already_appeared.insert(item.label.clone());
                comps.push(item);
            }
        }
        comps
    }

    pub(crate) fn handle_completion(
        &mut self,
        params: CompletionParams,
    ) -> ELSResult<Option<CompletionResponse>> {
        _log!(self, "completion requested: {params:?}");
        let uri = NormalizedUrl::new(params.text_document_position.text_document.uri);
        let path = util::uri_to_path(&uri);
        let mut pos = params.text_document_position.position;
        // ignore comments
        // TODO: multiline comments
        if self
            .file_cache
            .get_line(&uri, pos.line)
            .map_or(false, |line| line.starts_with('#'))
        {
            return Ok(None);
        }
        let trigger = params
            .context
            .as_ref()
            .and_then(|comp_ctx| comp_ctx.trigger_character.as_ref().map(|s| &s[..]));
        let comp_kind = match trigger {
            Some(".") => CompletionKind::Method,
            Some(":") => CompletionKind::Method,
            Some(" ") => CompletionKind::Local,
            Some("(") => CompletionKind::LParen,
            _ => {
                let offset = match self.file_cache.get_token(&uri, pos).map(|tk| tk.kind) {
                    Some(TokenKind::Newline | TokenKind::EOF) => -2,
                    _ => -1,
                };
                let prev_token = self.file_cache.get_token_relatively(&uri, pos, offset);
                match prev_token {
                    Some(prev) if matches!(prev.kind, Dot | DblColon) => {
                        if let Some(p) = loc_to_pos(prev.loc()) {
                            pos = p;
                        }
                        CompletionKind::RetriggerMethod
                    }
                    _ => CompletionKind::RetriggerLocal,
                }
            }
        };
        self.send_log(format!("CompletionKind: {comp_kind:?}"))?;
        let mut result: Vec<CompletionItem> = vec![];
        let mut already_appeared = Set::new();
        let (receiver_t, contexts) = if comp_kind.should_be_local() {
            (None, self.get_local_ctx(&uri, pos))
        } else {
            self.get_receiver_and_ctxs(&uri, pos)?
        };
        let offset = match comp_kind {
            CompletionKind::RetriggerLocal => 0,
            CompletionKind::Method => -1,
            CompletionKind::Local => -1,
            CompletionKind::LParen => 0,
            CompletionKind::RetriggerMethod => -1,
        };
        let arg_pt = self
            .get_min_expr(&uri, pos, offset)
            .and_then(|(token, expr)| match expr {
                Expr::Call(call) => {
                    let sig_t = call.obj.t();
                    let nth = self.nth(&uri, &call, pos);
                    let additional = if matches!(token.kind, Comma) { 1 } else { 0 };
                    let nth = nth + additional;
                    sig_t.non_default_params()?.get(nth).cloned()
                }
                other if comp_kind == CompletionKind::Local => {
                    match other.show_acc().as_deref() {
                        Some("import") => {
                            let insert = other
                                .col_begin()
                                .and_then(|cb| (cb == 0).then(|| other.ln_begin().unwrap_or(0)));
                            for erg_mod in erg_common::erg_util::BUILTIN_ERG_MODS {
                                result.push(module_item(erg_mod, false, insert));
                            }
                            for py_mod in BUILTIN_PYTHON_MODS {
                                result.push(module_item(py_mod, true, insert));
                            }
                        }
                        Some("pyimport") => {
                            let insert = other
                                .col_begin()
                                .and_then(|cb| (cb == 0).then(|| other.ln_begin().unwrap_or(0)));
                            for py_mod in BUILTIN_PYTHON_MODS {
                                result.push(module_item(py_mod, false, insert));
                            }
                        }
                        _ => {}
                    }
                    let sig_t = other.t();
                    sig_t.non_default_params()?.first().cloned()
                }
                _ => None,
            });
        let Some(mod_ctx) = self.get_mod_ctx(&uri) else {
            _log!(self, "module context not found: {uri}");
            return Ok(Some(CompletionResponse::Array(result)));
        };
        for (name, vi) in contexts.into_iter().flat_map(|ctx| ctx.local_dir()) {
            if comp_kind.should_be_method() && vi.vis.is_private() {
                continue;
            }
            // only show static methods, if the receiver is a type
            if vi.t.is_method()
                && receiver_t.as_ref().map_or(true, |receiver| {
                    !mod_ctx
                        .context
                        .subtype_of(receiver, vi.t.self_t().unwrap_or(Type::OBJ))
                })
            {
                continue;
            }
            let label = name.inspect();
            // don't show overridden items
            if already_appeared.contains(&label[..]) {
                continue;
            }
            if label.starts_with('%') {
                continue;
            }
            let label = label.trim_end_matches('\0').to_string();
            // don't show future defined items
            if vi.def_loc.module.as_deref() == Some(&path)
                && name.ln_begin().unwrap_or(0) > pos.line + 1
            {
                continue;
            }
            let readable_t = mod_ctx.context.readable_type(vi.t.clone());
            let mut item = CompletionItem::new_simple(label, readable_t.to_string());
            CompletionOrderSetter::new(vi, arg_pt.as_ref(), &mod_ctx.context, item.label.clone())
                .set(&mut item);
            item.kind = Some(comp_item_kind(vi));
            item.data = Some(Value::String(vi.def_loc.to_string()));
            already_appeared.insert(item.label.clone());
            result.push(item);
        }
        if comp_kind.should_be_local() {
            if let Some(comps) = self.comp_cache.get("<module>") {
                result.extend(comps.clone());
            } else {
                let comps = module_completions();
                self.comp_cache.insert("<module>".into(), comps.clone());
                result.extend(comps);
            }
            result.extend(self.neighbor_completion(&uri, arg_pt, &mut already_appeared));
        }
        _log!(self, "completion items: {}", result.len());
        Ok(Some(CompletionResponse::Array(result)))
    }

    pub(crate) fn handle_resolve_completion(
        &mut self,
        mut item: CompletionItem,
    ) -> ELSResult<CompletionItem> {
        self.send_log(format!("completion resolve requested: {item:?}"))?;
        if let Some(data) = &item.data {
            let mut contents = vec![];
            let Ok(def_loc) = data.as_str().unwrap_or_default().parse::<AbsLocation>() else {
                return Ok(item);
            };
            self.show_doc_comment(None, &mut contents, &def_loc)?;
            let mut contents = contents.into_iter().map(mark_to_string).collect::<Vec<_>>();
            contents.sort_by_key(|cont| markdown_order(cont));
            item.documentation = Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: contents.join("\n"),
            }));
        }
        Ok(item)
    }
}
