use std::path::Path;

use erg_common::spawn::safe_yield;
use lsp_types::{
    CompletionResponse, DiagnosticSeverity, DocumentSymbolResponse, FoldingRange, FoldingRangeKind,
    GotoDefinitionResponse, HoverContents, InlayHintLabel, MarkedString,
};
const FILE_A: &str = "tests/a.er";
const FILE_B: &str = "tests/b.er";
const FILE_C: &str = "tests/c.er";
const FILE_IMPORTS: &str = "tests/imports.er";
const FILE_INVALID_SYNTAX: &str = "tests/invalid_syntax.er";
const FILE_RETRIGGER: &str = "tests/retrigger.er";

use els::{NormalizedUrl, Server};
use erg_proc_macros::exec_new_thread;
use molc::{add_char, delete_line, oneline_range};

#[test]
fn test_open() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    client.wait_messages(3)?;
    client.notify_open(FILE_A)?;
    client.wait_messages(3)?;
    assert!(client.responses.iter().any(|val| val
        .to_string()
        .contains("tests/a.er passed, found warns: 0")));
    Ok(())
}

#[test]
fn test_completion() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    let uri_a = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    let uri_b = NormalizedUrl::from_file_path(Path::new(FILE_B).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    client.notify_change(uri_a.clone().raw(), add_char(2, 0, "x"))?;
    client.notify_change(uri_a.clone().raw(), add_char(2, 1, "."))?;
    let resp = client.request_completion(uri_a.raw(), 2, 2, ".")?;
    if let Some(CompletionResponse::Array(items)) = resp {
        assert!(items.len() >= 40);
        assert!(items.iter().any(|item| item.label == "abs"));
    } else {
        return Err(format!("not items: {resp:?}").into());
    }
    client.notify_open(FILE_B)?;
    client.notify_change(uri_b.clone().raw(), add_char(6, 20, "."))?;
    let resp = client.request_completion(uri_b.raw(), 6, 21, ".")?;
    if let Some(CompletionResponse::Array(items)) = resp {
        assert!(items.iter().any(|item| item.label == "a"));
    } else {
        return Err(format!("not items: {resp:?}").into());
    }
    Ok(())
}

#[test]
fn test_neighbor_completion() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    client.notify_open(FILE_B)?;
    let resp = client.request_completion(uri.raw(), 2, 0, "n")?;
    if let Some(CompletionResponse::Array(items)) = resp {
        assert!(items.len() >= 40);
        assert!(items
            .iter()
            .any(|item| item.label == "neighbor (import from b)"));
        Ok(())
    } else {
        Err(format!("not items: {resp:?}").into())
    }
}

#[test]
fn test_pymodule_completion() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    while !client.server.flags.builtin_modules_loaded() {
        safe_yield();
    }
    client.notify_change(uri.clone().raw(), add_char(2, 0, "c"))?;
    let resp = client.request_completion(uri.raw(), 2, 0, "c")?;
    if let Some(CompletionResponse::Array(items)) = resp {
        assert!(items.len() >= 100);
        assert!(items
            .iter()
            .any(|item| item.label == "cos (import from math)"));
        Ok(())
    } else {
        Err(format!("not items: {resp:?}").into())
    }
}

#[test]
fn test_completion_retrigger() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_RETRIGGER).canonicalize()?)?;
    client.notify_open(FILE_RETRIGGER)?;
    let _ = client.wait_diagnostics()?;
    client.notify_change(uri.clone().raw(), add_char(2, 7, "n"))?;
    let resp = client.request_completion(uri.clone().raw(), 2, 7, "n")?;
    if let Some(CompletionResponse::Array(items)) = resp {
        assert!(!items.is_empty());
        assert!(items.iter().any(|item| item.label == "print!"));
    } else {
        return Err(format!("not items: {resp:?}").into());
    }
    client.notify_change(uri.clone().raw(), add_char(3, 15, "t"))?;
    let resp = client.request_completion(uri.raw(), 3, 15, "t")?;
    if let Some(CompletionResponse::Array(items)) = resp {
        assert!(!items.is_empty());
        assert!(items.iter().any(|item| item.label == "bit_count"));
        assert!(items.iter().any(|item| item.label == "bit_length"));
    } else {
        return Err(format!("not items: {resp:?}").into());
    }
    Ok(())
}

#[test]
#[exec_new_thread]
fn test_rename() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    let edit = client
        .request_rename(uri.clone().raw(), 1, 5, "y")?
        .unwrap();
    assert!(edit
        .changes
        .is_some_and(|changes| changes.values().next().unwrap().len() == 2));
    client.notify_open(FILE_C)?;
    client.notify_open(FILE_B)?;
    let uri_b = NormalizedUrl::from_file_path(Path::new(FILE_B).canonicalize()?)?;
    // let uri_c = NormalizedUrl::from_file_path(Path::new(FILE_C).canonicalize()?)?;
    let edit = client
        .request_rename(uri_b.clone().raw(), 2, 1, "y")?
        .unwrap();
    assert_eq!(edit.changes.as_ref().unwrap().iter().count(), 2);
    for (_, change) in edit.changes.unwrap() {
        assert_eq!(change.len(), 1);
    }
    Ok(())
}

