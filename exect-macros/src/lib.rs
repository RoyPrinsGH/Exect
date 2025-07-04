use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
/// Turn this function into a serializable instruction.
pub fn exect(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse the input as a function
    let input = parse_macro_input!(item as ItemFn);
    let func_sig = &input.sig;
    let func_body = &input.block;
    let func_vis = &input.vis;
    let func_name = &func_sig.ident;

    // extract parameter names and types
    let params = func_sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    Some((pat_ident.ident.clone(), (*pat_type.ty).clone()))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // generate struct fields
    let fields: Vec<_> = params
        .iter()
        .map(|(ident, ty)| {
            quote! {
                pub #ident: #ty
            }
        })
        .collect();

    // generate the arguments for the execute call
    let args: Vec<_> = params
        .iter()
        .map(|(ident, _)| {
            quote! {
                self.#ident
            }
        })
        .collect();

    // collect identifiers for formatting
    let idents: Vec<_> = params.iter().map(|(ident, _)| ident.clone()).collect();

    // build a new struct name: e.g. `foo` → `FooInstruction`
    let name_str = func_name.to_string();
    // Convert snake_case to CamelCase and append "Instruction"
    let struct_name_str = {
        let camel: String = name_str
            .split('_')
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect();
        format!("{}Instruction", camel)
    };
    let struct_name = format_ident!("{}", struct_name_str, span = Span::call_site());

    // emit the original function plus the new struct + impl
    let id_expr = parse_macro_input!(_attr as syn::Expr);

    let factory_fn_name = format_ident!("__factory_inst_{}", name_str, span = Span::call_site());

    let execute_function = if matches!(func_sig.output, syn::ReturnType::Default) {
        // no return ⇒ call and return None
        quote! {
            fn execute(self: Box<Self>) -> Option<exect_core::ExecutorSignal> {
                #func_name(#(#args),*);
                None
            }
        }
    } else if let syn::ReturnType::Type(_, ty) = &func_sig.output {
        // has a return type
        if let syn::Type::Path(type_path) = &**ty {
            if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Option" {
                quote! {
                    fn execute(self: Box<Self>) -> Option<exect_core::ExecutorSignal> {
                        // returns Option<ExecutorSignal> directly
                        #func_name(#(#args),*)
                    }
                }
            } else {
                // anything else ⇒ assume ExecutorSignal and wrap in Some()
                quote! {
                    fn execute(self: Box<Self>) -> Option<exect_core::ExecutorSignal> {
                        Some(#func_name(#(#args),*))
                    }
                }
            }
        } else {
            // anything else ⇒ assume ExecutorSignal and wrap in Some()
            quote! {
                fn execute(self: Box<Self>) -> Option<exect_core::ExecutorSignal> {
                    Some(#func_name(#(#args),*))
                }
            }
        }
    } else {
        unreachable!("unexpected return type");
    };

    let expanded = quote! {
        #func_vis #func_sig #func_body

        /// Auto-generated by exect
        #[derive(exect_core::__exect_serde::Serialize, exect_core::__exect_serde::Deserialize, Debug)]
        #[serde(crate = "exect_core::__exect_serde")]
        pub struct #struct_name {
            #(#fields),*
        }

        impl std::fmt::Display for #struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}(", stringify!(#func_name))?;
                let mut first = true;
                #(
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{}: {:?}", stringify!(#idents), #args)?;
                )*
                write!(f, ")")
            }
        }

        impl exect_core::Instruction for #struct_name {
            fn get_code(&self) -> u8 {
                #id_expr as u8
            }
            fn to_bytes(self) -> Vec<u8> {
                let mut buf = Vec::new();
                buf.extend(exect_core::__exect_postcard::to_allocvec(&self).expect("postcard serialization failed"));
                buf
            }
            #execute_function
        }

        fn #factory_fn_name(buffer: &[u8]) -> (Box<dyn exect_core::Instruction>, &[u8]) {
            let (instruction, unused) =
                exect_core::__exect_postcard::take_from_bytes::<#struct_name>(buffer)
                    .expect("postcard deserialization failed");
            (Box::new(instruction), unused)
        }

        exect_core::__exect_inventory::submit! {
            exect_core::InstructionInfo {
                code: #id_expr as u8,
                name: stringify!(#func_name),
                instruction_factory: #factory_fn_name,
            }
        }
    };

    expanded.into()
}
