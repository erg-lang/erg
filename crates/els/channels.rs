use std::sync::mpsc;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::parse::Parsable;

use lsp_types::request::{
    CallHierarchyIncomingCalls, CallHierarchyOutgoingCalls, CallHierarchyPrepare,
    CodeActionRequest, CodeActionResolveRequest, CodeLensRequest, Completion,
    DocumentSymbolRequest, ExecuteCommand, FoldingRangeRequest, GotoDefinition, GotoImplementation,
    GotoImplementationParams, HoverRequest, InlayHintRequest, InlayHintResolveRequest, References,
    ResolveCompletionItem, SemanticTokensFullRequest, SignatureHelpRequest, WillRenameFiles,
    WorkspaceSymbol,
};
use lsp_types::{
    CallHierarchyIncomingCallsParams, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    CodeAction, CodeActionParams, CodeLensParams, CompletionItem, CompletionParams,
    DocumentSymbolParams, ExecuteCommandParams, FoldingRangeParams, GotoDefinitionParams,
    HoverParams, InlayHint, InlayHintParams, ReferenceParams, RenameFilesParams,
    SemanticTokensParams, SignatureHelpParams, WorkspaceSymbolParams,
};

use crate::server::Server;

#[derive(Debug, Clone)]
pub enum WorkerMessage<P> {
    Request(i64, P),
    Kill,
}

impl<P> From<(i64, P)> for WorkerMessage<P> {
    fn from((id, params): (i64, P)) -> Self {
        Self::Request(id, params)
    }
}

#[derive(Debug, Clone)]
pub struct SendChannels {
    completion: mpsc::Sender<WorkerMessage<CompletionParams>>,
    resolve_completion: mpsc::Sender<WorkerMessage<CompletionItem>>,
    goto_definition: mpsc::Sender<WorkerMessage<GotoDefinitionParams>>,
    goto_implementation: mpsc::Sender<WorkerMessage<GotoImplementationParams>>,
    semantic_tokens_full: mpsc::Sender<WorkerMessage<SemanticTokensParams>>,
    inlay_hint: mpsc::Sender<WorkerMessage<InlayHintParams>>,
    inlay_hint_resolve: mpsc::Sender<WorkerMessage<InlayHint>>,
    hover: mpsc::Sender<WorkerMessage<HoverParams>>,
    references: mpsc::Sender<WorkerMessage<ReferenceParams>>,
    code_lens: mpsc::Sender<WorkerMessage<CodeLensParams>>,
    code_action: mpsc::Sender<WorkerMessage<CodeActionParams>>,
    code_action_resolve: mpsc::Sender<WorkerMessage<CodeAction>>,
    signature_help: mpsc::Sender<WorkerMessage<SignatureHelpParams>>,
    will_rename_files: mpsc::Sender<WorkerMessage<RenameFilesParams>>,
    execute_command: mpsc::Sender<WorkerMessage<ExecuteCommandParams>>,
    workspace_symbol: mpsc::Sender<WorkerMessage<WorkspaceSymbolParams>>,
    document_symbol: mpsc::Sender<WorkerMessage<DocumentSymbolParams>>,
    call_hierarchy_prepare: mpsc::Sender<WorkerMessage<CallHierarchyPrepareParams>>,
    call_hierarchy_incoming: mpsc::Sender<WorkerMessage<CallHierarchyIncomingCallsParams>>,
    call_hierarchy_outgoing: mpsc::Sender<WorkerMessage<CallHierarchyOutgoingCallsParams>>,
    folding_range: mpsc::Sender<WorkerMessage<FoldingRangeParams>>,
    pub(crate) health_check: mpsc::Sender<WorkerMessage<()>>,
}

impl SendChannels {
    pub fn new() -> (Self, ReceiveChannels) {
        let (tx_completion, rx_completion) = mpsc::channel();
        let (tx_resolve_completion, rx_resolve_completion) = mpsc::channel();
        let (tx_goto_definition, rx_goto_definition) = mpsc::channel();
        let (tx_goto_implementation, rx_goto_implementation) = mpsc::channel();
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
        let (tx_workspace_symbol, rx_workspace_symbol) = mpsc::channel();
        let (tx_document_symbol, rx_document_symbol) = mpsc::channel();
        let (tx_call_hierarchy_prepare, rx_call_hierarchy_prepare) = mpsc::channel();
        let (tx_call_hierarchy_incoming, rx_call_hierarchy_incoming) = mpsc::channel();
        let (tx_call_hierarchy_outgoing, rx_call_hierarchy_outgoing) = mpsc::channel();
        let (tx_foldging_range, rx_folding_range) = mpsc::channel();
        let (tx_health_check, rx_health_check) = mpsc::channel();
        (
            Self {
                completion: tx_completion,
                resolve_completion: tx_resolve_completion,
                goto_definition: tx_goto_definition,
                goto_implementation: tx_goto_implementation,
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
                workspace_symbol: tx_workspace_symbol,
                document_symbol: tx_document_symbol,
                call_hierarchy_prepare: tx_call_hierarchy_prepare,
                call_hierarchy_incoming: tx_call_hierarchy_incoming,
                call_hierarchy_outgoing: tx_call_hierarchy_outgoing,
                folding_range: tx_foldging_range,
                health_check: tx_health_check,
            },
            ReceiveChannels {
                completion: rx_completion,
                resolve_completion: rx_resolve_completion,
                goto_definition: rx_goto_definition,
                goto_implementation: rx_goto_implementation,
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
                workspace_symbol: rx_workspace_symbol,
                document_symbol: rx_document_symbol,
                call_hierarchy_prepare: rx_call_hierarchy_prepare,
                call_hierarchy_incoming: rx_call_hierarchy_incoming,
                call_hierarchy_outgoing: rx_call_hierarchy_outgoing,
                folding_range: rx_folding_range,
                health_check: rx_health_check,
            },
        )
    }