#[test]
fn test_signature_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    client.notify_change(uri.clone().raw(), add_char(2, 0, "assert"))?;
    client.notify_change(uri.clone().raw(), add_char(2, 6, "("))?;
    let help = client
        .request_signature_help(uri.raw(), 2, 7, "(")?
        .unwrap();
    assert_eq!(help.signatures.len(), 1);
    let sig = &help.signatures[0];
    assert_eq!(sig.label, "::assert: (test: Bool, msg := Str) -> NoneType");
    assert_eq!(sig.active_parameter, Some(0));
    Ok(())
}

#[test]
fn test_hover() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    let hover = client.request_hover(uri.raw(), 1, 4)?.unwrap();
    let HoverContents::Array(contents) = hover.contents else {
        todo!()
    };
    assert_eq!(contents.len(), 2);
    let MarkedString::LanguageString(content) = &contents[0] else {
        todo!()
    };
    assert!(
        content.value == "# tests/a.er, line 1\nx = 1"
            || content.value == "# tests\\a.er, line 1\nx = 1"
    );
    let MarkedString::LanguageString(content) = &contents[1] else {
        todo!()
    };
    assert_eq!(content.value, "x: {1}");
    Ok(())
}

#[test]
#[exec_new_thread]
fn test_references() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    let locations = client.request_references(uri.raw(), 1, 4)?.unwrap();
    assert_eq!(locations.len(), 1);
    assert_eq!(&locations[0].range, &oneline_range(1, 4, 5));
    client.notify_open(FILE_C)?;
    client.notify_open(FILE_B)?;
    let uri_b = NormalizedUrl::from_file_path(Path::new(FILE_B).canonicalize()?)?;
    let uri_c = NormalizedUrl::from_file_path(Path::new(FILE_C).canonicalize()?)?;
    let locations = client.request_references(uri_b.raw(), 0, 2)?.unwrap();
    assert_eq!(locations.len(), 1);
    assert_eq!(NormalizedUrl::new(locations[0].uri.clone()), uri_c);
    Ok(())
}

#[test]
fn test_goto_definition() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    let Some(GotoDefinitionResponse::Scalar(location)) =
        client.request_goto_definition(uri.raw(), 1, 4)?
    else {
        todo!()
    };
    assert_eq!(&location.range, &oneline_range(0, 0, 1));
    Ok(())
}

#[test]
#[exec_new_thread]
fn test_folding_range() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_IMPORTS).canonicalize()?)?;
    client.notify_open(FILE_IMPORTS)?;
    let ranges = client.request_folding_range(uri.raw())?.unwrap();
    assert_eq!(ranges.len(), 1);
    assert_eq!(
        &ranges[0],
        &FoldingRange {
            start_line: 0,
            start_character: Some(0),
            end_line: 3,
            end_character: Some(22),
            kind: Some(FoldingRangeKind::Imports),
        }
    );
    Ok(())
}

#[test]
fn test_document_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    let Some(DocumentSymbolResponse::Nested(symbols)) =
        client.request_document_symbols(uri.raw())?
    else {
        todo!()
    };
    assert_eq!(symbols.len(), 2);
    assert_eq!(&symbols[0].name, "x");
    Ok(())
}

#[test]
fn test_inlay_hint() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_A).canonicalize()?)?;
    client.notify_open(FILE_A)?;
    let hints = client.request_inlay_hint(uri.raw())?.unwrap();
    assert_eq!(hints.len(), 2);
    let InlayHintLabel::String(label) = &hints[0].label else {
        todo!()
    };
    assert_eq!(label, ": {1}");
    let InlayHintLabel::String(label) = &hints[1].label else {
        todo!()
    };
    assert_eq!(label, ": Nat");
    Ok(())
}

#[test]
#[exec_new_thread]
fn test_dependents_check() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    client.wait_messages(3)?;
    client.notify_open(FILE_B)?;
    client.wait_messages(4)?;
    client.notify_open(FILE_C)?;
    client.wait_messages(4)?;
    let uri_b = NormalizedUrl::from_file_path(Path::new(FILE_B).canonicalize()?)?;
    client.notify_change(uri_b.clone().raw(), delete_line(2))?;
    client.wait_messages(2)?;
    client.responses.clear();
    client.notify_save(uri_b.clone().raw())?;
    let b_diags = client.wait_diagnostics()?;
    assert!(b_diags.diagnostics.is_empty(), "{:?}", b_diags.diagnostics);
    let c_diags = client.wait_diagnostics()?;
    assert_eq!(
        c_diags.diagnostics.len(),
        1,
        "{:?} / {:?}",
        b_diags.diagnostics,
        c_diags.diagnostics
    );
    assert_eq!(
        c_diags.diagnostics[0].severity,
        Some(DiagnosticSeverity::ERROR)
    );
    Ok(())
}

#[test]
fn test_fix_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Server::bind_fake_client();
    client.request_initialize()?;
    client.notify_initialized()?;
    client.notify_open(FILE_INVALID_SYNTAX)?;
    let diags = client.wait_diagnostics()?;
    assert_eq!(diags.diagnostics.len(), 1);
    assert_eq!(
        diags.diagnostics[0].severity,
        Some(DiagnosticSeverity::ERROR)
    );
    let uri = NormalizedUrl::from_file_path(Path::new(FILE_INVALID_SYNTAX).canonicalize()?)?;
    client.notify_change(uri.clone().raw(), add_char(0, 10, " 1"))?;
    client.notify_save(uri.clone().raw())?;
    let diags = client.wait_diagnostics()?;
    assert_eq!(diags.diagnostics.len(), 0);
    Ok(())
}
