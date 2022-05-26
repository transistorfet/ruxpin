
#![feature(box_patterns)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, FnArg, Pat, Ident, Item, ItemFn, ReturnType, Type};

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

#[proc_macro_attribute]
pub fn syscall_function(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(attr as Ident);

    let info = parse_macro_input!(item as ItemFn);
    let vis = info.vis.clone();
    let name = info.sig.ident.clone();
    let inputs = info.sig.inputs.clone();
    let output = info.sig.output.clone();

    let args = info.sig.inputs.iter().cloned().collect::<Vec<_>>();

    let process_result = if let ReturnType::Type(_, box Type::Never(_)) = output {
        quote! { loop {} }
    } else {
        quote! {
            match syscall.error {
                false => Ok(FromSyscallResult::from_result(syscall.result)),
                true => Err(ApiError::from(syscall.result)),
            }
        }
    };

    let expanded = quote! {
        #vis fn #name(#inputs) #output {
            use ruxpin_syscall::arch::execute_syscall;
            use ruxpin_syscall::{SyscallRequest, SyscallFunction, FromSyscallResult};
            let mut i = 0;
            let mut syscall: SyscallRequest = Default::default();
            #( ruxpin_syscall::syscall_encode!(syscall, i, #args); )*
            syscall.function = SyscallFunction::#function;
            execute_syscall(&mut syscall);
            #process_result
        }
    };

    expanded.into()
}

