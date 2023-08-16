use std::sync::mpsc;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::parse::Parsable;

use lsp_types::request::{
    CodeActionRequest, CodeActionResolveRequest, CodeLensRequest, Completion, ExecuteCommand,
    GotoDefinition, HoverRequest, InlayHintRequest, InlayHintResolveRequest, References,
    ResolveCompletionItem, SemanticTokensFullRequest, SignatureHelpRequest, WillRenameFiles,
};
use lsp_types::{
    CodeAction, CodeActionParams, CodeLensParams, CompletionItem, CompletionParams,
    ExecuteCommandParams, GotoDefinitionParams, HoverParams, InlayHint, InlayHintParams,
    ReferenceParams, RenameFilesParams, SemanticTokensParams, SignatureHelpParams,
};

use crate::server::Server;

#[derive(Debug, Clone)]
pub struct SendChannels {
    completion: mpsc::Sender<(i64, CompletionParams)>,
    resolve_completion: mpsc::Sender<(i64, CompletionItem)>,
    goto_definition: mpsc::Sender<(i64, GotoDefinitionParams)>,
    semantic_tokens_full: mpsc::Sender<(i64, SemanticTokensParams)>,
    inlay_hint: mpsc::Sender<(i64, InlayHintParams)>,
    inlay_hint_resolve: mpsc::Sender<(i64, InlayHint)>,
    hover: mpsc::Sender<(i64, HoverParams)>,
    references: mpsc::Sender<(i64, ReferenceParams)>,
    code_lens: mpsc::Sender<(i64, CodeLensParams)>,
    code_action: mpsc::Sender<(i64, CodeActionParams)>,
    code_action_resolve: mpsc::Sender<(i64, CodeAction)>,
    signature_help: mpsc::Sender<(i64, SignatureHelpParams)>,
    will_rename_files: mpsc::Sender<(i64, RenameFilesParams)>,
    execute_command: mpsc::Sender<(i64, ExecuteCommandParams)>,
}

impl SendChannels {
    pub fn new() -> (Self, ReceiveChannels) {
        let (tx_completion, rx_completion) = mpsc::channel();
        let (tx_resolve_completion, rx_resolve_completion) = mpsc::channel();
        let (tx_goto_definition, rx_goto_definition) = mpsc::channel();
        let (tx_semantic_tokens_full, rx_semantic_tokens_full) = mpsc::channel();
        let (tx_inlay_hint, rx_inlay_hint) = mpsc::channel();
        let (tx_inlay_hint_resolve, rx_inlay_hint_resolve) = mpsc::channel();
        let (tx_hover, rx_hover) = mpsc::channel();
        let (tx_references, rx_references) = mpsc::channel();
        let (tx_code_lens, rx_code_lens) = mpsc::channel();
        let (tx_code_action, rx_code_action) = mpsc::channel();
        let (tx_code_action_resolve, rx_code_action_resolve) = mpsc::channel();
        let (tx_sig_help, rx_sig_help) = mpsc::channel();
        let (tx_will_rename_files, rx_will_rename_files) = mpsc::channel();
        let (tx_execute_command, rx_execute_command) = mpsc::channel();
        (
            Self {
                completion: tx_completion,
                resolve_completion: tx_resolve_completion,
                goto_definition: tx_goto_definition,
                semantic_tokens_full: tx_semantic_tokens_full,
                inlay_hint: tx_inlay_hint,
                inlay_hint_resolve: tx_inlay_hint_resolve,
                hover: tx_hover,
                references: tx_references,
                code_lens: tx_code_lens,
                code_action: tx_code_action,
                code_action_resolve: tx_code_action_resolve,
                signature_help: tx_sig_help,
                will_rename_files: tx_will_rename_files,
                execute_command: tx_execute_command,
            },
            ReceiveChannels {
                completion: rx_completion,
                resolve_completion: rx_resolve_completion,
                goto_definition: rx_goto_definition,
                semantic_tokens_full: rx_semantic_tokens_full,
                inlay_hint: rx_inlay_hint,
                inlay_hint_resolve: rx_inlay_hint_resolve,
                hover: rx_hover,
                references: rx_references,
                code_lens: rx_code_lens,
                code_action: rx_code_action,
                code_action_resolve: rx_code_action_resolve,
                signature_help: rx_sig_help,
                will_rename_files: rx_will_rename_files,
                execute_command: rx_execute_command,
            },
        )
    }
}

#[derive(Debug)]
pub struct ReceiveChannels {
    pub(crate) completion: mpsc::Receiver<(i64, CompletionParams)>,
    pub(crate) resolve_completion: mpsc::Receiver<(i64, CompletionItem)>,
    pub(crate) goto_definition: mpsc::Receiver<(i64, GotoDefinitionParams)>,
    pub(crate) semantic_tokens_full: mpsc::Receiver<(i64, SemanticTokensParams)>,
    pub(crate) inlay_hint: mpsc::Receiver<(i64, InlayHintParams)>,
    pub(crate) inlay_hint_resolve: mpsc::Receiver<(i64, InlayHint)>,
    pub(crate) hover: mpsc::Receiver<(i64, HoverParams)>,
    pub(crate) references: mpsc::Receiver<(i64, ReferenceParams)>,
    pub(crate) code_lens: mpsc::Receiver<(i64, CodeLensParams)>,
    pub(crate) code_action: mpsc::Receiver<(i64, CodeActionParams)>,
    pub(crate) code_action_resolve: mpsc::Receiver<(i64, CodeAction)>,
    pub(crate) signature_help: mpsc::Receiver<(i64, SignatureHelpParams)>,
    pub(crate) will_rename_files: mpsc::Receiver<(i64, RenameFilesParams)>,
    pub(crate) execute_command: mpsc::Receiver<(i64, ExecuteCommandParams)>,
}

pub trait Sendable<R: lsp_types::request::Request + 'static> {
    fn send(&self, id: i64, params: R::Params);
}

macro_rules! impl_sendable {
    ($Request: ident, $Params: ident, $receiver: ident) => {
        impl<Checker: BuildRunnable, Parser: Parsable> Sendable<$Request>
            for Server<Checker, Parser>
        {
            fn send(&self, id: i64, params: $Params) {
                self.channels
                    .as_ref()
                    .unwrap()
                    .$receiver
                    .send((id, params))
                    .unwrap();
            }
        }
    };
}

impl_sendable!(Completion, CompletionParams, completion);
impl_sendable!(ResolveCompletionItem, CompletionItem, resolve_completion);
impl_sendable!(GotoDefinition, GotoDefinitionParams, goto_definition);
impl_sendable!(
    SemanticTokensFullRequest,
    SemanticTokensParams,
    semantic_tokens_full
);
impl_sendable!(InlayHintRequest, InlayHintParams, inlay_hint);
impl_sendable!(InlayHintResolveRequest, InlayHint, inlay_hint_resolve);
impl_sendable!(HoverRequest, HoverParams, hover);
impl_sendable!(References, ReferenceParams, references);
impl_sendable!(CodeLensRequest, CodeLensParams, code_lens);
impl_sendable!(CodeActionRequest, CodeActionParams, code_action);
impl_sendable!(CodeActionResolveRequest, CodeAction, code_action_resolve);
impl_sendable!(SignatureHelpRequest, SignatureHelpParams, signature_help);
impl_sendable!(WillRenameFiles, RenameFilesParams, will_rename_files);
impl_sendable!(ExecuteCommand, ExecuteCommandParams, execute_command);
