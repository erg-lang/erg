#![allow(unused_imports)]

use erg_common::traits::NoTypeDisplay;
use erg_compiler::ty::HasType;
use lsp_types::Position;
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_common::dict::Dict;
use erg_common::error::Location;
use erg_common::traits::{Locational, Runnable, Stream};

use erg_compiler::artifact::{BuildRunnable, IncompleteArtifact};
use erg_compiler::hir::{Block, Call, ClassDef, Def, Expr, Lambda, Params, PatchDef, Signature};
use lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams};

use crate::server::{ELSResult, Server};
use crate::util;
use crate::util::loc_to_range;

fn anot(ln: u32, col: u32, cont: String) -> InlayHint {
    let position = Position::new(ln - 1, col);
    let label = InlayHintLabel::String(cont);
    let kind = Some(InlayHintKind::TYPE);
    InlayHint {
        position,
        label,
        kind,
        text_edits: None,
        tooltip: None,
        padding_left: Some(false),
        padding_right: Some(false),
        data: None,
    }
}

fn type_anot<D: std::fmt::Display>(ln_end: u32, col_end: u32, ty: D, return_t: bool) -> InlayHint {
    let position = Position::new(ln_end - 1, col_end);
    let string = if return_t {
        format!("): {ty}")
    } else {
        format!(": {ty}")
    };
    let label = InlayHintLabel::String(string);
    let kind = Some(InlayHintKind::TYPE);
    InlayHint {
        position,
        label,
        kind,
        text_edits: None,
        tooltip: None,
        padding_left: Some(return_t),
        padding_right: Some(false),
        data: None,
    }
}

fn type_bounds_anot(ln_end: u32, col_end: u32, ty_bounds: String) -> InlayHint {
    let position = Position::new(ln_end - 1, col_end);
    let label = InlayHintLabel::String(ty_bounds);
    let kind = Some(InlayHintKind::TYPE);
    InlayHint {
        position,
        label,
        kind,
        text_edits: None,
        tooltip: None,
        padding_left: Some(false),
        padding_right: Some(false),
        data: None,
    }
}

