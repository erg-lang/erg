use erg_compiler::erg_parser::ast::ClassAttr;
use lsp_types::SemanticToken;
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;

use erg_common::dict::Dict;
use erg_common::error::Location;
use erg_common::traits::{Runnable, Locational};

use erg_compiler::erg_parser::ast::{Accessor, Args, AST, BinOp, Block, Call, ClassDef, Expr, Def, DefKind, Params, UnaryOp};
use erg_compiler::ASTBuilder;
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::token::TokenKind;

use lsp_types::{SemanticTokens, SemanticTokensParams, SemanticTokenType};

use crate::server::{ELSResult, Server};
use crate::util;

#[derive(Debug)]
struct ASTSemanticState {
    namespaces: Vec<Dict<String, SemanticTokenType>>,
    tokens: Vec<SemanticToken>,
}

impl ASTSemanticState {
    fn new() -> Self {
        Self {
            namespaces: vec![Dict::new()],
            tokens: Vec::new(),
        }
    }

    fn get_type(&self, key: &str) -> Option<SemanticTokenType> {
        for namespace in self.namespaces.iter().rev() {
            if let Some(typ) = namespace.get(key) {
                return Some(typ.clone());
            }
        }
        None
    }

    fn get_variable_type(&self, key: &str) -> SemanticTokenType {
        self.get_type(key).unwrap_or(SemanticTokenType::VARIABLE)
    }

    fn push_current_namespace(&mut self, key: String, typ: SemanticTokenType) {
        let current = self.namespaces.last_mut().unwrap();
        current.insert(key, typ);
    }

    fn enumerate_tokens(&mut self, ast: AST) -> SemanticTokens {
        for expr in ast.module.into_iter() {
            let tokens = self.gen_from_expr(expr);
            self.tokens.extend(tokens);
        }
        SemanticTokens {
            result_id: None,
            data: std::mem::take(&mut self.tokens),
        }
    }

    fn token_type_as_u32(token_type: SemanticTokenType) -> u32 {
        match token_type.as_str() {
            "namespace" => 0,
            "type" => 1,
            "class" => 2,
            "enum" => 3,
            "interface" => 4,
            "struct" => 5,
            "typeParameter" => 6,
            "parameter" => 7,
            "variable" => 8,
            "property" => 9,
            "enumMember" => 10,
            "event" => 11,
            "function" => 12,
            "method" => 13,
            "macro" => 14,
            "keyword" => 15,
            "modifier" => 16,
            "comment" => 17,
            "string" => 18,
            "number" => 19,
            "regexp" => 20,
            "operator" => 21,
            _ => 8,
        }
    }

    fn gen_token(loc: Location, token_type: SemanticTokenType) -> SemanticToken {
        SemanticToken {
            delta_line: loc.ln_begin().unwrap_or(0),
            delta_start: loc.col_begin().unwrap_or(0),
            length: loc.col_end().unwrap_or(0) - loc.col_begin().unwrap_or(0),
            token_type: Self::token_type_as_u32(token_type),
            token_modifiers_bitset: 0,
        }
    }

    fn gen_from_expr(&mut self, expr: Expr) -> Vec<SemanticToken> {
        match expr {
            Expr::Lit(lit) => {
                let typ = match lit.token.kind {
                    TokenKind::StrLit => SemanticTokenType::STRING,
                    TokenKind::NatLit | TokenKind::IntLit | TokenKind::RatioLit => SemanticTokenType::NUMBER,
                    _ => SemanticTokenType::VARIABLE,
                };
                let token = Self::gen_token(lit.loc(), typ);
                vec![token]
            },
            Expr::Def(def) => self.gen_from_def(def),
            Expr::Lambda(lambda) => self.gen_from_block(Some(lambda.sig.params), lambda.body),
            Expr::ClassDef(classdef) => self.gen_from_classdef(classdef),
            Expr::Accessor(acc) => self.gen_from_acc(acc),
            Expr::Call(call) => self.gen_from_call(call),
            Expr::BinOp(bin) => self.gen_from_bin(bin),
            Expr::UnaryOp(unary) => self.gen_from_unary(unary),
            _ => vec![],
        }
    }

    fn gen_from_def(&mut self, def: Def) -> Vec<SemanticToken> {
        let name = def.sig.ident().map(|id| id.name.to_string()).unwrap_or_else(|| "_".to_string());
        let typ = match def.def_kind() {
            DefKind::Class => SemanticTokenType::CLASS,
            DefKind::Trait => SemanticTokenType::INTERFACE,
            _ if def.is_subr() => SemanticTokenType::FUNCTION,
            _ => SemanticTokenType::VARIABLE,
        };
        self.push_current_namespace(name, typ);
        let mut tokens = vec![];
        let params = def.sig.params();
        tokens.extend(self.gen_from_block(params, def.body.block));
        tokens
    }