    pub(crate) fn close(&self) {
        self.completion.send(WorkerMessage::Kill).unwrap();
        self.resolve_completion.send(WorkerMessage::Kill).unwrap();
        self.goto_definition.send(WorkerMessage::Kill).unwrap();
        self.semantic_tokens_full.send(WorkerMessage::Kill).unwrap();
        self.inlay_hint.send(WorkerMessage::Kill).unwrap();
        self.inlay_hint_resolve.send(WorkerMessage::Kill).unwrap();
        self.hover.send(WorkerMessage::Kill).unwrap();
        self.references.send(WorkerMessage::Kill).unwrap();
        self.code_lens.send(WorkerMessage::Kill).unwrap();
        self.code_action.send(WorkerMessage::Kill).unwrap();
        self.code_action_resolve.send(WorkerMessage::Kill).unwrap();
        self.signature_help.send(WorkerMessage::Kill).unwrap();
        self.will_rename_files.send(WorkerMessage::Kill).unwrap();
        self.execute_command.send(WorkerMessage::Kill).unwrap();
        self.workspace_symbol.send(WorkerMessage::Kill).unwrap();
        self.document_symbol.send(WorkerMessage::Kill).unwrap();
        self.call_hierarchy_prepare
            .send(WorkerMessage::Kill)
            .unwrap();
        self.call_hierarchy_incoming
            .send(WorkerMessage::Kill)
            .unwrap();
        self.call_hierarchy_outgoing
            .send(WorkerMessage::Kill)
            .unwrap();
        self.folding_range.send(WorkerMessage::Kill).unwrap();
        self.health_check.send(WorkerMessage::Kill).unwrap();
    }
}

#[derive(Debug)]
pub struct ReceiveChannels {
    pub(crate) completion: mpsc::Receiver<WorkerMessage<CompletionParams>>,
    pub(crate) resolve_completion: mpsc::Receiver<WorkerMessage<CompletionItem>>,
    pub(crate) goto_definition: mpsc::Receiver<WorkerMessage<GotoDefinitionParams>>,
    pub(crate) goto_implementation: mpsc::Receiver<WorkerMessage<GotoImplementationParams>>,
    pub(crate) semantic_tokens_full: mpsc::Receiver<WorkerMessage<SemanticTokensParams>>,
    pub(crate) inlay_hint: mpsc::Receiver<WorkerMessage<InlayHintParams>>,
    pub(crate) inlay_hint_resolve: mpsc::Receiver<WorkerMessage<InlayHint>>,
    pub(crate) hover: mpsc::Receiver<WorkerMessage<HoverParams>>,
    pub(crate) references: mpsc::Receiver<WorkerMessage<ReferenceParams>>,
    pub(crate) code_lens: mpsc::Receiver<WorkerMessage<CodeLensParams>>,
    pub(crate) code_action: mpsc::Receiver<WorkerMessage<CodeActionParams>>,
    pub(crate) code_action_resolve: mpsc::Receiver<WorkerMessage<CodeAction>>,
    pub(crate) signature_help: mpsc::Receiver<WorkerMessage<SignatureHelpParams>>,
    pub(crate) will_rename_files: mpsc::Receiver<WorkerMessage<RenameFilesParams>>,
    pub(crate) execute_command: mpsc::Receiver<WorkerMessage<ExecuteCommandParams>>,
    pub(crate) workspace_symbol: mpsc::Receiver<WorkerMessage<WorkspaceSymbolParams>>,
    pub(crate) document_symbol: mpsc::Receiver<WorkerMessage<DocumentSymbolParams>>,
    pub(crate) call_hierarchy_prepare: mpsc::Receiver<WorkerMessage<CallHierarchyPrepareParams>>,
    pub(crate) call_hierarchy_incoming:
        mpsc::Receiver<WorkerMessage<CallHierarchyIncomingCallsParams>>,
    pub(crate) call_hierarchy_outgoing:
        mpsc::Receiver<WorkerMessage<CallHierarchyOutgoingCallsParams>>,
    pub(crate) folding_range: mpsc::Receiver<WorkerMessage<FoldingRangeParams>>,
    pub(crate) health_check: mpsc::Receiver<WorkerMessage<()>>,
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
                    .send($crate::channels::WorkerMessage::Request(id, params))
                    .unwrap();
            }
        }
    };
}

impl_sendable!(Completion, CompletionParams, completion);
impl_sendable!(ResolveCompletionItem, CompletionItem, resolve_completion);
impl_sendable!(GotoDefinition, GotoDefinitionParams, goto_definition);
impl_sendable!(
    GotoImplementation,
    GotoImplementationParams,
    goto_implementation
);
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
impl_sendable!(WorkspaceSymbol, WorkspaceSymbolParams, workspace_symbol);
impl_sendable!(DocumentSymbolRequest, DocumentSymbolParams, document_symbol);
impl_sendable!(
    CallHierarchyPrepare,
    CallHierarchyPrepareParams,
    call_hierarchy_prepare
);
impl_sendable!(
    CallHierarchyIncomingCalls,
    CallHierarchyIncomingCallsParams,
    call_hierarchy_incoming
);
impl_sendable!(
    CallHierarchyOutgoingCalls,
    CallHierarchyOutgoingCallsParams,
    call_hierarchy_outgoing
);
impl_sendable!(FoldingRangeRequest, FoldingRangeParams, folding_range);
