use erg_common::config::ErgConfig;
use erg_common::error::MultiErrorDisplay;
use erg_common::io::Output;
use erg_common::spawn::exec_new_thread;
use erg_common::traits::Runnable;

use erg_compiler::context::{Context, ModuleContext};
use erg_compiler::error::CompileErrors;
use erg_compiler::lower::ASTLowerer;

use erg_compiler::ty::constructors::{
    func0, func1, func2, kw, mono, nd_func, nd_proc, or, poly, proc1, subtype_q, ty_tp, type_q,
};
use erg_compiler::ty::Type::*;

fn load_file(path: &'static str) -> Result<ModuleContext, CompileErrors> {
    let mut cfg = ErgConfig::with_main_path(path.into());
    cfg.output = Output::Null;
    let mut lowerer = ASTLowerer::new(cfg);
    lowerer.exec()?;
    Ok(lowerer.pop_mod_ctx().unwrap())
}

#[test]
fn test_infer_types() -> Result<(), ()> {
    exec_new_thread(_test_infer_types, "test_infer_types")
}

fn _test_infer_types() -> Result<(), ()> {
    let module = load_file("tests/infer.er").map_err(|errs| {
        errs.write_all_stderr();
    })?;
    let t = type_q("T");
    let u = type_q("U");
    let id_t = func1(t.clone(), t.clone()).quantify();
    module.context.assert_var_type("id", &id_t)?;
    let tu = or(t.clone(), u.clone());
    let if_t = nd_func(
        vec![
            kw("cond", Bool),
            kw("then", func0(t.clone())),
            kw("else", func0(u)),
        ],
        None,
        tu,
    )
    .quantify();
    module.context.assert_var_type("if__", &if_t)?;
    let for_t = nd_proc(
        vec![
            kw("i", poly("Iterable", vec![ty_tp(t.clone())])),
            kw("proc!", proc1(t.clone(), NoneType)),
        ],
        None,
        NoneType,
    )
    .quantify();
    module.context.assert_var_type("for__!", &for_t)?;
    let a = subtype_q("A", poly("Add", vec![ty_tp(t.clone())]));
    let o = a.clone().proj("Output");
    let add_t = func2(a, t, o).quantify();
    module.context.assert_var_type("add", &add_t)?;
    let abs_t = func1(Int, Nat);
    module.context.assert_var_type("abs_", &abs_t)?;
    let norm_t = func1(mono("<module>::Norm"), Nat);
    module.context.assert_var_type("norm", &norm_t)?;
    Ok(())
}

#[test]
fn test_refinement_subtyping() -> Result<(), ()> {
    let context = Context::default_with_name("<module>");
    context.test_refinement_subtyping()?;
    Ok(())
}

#[test]
fn test_quant_subtyping() -> Result<(), ()> {
    let context = Context::default_with_name("<module>");
    context.test_quant_subtyping()?;
    Ok(())
}

#[test]
fn test_instantiation_and_generalization() -> Result<(), ()> {
    let context = Context::default_with_name("<module>");
    context.test_instantiation_and_generalization()?;
    Ok(())
}

#[test]
fn test_intersection() -> Result<(), ()> {
    let context = Context::default_with_name("<module>");
    context.test_intersection()?;
    Ok(())
}

/*
#[test]
fn test_patch() -> Result<(), ()> {
    let shared = SharedCompilerResource::new(ErgConfig::default());
    shared.mod_cache.get(Path::new("<builtins>")).unwrap().module.context.test_patch()?;
    Ok(())
}

#[test]
fn test_resolve_trait() -> Result<(), ()> {
    let context = Context::new_main_module();
    context.test_resolve_trait()?;
    Ok(())
}

#[test]
fn test_resolve_trait_inner1() -> Result<(), ()> {
    let context = Context::new_main_module();
    context.test_resolve_trait_inner1()?;
    Ok(())
}
*/

// #[test]
fn _test_dir() -> Result<(), ()> {
    let context = Context::default_with_name("<module>");
    let vars = context.dir();
    for (name, vi) in vars.into_iter() {
        println!("{name}: {vi}");
    }
    Ok(())
}