    fn gen_from_classdef(&mut self, classdef: ClassDef) -> Vec<SemanticToken> {
        let mut tokens = self.gen_from_def(classdef.def);
        for methods in classdef.methods_list.into_iter() {
            for attr in methods.attrs.into_iter() {
                #[allow(clippy::single_match)]
                match attr {
                    ClassAttr::Def(def) => tokens.extend(self.gen_from_def(def)),
                    _ => {}
                }
            }
        }
        tokens
    }

    fn gen_from_acc(&mut self, acc: Accessor) -> Vec<SemanticToken> {
        match acc {
            Accessor::Ident(ident) => {
                let typ = self.get_variable_type(ident.inspect());
                vec![Self::gen_token(ident.name.loc(), typ)]
            }
            Accessor::Attr(attr) => {
                let mut tokens = self.gen_from_expr(*attr.obj);
                tokens.push(Self::gen_token(attr.ident.name.loc(), SemanticTokenType::PROPERTY));
                tokens
            }
            _ => vec![]
        }
    }

    fn gen_from_call(&mut self, call: Call) -> Vec<SemanticToken> {
        let mut tokens = self.gen_from_expr(*call.obj);
        tokens.extend(self.gen_from_args(call.args));
        tokens
    }

    fn gen_from_args(&mut self, args: Args) -> Vec<SemanticToken> {
        let mut tokens = vec![];
        let (pos_args, kw_args, ..) = args.deconstruct();
        for arg in pos_args {
            tokens.extend(self.gen_from_expr(arg.expr));
        }
        for arg in kw_args {
            tokens.extend(self.gen_from_expr(arg.expr));
        }
        tokens
    }

    fn gen_from_bin(&mut self, bin: BinOp) -> Vec<SemanticToken> {
        let mut args = bin.args.into_iter();
        let mut tokens = self.gen_from_expr(*args.next().unwrap());
        tokens.push(Self::gen_token(bin.op.loc(), SemanticTokenType::OPERATOR));
        tokens.extend(self.gen_from_expr(*args.next().unwrap()));
        tokens
    }

    fn gen_from_unary(&mut self, unary: UnaryOp) -> Vec<SemanticToken> {
        let mut tokens = vec![Self::gen_token(unary.op.loc(), SemanticTokenType::OPERATOR)];
        let mut args = unary.args.into_iter();
        tokens.extend(self.gen_from_expr(*args.next().unwrap()));
        tokens
    }

    fn gen_from_block(&mut self, params: Option<Params>, block: Block) -> Vec<SemanticToken> {
        self.namespaces.push(Dict::new());
        let mut tokens = vec![];
        if let Some(params) = params {
            let (nd_params, var_params, d_params, ..) = params.deconstruct();
            for param in nd_params.into_iter() {
                let typ = SemanticTokenType::PARAMETER;
                tokens.push(Self::gen_token(param.loc(), typ));
            }
            if let Some(var_param) = var_params {
                let typ = SemanticTokenType::PARAMETER;
                tokens.push(Self::gen_token(var_param.loc(), typ));
            }
            for param in d_params.into_iter() {
                let typ = SemanticTokenType::PARAMETER;
                tokens.push(Self::gen_token(param.loc(), typ));
            }
        }
        for expr in block.into_iter() {
            tokens.extend(self.gen_from_expr(expr));
        }
        self.namespaces.pop();
        tokens
    }
}

impl<Checker: BuildRunnable> Server<Checker> {
    pub(crate) fn get_semantic_tokens_full(&mut self, msg: &Value) -> ELSResult<()> {
        Self::send_log(format!("definition requested: {msg}"))?;
        let params = SemanticTokensParams::deserialize(&msg["params"])?;
        let uri = util::normalize_url(params.text_document.uri);
        let path = util::uri_to_path(&uri);
        let src = util::get_code_from_uri(&uri)?;
        let mut builder = ASTBuilder::new(self.cfg.inherit(path));
        let result = match builder.build_without_desugaring(src) {
            Ok(ast) => {
                let mut state = ASTSemanticState::new();
                json!(state.enumerate_tokens(ast))
            },
            Err(_) => json!(null),
        };
        Self::send(
            &json!({ "jsonrpc": "2.0", "id": msg["id"].as_i64().unwrap(), "result": result }),
        )
    }
}
