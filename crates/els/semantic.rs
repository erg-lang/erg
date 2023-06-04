use erg_common::dict::Dict;
use erg_common::error::Location;
use erg_common::traits::{Locational, Runnable};

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::ast::{
    Accessor, Args, BinOp, Block, Call, ClassAttr, Def, DefKind, Expr, Identifier, Methods, Params,
    PolyTypeSpec, PreDeclTypeSpec, TypeSpec, UnaryOp, AST,
};
use erg_compiler::erg_parser::parse::Parsable;
use erg_compiler::erg_parser::token::TokenKind;
use erg_compiler::ASTBuilder;

use lsp_types::{
    SemanticToken, SemanticTokenType, SemanticTokens, SemanticTokensParams, SemanticTokensResult,
};

use crate::server::{send_log, ELSResult, Server};
use crate::util::{self, NormalizedUrl};

#[derive(Debug)]
struct ASTSemanticState {
    prev_line: u32,
    prev_col: u32,
    namespaces: Vec<Dict<String, SemanticTokenType>>,
    tokens: Vec<SemanticToken>,
}

impl ASTSemanticState {
    fn new() -> Self {
        Self {
            prev_line: 1,
            prev_col: 0,
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

    fn gen_token(&mut self, loc: Location, token_type: SemanticTokenType) -> SemanticToken {
        let delta_line = loc.ln_begin().unwrap_or(1).saturating_sub(self.prev_line);
        let delta_start = if delta_line == 0 {
            loc.col_begin().unwrap_or(0).saturating_sub(self.prev_col)
        } else {
            loc.col_begin().unwrap_or(0)
        };
        let token = SemanticToken {
            delta_line,
            delta_start,
            length: loc.length().unwrap_or(1),
            token_type: Self::token_type_as_u32(token_type),
            token_modifiers_bitset: 0,
        };
        self.prev_line = loc.ln_begin().unwrap_or(self.prev_line);
        self.prev_col = loc.col_begin().unwrap_or(self.prev_col);
        token
    }

    fn gen_from_typespec(&mut self, t_spec: TypeSpec) -> Vec<SemanticToken> {
        match t_spec {
            TypeSpec::PreDeclTy(predecl) => match predecl {
                PreDeclTypeSpec::Mono(ident) => self.gen_from_ident(ident),
                PreDeclTypeSpec::Poly(poly) => self.gen_from_poly_typespec(poly),
                PreDeclTypeSpec::Attr { namespace, t } => {
                    let mut tokens = self.gen_from_expr(*namespace);
                    let ts = self.gen_from_ident(t);
                    tokens.extend(ts);
                    tokens
                }
                _ => vec![],
            },
            _ => vec![],
        }
    }

    fn gen_from_poly_typespec(&mut self, t_spec: PolyTypeSpec) -> Vec<SemanticToken> {
        let mut tokens = vec![];
        let token = self.gen_token(t_spec.acc.loc(), SemanticTokenType::TYPE);
        tokens.push(token);
        tokens
    }

    fn gen_from_expr(&mut self, expr: Expr) -> Vec<SemanticToken> {
        match expr {
            Expr::Literal(lit) => {
                let typ = match lit.token.kind {
                    TokenKind::StrLit => SemanticTokenType::STRING,
                    TokenKind::NatLit | TokenKind::IntLit | TokenKind::RatioLit => {
                        SemanticTokenType::NUMBER
                    }
                    _ => SemanticTokenType::VARIABLE,
                };
                let token = self.gen_token(lit.loc(), typ);
                vec![token]
            }
            Expr::Def(def) => self.gen_from_def(def),
            Expr::Lambda(lambda) => self.gen_from_block(Some(lambda.sig.params), lambda.body),
            Expr::Methods(methods) => self.gen_from_methods(methods),
            Expr::Accessor(acc) => self.gen_from_acc(acc),
            Expr::Call(call) => self.gen_from_call(call),
            Expr::BinOp(bin) => self.gen_from_bin(bin),
            Expr::UnaryOp(unary) => self.gen_from_unary(unary),
            _ => vec![],
        }
    }

    fn gen_from_def(&mut self, def: Def) -> Vec<SemanticToken> {
        let mut tokens = vec![];
        let (_loc, name) = def
            .sig
            .ident()
            .map(|id| (Some(id.name.loc()), id.name.to_string()))
            .unwrap_or_else(|| (None, "_".to_string()));
        let typ = match def.def_kind() {
            DefKind::Class => SemanticTokenType::CLASS,
            DefKind::Trait => SemanticTokenType::INTERFACE,
            _ if def.is_subr() => SemanticTokenType::FUNCTION,
            _ => SemanticTokenType::VARIABLE,
        };
        self.push_current_namespace(name, typ);
        if let Some(decos) = def.sig.decorators() {
            for deco in decos.iter() {
                tokens.extend(self.gen_from_expr(deco.expr().clone()));
            }
        }
        // HACK: the cause is unknown, but pushing _loc will break the order of tokens
        /*if let Some(loc) = loc {
            tokens.push(self.gen_token(loc, typ));
        }
        if let Some(t_spec) = def.sig.t_spec() {
            tokens.extend(self.gen_from_typespec(t_spec.clone()));
        }*/
        let params = def.sig.params();
        tokens.extend(self.gen_from_block(params, def.body.block));
        tokens
    }

    fn gen_from_methods(&mut self, methods: Methods) -> Vec<SemanticToken> {
        let mut tokens = vec![];
        tokens.extend(self.gen_from_typespec(methods.class));
        for attr in methods.attrs.into_iter() {
            #[allow(clippy::single_match)]
            match attr {
                ClassAttr::Def(def) => tokens.extend(self.gen_from_def(def)),
                _ => {}
            }
        }
        tokens
    }

    fn gen_from_ident(&mut self, ident: Identifier) -> Vec<SemanticToken> {
        let typ = self.get_variable_type(ident.inspect());
        vec![self.gen_token(ident.name.loc(), typ)]
    }

    fn gen_from_acc(&mut self, acc: Accessor) -> Vec<SemanticToken> {
        match acc {
            Accessor::Ident(ident) => self.gen_from_ident(ident),
            Accessor::Attr(attr) => {
                let mut tokens = self.gen_from_expr(*attr.obj);
                tokens.push(self.gen_token(attr.ident.name.loc(), SemanticTokenType::PROPERTY));
                tokens
            }
            _ => vec![],
        }
    }

    fn gen_from_call(&mut self, call: Call) -> Vec<SemanticToken> {
        let mut tokens = self.gen_from_expr(*call.obj);
        tokens.extend(self.gen_from_args(call.args));
        tokens
    }

    fn gen_from_args(&mut self, args: Args) -> Vec<SemanticToken> {
        let mut tokens = vec![];
        let (pos_args, var_args, kw_args, ..) = args.deconstruct();
        for arg in pos_args {
            tokens.extend(self.gen_from_expr(arg.expr));
        }
        if let Some(var_args) = var_args {
            tokens.extend(self.gen_from_expr(var_args.expr));
        }
        for arg in kw_args {
            tokens.extend(self.gen_from_expr(arg.expr));
        }
        tokens
    }

    fn gen_from_bin(&mut self, bin: BinOp) -> Vec<SemanticToken> {
        let mut args = bin.args.into_iter();
        let mut tokens = self.gen_from_expr(*args.next().unwrap());
        tokens.push(self.gen_token(bin.op.loc(), SemanticTokenType::OPERATOR));
        tokens.extend(self.gen_from_expr(*args.next().unwrap()));
        tokens
    }

    fn gen_from_unary(&mut self, unary: UnaryOp) -> Vec<SemanticToken> {
        let mut tokens = vec![self.gen_token(unary.op.loc(), SemanticTokenType::OPERATOR)];
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
                tokens.push(self.gen_token(param.loc(), typ));
            }
            if let Some(var_param) = var_params {
                let typ = SemanticTokenType::PARAMETER;
                tokens.push(self.gen_token(var_param.loc(), typ));
            }
            for param in d_params.into_iter() {
                let typ = SemanticTokenType::PARAMETER;
                tokens.push(self.gen_token(param.loc(), typ));
            }
        }
        for expr in block.into_iter() {
            tokens.extend(self.gen_from_expr(expr));
        }
        self.namespaces.pop();
        tokens
    }
}

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn handle_semantic_tokens_full(
        &mut self,
        params: SemanticTokensParams,
    ) -> ELSResult<Option<SemanticTokensResult>> {
        send_log(format!("full semantic tokens request: {params:?}"))?;
        let uri = NormalizedUrl::new(params.text_document.uri);
        let path = util::uri_to_path(&uri);
        let src = self.file_cache.get_entire_code(&uri)?;
        let mut builder = ASTBuilder::new(self.cfg.inherit(path));
        let result = match builder.build_without_desugaring(src) {
            Ok(ast) => {
                let mut state = ASTSemanticState::new();
                let tokens = state.enumerate_tokens(ast);
                Some(SemanticTokensResult::Tokens(tokens))
            }
            Err(_) => None,
        };
        Ok(result)
    }
}
