
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, FnArg, Pat, Ident, Item, ItemFn};

#[proc_macro_attribute]
pub fn syscall_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let info = parse_macro_input!(item as ItemFn);
    let existing_name = info.sig.ident.clone();
    let handler_name = Ident::new(&format!("handle_{}", existing_name), Span::call_site());

    let args = info.sig.inputs.iter().cloned().collect::<Vec<_>>();
    let args_names = info.sig.inputs.iter().map(|arg| {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                pat_ident.ident.clone()
            } else {
                panic!("expected identifier");
            }
        } else {
            panic!("expected typed argument");
        }
    }).collect::<Vec<_>>();

    let existing = Item::Fn(info);
    let expanded = quote! {
        #existing

        pub fn #handler_name(syscall: &mut ruxpin_syscall::SyscallRequest) {
            use ruxpin_syscall::IntoSyscallResult; 
            let mut i = 0;
            #( ruxpin_syscall::syscall_decode!(syscall, i, #args); )*
            let result = #existing_name(#( #args_names ),*);
            syscall.store_result(result.map(|ret| ret.into_result()).map_err(|err| ruxpin_types::ApiError::from(err)));
        }
    };

    expanded.into()
}



/*
use syn::{parse_macro_input, Expr, Data, DeriveInput};

#[proc_macro_derive(EnumSubset)]
pub fn derive_enum_subset(item: TokenStream) -> TokenStream {
    let info = parse_macro_input!(item as DeriveInput);
    let enum_data = match &info.data {
        Data::Enum(data) => data,
        _ => panic!("expected enum"),
    };

    let expanded = quote! {
        impl #info.ident {

        }
    };

    TokenStream::from(expanded)
}
*/
