use proc_macro::TokenStream;

use quote::quote;
use syn::{
    punctuated::Punctuated, AngleBracketedGenericArguments, GenericArgument, PathArguments,
    ReturnType, Type, TypePath, TypeReference, TypeSlice,
};

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

/// dummy attribute
#[proc_macro_attribute]
pub fn pyo3(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// dummy attribute
#[proc_macro_attribute]
pub fn pyclass(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// dummy attribute
#[proc_macro_attribute]
pub fn pymethods(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// dummy attribute
#[proc_macro_attribute]
pub fn staticmethod(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// dummy attribute
#[proc_macro_attribute]
pub fn classmethod(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// dummy attribute
#[proc_macro_attribute]
pub fn getter(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// dummy attribute
#[proc_macro_attribute]
pub fn setter(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

fn args_to_owned(args: &PathArguments) -> PathArguments {
    match args {
        PathArguments::AngleBracketed(args) => {
            let res = args
                .args
                .iter()
                .map(|arg| match arg {
                    GenericArgument::Type(t) => GenericArgument::Type(type_to_owned(t)),
                    _ => arg.clone(),
                })
                .collect::<Vec<_>>();
            let mut punc = Punctuated::new();
            punc.extend(res);
            let args = AngleBracketedGenericArguments {
                colon2_token: args.colon2_token,
                lt_token: args.lt_token,
                args: punc,
                gt_token: args.gt_token,
            };
            PathArguments::AngleBracketed(args)
        }
        _ => args.clone(),
    }
}

fn type_to_owned(t: &Type) -> Type {
    match t {
        Type::Reference(TypeReference { elem, .. }) => match elem.as_ref() {
            Type::Slice(TypeSlice { elem, .. }) => syn::parse_quote! { Vec<#elem> },
            Type::Path(TypePath { path, .. }) => {
                match path.segments.first().unwrap().ident.to_string().as_str() {
                    "str" => syn::parse_quote! { String },
                    _ => elem.as_ref().clone(),
                }
            }
            _ => elem.as_ref().clone(),
        },
        Type::Path(TypePath { qself, path }) => {
            let mut segments = Punctuated::new();
            segments.extend(path.segments.iter().map(|seg| {
                let mut seg = seg.clone();
                seg.arguments = args_to_owned(&seg.arguments);
                seg
            }));
            let path = syn::Path {
                leading_colon: path.leading_colon,
                segments,
            };
            Type::Path(TypePath {
                qself: qself.to_owned(),
                path,
            })
        }
        _ => t.clone(),
    }
}

/// ```rust
/// #[erg_proc_macros::to_owned]
/// fn foo(s: &str) -> &str { s }
/// ```
/// ↓ ↓
/// ```rust
/// fn foo(s: &str) -> String { let r = s; r.to_owned() }
/// ```
#[proc_macro_attribute]
pub fn to_owned(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_fn = syn::parse_macro_input!(item as syn::ItemFn);
    let ReturnType::Type(_, out) = &item_fn.sig.output else {
        todo!()
    };
    let out = type_to_owned(out);
    let block = item_fn.block;
    let block = if attr
        .into_iter()
        .next()
        .is_some_and(|attr| attr.to_string().as_str() == "cloned")
    {
        syn::parse_quote! {{
            let r = #block;
            r.cloned()
        }}
    } else {
        syn::parse_quote! {{
            let r = #block;
            r.to_owned()
        }}
    };
    item_fn.block = Box::new(block);
    item_fn.sig.output = syn::parse_quote! { -> #out };
    let item = quote! { #item_fn };
    item.into()
}
