use erg_common::config::ErgConfig;
use erg_common::error::ErrorKind;
use erg_common::error::{ErrorDisplay, MultiErrorDisplay};
use erg_common::io::Input;
use erg_common::log;
use erg_common::traits::{BlockKind, ExitStatus, Locational, New, Runnable, Stream};

use erg_compiler::artifact::{Buildable, ErrorArtifact};
use erg_compiler::build_package::PackageBuilder;
use erg_compiler::error::{CompileError, CompileErrors, CompileWarnings};
use erg_compiler::hir::{Accessor, Def, Dict, Expr, List, Set, Signature, Tuple};
use erg_compiler::module::SharedCompilerResource;

use erg_parser::ParserRunner;

use crate::warn::too_many_params;

#[derive(Debug)]
pub struct Linter {
    pub cfg: ErgConfig,
    builder: PackageBuilder,
    warns: CompileWarnings,
}

impl Default for Linter {
    fn default() -> Self {
        Self::new(ErgConfig::default())
    }
}

impl New for Linter {
    fn new(cfg: ErgConfig) -> Self {
        let shared = SharedCompilerResource::new(cfg.copy());
        Self {
            builder: PackageBuilder::new(cfg.copy(), shared),
            cfg,
            warns: CompileWarnings::empty(),
        }
    }
}

impl Runnable for Linter {
    type Err = CompileError;
    type Errs = CompileErrors;
    const NAME: &'static str = "Erg linter";

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        &self.cfg
    }
    #[inline]
    fn cfg_mut(&mut self) -> &mut ErgConfig {
        &mut self.cfg
    }

    #[inline]
    fn finish(&mut self) {}

    fn initialize(&mut self) {
        self.builder.initialize();
        self.warns.clear();
    }

    fn clear(&mut self) {
        self.builder.clear();
        self.warns.clear();
    }

    fn set_input(&mut self, input: erg_common::io::Input) {
        self.cfg.input = input;
        self.builder.set_input(self.cfg.input.clone());
    }

    fn exec(&mut self) -> Result<ExitStatus, Self::Errs> {
        let warns = self.lint_module().map_err(|eart| {
            eart.warns.write_all_stderr();
            eart.errors
        })?;
        warns.write_all_stderr();
        Ok(ExitStatus::compile_passed(warns.len()))
    }

    fn eval(&mut self, src: String) -> Result<String, CompileErrors> {
        let warns = self.lint(src).map_err(|eart| {
            eart.warns.write_all_stderr();
            eart.errors
        })?;
        warns.write_all_stderr();
        Ok("OK".to_string())
    }

    fn expect_block(&self, src: &str) -> BlockKind {
        let mut parser = ParserRunner::new(self.cfg().clone());
        match parser.eval(src.to_string()) {
            Err(errs) => {
                let kind = errs
                    .iter()
                    .filter(|e| e.core().kind == ErrorKind::ExpectNextLine)
                    .map(|e| {
                        let msg = e.core().sub_messages.last().unwrap();
                        // ExpectNextLine error must have msg otherwise it's a bug
                        msg.get_msg().first().unwrap().to_owned()
                    })
                    .next();
                if let Some(kind) = kind {
                    return BlockKind::from(kind.as_str());
                }
                if errs
                    .iter()
                    .any(|err| err.core.main_message.contains("\"\"\""))
                {
                    return BlockKind::MultiLineStr;
                }
                BlockKind::Error
            }
            Ok(_) => {
                if src.contains("Class") {
                    return BlockKind::ClassDef;
                }
                BlockKind::None
            }
        }
    }
}

impl Linter {
    pub fn new(cfg: ErgConfig) -> Self {
        New::new(cfg)
    }

    fn caused_by(&self) -> String {
        self.builder.get_context().unwrap().context.caused_by()
    }

    fn input(&self) -> Input {
        self.builder.input().clone()
    }

    pub fn lint_module(&mut self) -> Result<CompileWarnings, ErrorArtifact> {
        let src = self.input().read();
        self.lint(src)
    }

    pub fn lint(&mut self, src: String) -> Result<CompileWarnings, ErrorArtifact> {
        log!(info "Start linting");
        let art = self.builder.build(src, "exec")?;
        self.warns.extend(art.warns);
        for chunk in art.object.module.iter() {
            self.lint_too_many_params(chunk);
        }
        log!(info "Finished linting");
        Ok(self.warns.take())
    }

    fn lint_too_many_params(&mut self, chunk: &Expr) {
        match chunk {
            Expr::Def(Def {
                sig: Signature::Subr(subr),
                body,
            }) => {
                if subr.params.len() >= 8 {
                    self.warns.push(too_many_params(
                        self.input(),
                        self.caused_by(),
                        subr.params.loc(),
                    ));
                }
                for chunk in body.block.iter() {
                    self.lint_too_many_params(chunk);
                }
            }
            _ => self.check_recursively(&Self::lint_too_many_params, chunk),
        }
    }

