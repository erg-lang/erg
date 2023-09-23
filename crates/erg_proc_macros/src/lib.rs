use proc_macro::TokenStream;

use quote::quote;
use syn::{PathArguments, ReturnType, Type, TypePath};

/// ```rust_
/// #[exec_new_thread]
/// fn foo() -> Result<isize, Box<dyn std::error::Error>> {
///     ...
/// }
/// ```
/// ↓ ↓
/// ```rust_
/// fn foo() -> Result<isize, Box<dyn std::error::Error>> {
///   fn error(msg: impl Into<String>) -> std::io::Error {
///     std::io::Error::new(std::io::ErrorKind::Other, msg.into())
///   }
///   fn f() -> Result<(), Box<dyn std::error::Error + Send>> {
///     {...}.map_err(|e| Box::new(error(e.to_string())) as _)
///   }
///   exec_new_thread(f, "foo").map_err(|e| e as _)
/// }
/// ```
#[proc_macro_attribute]
pub fn exec_new_thread(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_fn = syn::parse_macro_input!(item as syn::ItemFn);
    let name = item_fn.sig.ident.to_string();
    let ReturnType::Type(_, out) = &item_fn.sig.output else {
        todo!()
    };
    let Type::Path(TypePath { path, .. }) = out.as_ref() else {
        todo!()
    };
    let result_t = path.segments.first().unwrap();
    let PathArguments::AngleBracketed(args) = &result_t.arguments else {
        todo!()
    };
    let t = args.args.first().unwrap();
    let name = syn::LitStr::new(&name, item_fn.sig.ident.span());
    let block = item_fn.block;
    let block = syn::parse_quote! {{
        fn error(msg: impl Into<String>) -> std::io::Error {
            std::io::Error::new(std::io::ErrorKind::Other, msg.into())
        }
        fn _f() -> Result<#t, Box<dyn std::error::Error>> {
            #block
        }
        fn f() -> Result<#t, Box<dyn std::error::Error + Send>> {
            _f().map_err(|e| Box::new(error(e.to_string())) as _)
        }
        erg_common::spawn::exec_new_thread(f, #name).map_err(|e| e as _)
    }};
    item_fn.block = Box::new(block);
    let item = quote! { #item_fn };
    item.into()
}
