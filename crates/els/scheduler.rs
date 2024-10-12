use core::fmt;
use std::collections::VecDeque;

use lsp_types::request::{
    CallHierarchyIncomingCalls, CallHierarchyOutgoingCalls, CallHierarchyPrepare,
    CodeActionRequest, CodeActionResolveRequest, CodeLensRequest, Completion,
    DocumentSymbolRequest, FoldingRangeRequest, GotoDefinition, GotoImplementation, HoverRequest,
    InlayHintRequest, InlayHintResolveRequest, References, Request, ResolveCompletionItem,
    SemanticTokensFullRequest, SignatureHelpRequest,
};

use erg_common::{shared::Shared, spawn::safe_yield};

type TaskID = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestKind {
    // high-priority requests
    Completion,
    Hover,
    GotoDefinition,
    GotoImplementation,
    SignatureHelp,
    DocumentSymbol,
    CallHierarchy,
    References,
    // middle-priority requests
    InlayHint,
    CodeAction,
    CodeLens,
    // low-priority requests
    FoldingRange,
    CompletionResolve,
    InlayHintResolve,
    CodeActionResolve,
    SemanticTokensFull,
    Other,
}

impl From<&str> for RequestKind {
    fn from(s: &str) -> Self {
        match s {
            Completion::METHOD => Self::Completion,
            HoverRequest::METHOD => Self::Hover,
            GotoDefinition::METHOD => Self::GotoDefinition,
            GotoImplementation::METHOD => Self::GotoImplementation,
            SignatureHelpRequest::METHOD => Self::SignatureHelp,
            DocumentSymbolRequest::METHOD => Self::DocumentSymbol,
            CallHierarchyPrepare::METHOD
            | CallHierarchyIncomingCalls::METHOD
            | CallHierarchyOutgoingCalls::METHOD => Self::CallHierarchy,
            References::METHOD => Self::References,
            InlayHintRequest::METHOD => Self::InlayHint,
            CodeActionRequest::METHOD => Self::CodeAction,
            CodeLensRequest::METHOD => Self::CodeLens,
            FoldingRangeRequest::METHOD => Self::FoldingRange,
            ResolveCompletionItem::METHOD => Self::CompletionResolve,
            InlayHintResolveRequest::METHOD => Self::InlayHintResolve,
            CodeActionResolveRequest::METHOD => Self::CodeActionResolve,
            SemanticTokensFullRequest::METHOD => Self::SemanticTokensFull,
            _ => Self::Other,
        }
    }
}

impl RequestKind {
    pub fn priority(&self) -> u8 {
        match self {
            Self::Completion => 0,
            Self::Hover => 5,
            Self::GotoDefinition => 10,
            Self::GotoImplementation => 15,
            Self::SignatureHelp => 20,
            Self::DocumentSymbol => 25,
            Self::CallHierarchy => 30,
            Self::References => 35,
            Self::InlayHint => 40,
            Self::CodeAction => 45,
            Self::CodeLens => 50,
            Self::FoldingRange => 55,
            Self::CompletionResolve => 60,
            Self::InlayHintResolve => 65,
            Self::CodeActionResolve => 70,
            Self::SemanticTokensFull => 75,
            Self::Other => 255,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Task {
    kind: RequestKind,
    id: TaskID,
}

/// This pauses processing of tasks when a large number of requests are received from clients to reduce the load,
/// and discards tasks with cancellation requests.
#[derive(Clone, Debug, Default)]
pub struct Scheduler {
    pending: Shared<VecDeque<Task>>,
    executing: Shared<Vec<Task>>,
}

impl fmt::Display for Scheduler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pending = self.pending.borrow();
        let executing = self.executing.borrow();
        write!(
            f,
            "Scheduler {{ pending: {}, executing: {} }}",
            pending.len(),
            executing.len()
        )
    }
}

pub const MAX_WORKERS: usize = 10;

impl Scheduler {
    pub fn new() -> Self {
        Self {
            pending: Shared::new(VecDeque::with_capacity(10)),
            executing: Shared::new(Vec::with_capacity(10)),
        }
    }

    pub fn register(&self, id: TaskID, method: &str) {
        let request = RequestKind::from(method);
        let task = Task { kind: request, id };
        if self.executing.borrow().len() < MAX_WORKERS {
            self.executing.borrow_mut().push(task);
        } else {
            self.pending.borrow_mut().push_back(task);
        }
    }

    /// Blocks until the task is allowed to be executed.
    /// `None` means that the task has already been cancelled.
    pub fn acquire(&self, id: TaskID) -> Option<Task> {
        if let Some(idx) = self.executing.borrow().iter().find(|task| task.id == id) {
            Some(*idx)
        } else {
            let task = self
                .pending
                .borrow()
                .iter()
                .find(|task| task.id == id)
                .copied()?;
            loop {
                if self.executing.borrow().len() < MAX_WORKERS
                    && self
                        .pending
                        .borrow()
                        .iter()
                        .all(|t| t.kind.priority() >= task.kind.priority())
                {
                    break;
                } else {
                    safe_yield();
                }
            }
            let idx = self
                .pending
                .borrow()
                .iter()
                .position(|task| task.id == id)?;
            let task = self.pending.borrow_mut().remove(idx)?;
            self.executing.borrow_mut().push(task);
            Some(task)
        }
    }

    /// Only pending tasks can be cancelled
    /// TODO: cancel executing tasks
    pub fn cancel(&self, id: TaskID) -> Option<Task> {
        let mut lock = self.pending.borrow_mut();
        let idx = lock.iter().position(|task| task.id == id)?;
        lock.remove(idx)
    }

    pub fn finish(&self, id: TaskID) -> Option<Task> {
        let mut lock = self.executing.borrow_mut();
        let idx = lock.iter().position(|task| task.id == id)?;
        Some(lock.remove(idx))
    }
}