    /* ↓ Helper methods ↓ */
    fn check_recursively(&mut self, lint_fn: &impl Fn(&mut Linter, &Expr), expr: &Expr) {
        match expr {
            Expr::Literal(_) => {}
            Expr::Record(record) => {
                for attr in record.attrs.iter() {
                    for chunk in attr.body.block.iter() {
                        lint_fn(self, chunk);
                    }
                    if let Signature::Subr(subr) = &attr.sig {
                        for param in subr.params.defaults.iter() {
                            lint_fn(self, &param.default_val);
                        }
                    }
                }
            }
            Expr::List(list) => match list {
                List::Normal(lis) => {
                    for elem in lis.elems.pos_args.iter() {
                        lint_fn(self, &elem.expr);
                    }
                }
                List::WithLength(lis) => {
                    lint_fn(self, &lis.elem);
                    if let Some(len) = &lis.len {
                        lint_fn(self, len);
                    }
                }
                List::Comprehension(lis) => {
                    lint_fn(self, &lis.elem);
                    lint_fn(self, &lis.guard);
                }
            },
            Expr::Tuple(tuple) => match tuple {
                Tuple::Normal(tup) => {
                    for elem in tup.elems.pos_args.iter() {
                        lint_fn(self, &elem.expr);
                    }
                }
            },
            Expr::Set(set) => match set {
                Set::Normal(set) => {
                    for elem in set.elems.pos_args.iter() {
                        lint_fn(self, &elem.expr);
                    }
                }
                Set::WithLength(set) => {
                    lint_fn(self, &set.elem);
                    lint_fn(self, &set.len);
                }
            },
            Expr::Dict(dict) => match dict {
                Dict::Normal(dic) => {
                    for kv in dic.kvs.iter() {
                        lint_fn(self, &kv.key);
                        lint_fn(self, &kv.value);
                    }
                }
                _ => {
                    log!("Dict comprehension not implemented");
                }
            },
            Expr::BinOp(binop) => {
                lint_fn(self, &binop.lhs);
                lint_fn(self, &binop.rhs);
            }
            Expr::UnaryOp(unaryop) => {
                lint_fn(self, &unaryop.expr);
            }
            Expr::Call(call) => {
                lint_fn(self, &call.obj);
                for arg in call.args.iter() {
                    lint_fn(self, arg);
                }
            }
            Expr::Def(def) => {
                for chunk in def.body.block.iter() {
                    lint_fn(self, chunk);
                }
                if let Signature::Subr(subr) = &def.sig {
                    for param in subr.params.defaults.iter() {
                        lint_fn(self, &param.default_val);
                    }
                }
            }
            Expr::ClassDef(class_def) => {
                if let Signature::Subr(subr) = &class_def.sig {
                    for param in subr.params.defaults.iter() {
                        lint_fn(self, &param.default_val);
                    }
                }
                for methods in class_def.methods_list.iter() {
                    for method in methods.defs.iter() {
                        lint_fn(self, method);
                    }
                }
            }
            Expr::PatchDef(class_def) => {
                if let Signature::Subr(subr) = &class_def.sig {
                    for param in subr.params.defaults.iter() {
                        lint_fn(self, &param.default_val);
                    }
                }
                for method in class_def.methods.iter() {
                    lint_fn(self, method);
                }
            }
            Expr::ReDef(redef) => {
                self.check_acc_recursively(lint_fn, &redef.attr);
                for chunk in redef.block.iter() {
                    lint_fn(self, chunk);
                }
            }
            Expr::Lambda(lambda) => {
                for chunk in lambda.body.iter() {
                    lint_fn(self, chunk);
                }
                for param in lambda.params.defaults.iter() {
                    lint_fn(self, &param.default_val);
                }
            }
            Expr::TypeAsc(tasc) => {
                lint_fn(self, &tasc.expr);
            }
            Expr::Accessor(acc) => {
                self.check_acc_recursively(lint_fn, acc);
            }
            Expr::Compound(exprs) => {
                for chunk in exprs.iter() {
                    lint_fn(self, chunk);
                }
            }
            Expr::Dummy(exprs) => {
                for chunk in exprs.iter() {
                    lint_fn(self, chunk);
                }
            }
            _ => {}
        }
    }

    fn check_acc_recursively(&mut self, lint_fn: impl Fn(&mut Linter, &Expr), acc: &Accessor) {
        match acc {
            Accessor::Attr(attr) => lint_fn(self, &attr.obj),
            Accessor::Ident(_) => {}
        }
    }
}