fn param_anot<D: std::fmt::Display>(ln_begin: u32, col_begin: u32, name: D) -> InlayHint {
    let position = Position::new(ln_begin - 1, col_begin);
    let label = InlayHintLabel::String(format!("{name}:= "));
    let kind = Some(InlayHintKind::PARAMETER);
    InlayHint {
        position,
        label,
        kind,
        text_edits: None,
        tooltip: None,
        padding_left: Some(false),
        padding_right: Some(false),
        data: None,
    }
}

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn get_inlay_hint(&mut self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("inlay hint request: {msg}"))?;
        let params = InlayHintParams::deserialize(&msg["params"])?;
        let uri = util::normalize_url(params.text_document.uri);
        let mut result = vec![];
        if let Some(IncompleteArtifact {
            object: Some(hir), ..
        }) = self.artifacts.get(&uri)
        {
            for chunk in hir.module.iter() {
                result.extend(self.get_expr_hint(chunk));
            }
        }
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }),
        )
    }

    fn get_expr_hint(&self, expr: &Expr) -> Vec<InlayHint> {
        match expr {
            Expr::Def(def) if def.sig.is_subr() => self.get_subr_def_hint(def),
            Expr::Def(def) => self.get_var_def_hint(def),
            Expr::Lambda(lambda) => self.get_lambda_hint(lambda),
            Expr::ClassDef(class_def) => self.get_class_def_hint(class_def),
            Expr::PatchDef(patch_def) => self.get_patch_def_hint(patch_def),
            Expr::Call(call) => self.get_call_hint(call),
            _ => vec![],
        }
    }

    fn get_param_hint(&self, params: &Params) -> Vec<InlayHint> {
        let mut result = vec![];
        for nd_param in params.non_defaults.iter() {
            if nd_param.raw.t_spec.is_some() {
                continue;
            }
            let hint = type_anot(
                nd_param.ln_end().unwrap(),
                nd_param.col_end().unwrap(),
                &nd_param.vi.t,
                false,
            );
            result.push(hint);
        }
        if let Some(var_params) = &params.var_params {
            if var_params.raw.t_spec.is_some() {
                return result;
            }
            let hint = type_anot(
                var_params.ln_end().unwrap(),
                var_params.col_end().unwrap(),
                &var_params.vi.t,
                false,
            );
            result.push(hint);
        }
        for d_param in params.defaults.iter() {
            if d_param.sig.raw.t_spec.is_some() {
                continue;
            }
            let hint = type_anot(
                d_param.sig.ln_end().unwrap(),
                d_param.sig.col_end().unwrap(),
                &d_param.sig.vi.t,
                false,
            );
            result.push(hint);
        }
        result
    }

    fn get_subr_def_hint(&self, def: &Def) -> Vec<InlayHint> {
        let mut result = vec![];
        result.extend(self.get_block_hint(&def.body.block));
        let Signature::Subr(subr) = &def.sig else { unreachable!() };
        if subr.ref_t().is_quantified() && subr.bounds.is_empty() {
            let subr = subr.ref_t().to_string();
            let ty_bounds = format!("|{}|", subr.split('|').nth(1).unwrap_or(""));
            let hint = type_bounds_anot(
                def.sig.ident().ln_end().unwrap(),
                def.sig.ident().col_end().unwrap(),
                ty_bounds,
            );
            result.push(hint);
        }
        result.extend(self.get_param_hint(&subr.params));
        if def.sig.t_spec().is_none() {
            let Some(return_t) = subr.ref_t().return_t() else {
                return result;
            };
            let hint = type_anot(
                def.sig.ln_end().unwrap(),
                def.sig.col_end().unwrap(),
                return_t,
                subr.params.parens.is_none(),
            );
            result.push(hint);
            if subr.params.parens.is_none() {
                let hint = anot(
                    subr.params.ln_begin().unwrap(),
                    subr.params.col_begin().unwrap(),
                    "(".to_string(),
                );
                result.push(hint);
            }
        }
        result
    }

    fn get_var_def_hint(&self, def: &Def) -> Vec<InlayHint> {
        let mut result = self.get_block_hint(&def.body.block);
        // don't show hints for compiler internal variables
        if def.sig.t_spec().is_none() && !def.sig.ident().inspect().starts_with(['%']) {
            let hint = type_anot(
                def.sig.ln_end().unwrap(),
                def.sig.col_end().unwrap(),
                def.sig.ident().ref_t(),
                false,
            );
            result.push(hint);
        }
        result
    }

    fn get_lambda_hint(&self, lambda: &Lambda) -> Vec<InlayHint> {
        let mut result = vec![];
        result.extend(self.get_block_hint(&lambda.body));
        result.extend(self.get_param_hint(&lambda.params));
        let return_t = lambda.ref_t().return_t().unwrap();
        let hint = type_anot(
            lambda.params.ln_end().unwrap(),
            lambda.params.col_end().unwrap(),
            return_t,
            lambda.params.parens.is_none(),
        );
        if lambda.params.parens.is_none() {
            let hint = anot(
                lambda.params.ln_begin().unwrap(),
                lambda.params.col_begin().unwrap(),
                "(".to_string(),
            );
            result.push(hint);
        }
        result.push(hint);
        result
    }

    fn get_class_def_hint(&self, class_def: &ClassDef) -> Vec<InlayHint> {
        class_def
            .methods
            .iter()
            .flat_map(|expr| self.get_expr_hint(expr))
            .collect()
    }

    fn get_patch_def_hint(&self, patch_def: &PatchDef) -> Vec<InlayHint> {
        patch_def
            .methods
            .iter()
            .flat_map(|expr| self.get_expr_hint(expr))
            .collect()
    }

    fn get_call_hint(&self, call: &Call) -> Vec<InlayHint> {
        let mut result = vec![];
        let Some(call_t) = call.signature_t() else {
            return vec![];
        };
        let Some(param_ts) = call_t.non_var_params() else {
            return vec![];
        };
        let is_method = call.is_method_call();
        for (i, pos_arg) in call.args.pos_args.iter().enumerate() {
            let arg_is_lambda = matches!(&pos_arg.expr, Expr::Lambda(_));
            result.extend(self.get_expr_hint(&pos_arg.expr));
            let index = if is_method { i + 1 } else { i };
            if let Some(name) = param_ts.clone().nth(index).and_then(|pt| pt.name()) {
                let disp_arg = pos_arg.expr.to_string_notype();
                // if param_name is same as arg_name
                if disp_arg.trim_start_matches("::") == &name[..] {
                    continue;
                }
                let (Some(ln_begin), Some(col_begin)) = (pos_arg.ln_begin(), pos_arg.col_begin()) else {
                    continue;
                };
                // f i -> ...
                // NG: f(proc:= i: T): U -> ...
                // OK: f proc:= (i: T): U -> ...
                let (name, col_begin) = if arg_is_lambda {
                    (format!(" {name}"), col_begin.saturating_sub(1))
                } else {
                    (name.to_string(), col_begin)
                };
                let hint = param_anot(ln_begin, col_begin, name);
                result.push(hint);
            }
        }
        result
    }

    fn get_block_hint(&self, block: &Block) -> Vec<InlayHint> {
        block
            .iter()
            .flat_map(|expr| self.get_expr_hint(expr))
            .collect()
    }
}
