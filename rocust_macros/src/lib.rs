use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use quote::quote;
use syn::{parse::Parser, parse_macro_input, AttributeArgs, DeriveInput};

#[proc_macro_attribute]
pub fn user(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    let mut derive_block = parse_macro_input!(item as DeriveInput);
    match &mut derive_block.data {
        syn::Data::Struct(ref mut struct_data) => {
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => {
                    fields.named.push(
                        syn::Field::parse_named
                            .parse2(
                                quote! { pub results_sender: rocust_lib::results::ResultsSender },
                            )
                            .unwrap(),
                    );
                }
                _ => (),
            }

            return quote! {
                #derive_block
            }
            .into();
        }
        _ => panic!("`user` has to be used with structs "),
    }
}

#[proc_macro_attribute]
pub fn has_task(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attrs as AttributeArgs);
    let mut impl_block = syn::parse_macro_input!(item as syn::ItemImpl);
    //check if between is set
    let mut min = 0;
    let mut max = 0;
    for attr in attrs {
        if let syn::NestedMeta::Meta(syn::Meta::NameValue(name_value)) = attr {
            if name_value.path.get_ident().unwrap().to_string() == "between" {
                if let syn::Lit::Str(lit_str) = name_value.lit {
                    let between_str = lit_str.value();
                    let between_str = between_str.split_whitespace().collect::<String>();
                    if between_str.starts_with('(') && between_str.ends_with(')') {
                        let between_str = &between_str[1..between_str.len() - 1];
                        let between_str = between_str.trim();
                        let between_str = between_str.split(',');
                        let between_str = between_str.collect::<Vec<&str>>();
                        if between_str.len() != 2 {
                            panic!("between has to have 2 values");
                        }
                        min = between_str[0].parse::<u64>().unwrap();
                        max = between_str[1].parse::<u64>().unwrap();
                    } else {
                        panic!("between has to be in the format (min, max)");
                    }
                } else {
                    panic!("between has to be a string");
                }
            } else {
                panic!("Only between is supported");
            }
        } else {
            panic!("Only Meta is supported");
        }
    }
    let min = syn::LitInt::new(&min.to_string(), proc_macro2::Span::call_site());
    let max = syn::LitInt::new(&max.to_string(), proc_macro2::Span::call_site());

    let struct_name = if let syn::Type::Path(type_path) = &impl_block.self_ty.as_ref() {
        if let Some(ident) = type_path.path.get_ident() {
            ident
        } else {
            panic!("Could not get ident from type path");
        }
    } else {
        panic!("Could not get type path from self type");
    };

    let mut methods = Vec::new();

    //collect all the methods names if they have a "proiority" attribute and the value is a number (i32) and delete the attribute
    for item in impl_block.items.iter_mut() {
        if let syn::ImplItem::Method(method) = item {
            let task_attrs = method
                .attrs
                .iter()
                .filter(|attr| attr.path.segments[0].ident == "task");

            for attr in task_attrs {
                let mut token_stream = attr.tokens.clone().into_iter();
                if let TokenTree::Group(group) = token_stream.next().unwrap() {
                    let tokens = group.stream();
                    let mut iter = tokens.into_iter();

                    if let TokenTree::Ident(ident) = iter.next().unwrap() {
                        if ident.to_string() != "priority" {
                            panic!("Only priority is supported");
                        }
                    } else {
                        panic!("Only Ident is supported");
                    };

                    if let TokenTree::Punct(punct) = iter.next().unwrap() {
                        if punct.as_char() != '=' {
                            panic!("Only '=' is supported");
                        }
                    } else {
                        panic!("Only Punct is supported");
                    };

                    if let TokenTree::Literal(lit) = iter.next().unwrap() {
                        if let Ok(priority) = lit.to_string().parse::<i32>() {
                            if method.sig.asyncness.is_none() {
                                panic!("Only async methods are supported");
                            }
                            let priority = syn::LitInt::new(&priority.to_string(), lit.span());
                            methods.push((method.sig.ident.clone(), priority));
                        } else {
                            panic!("Only i32 is supported");
                        }
                    } else {
                        panic!("Only Literal is supported");
                    };
                }
            }

            //remove the attribute
            method.attrs.retain(|attr| !attr.path.is_ident("task"));
        }
    }

    let methods = methods.iter().map(|(method_name, priority)| {
        quote! {
            fn #method_name(u: &mut #struct_name) -> ::core::pin::Pin<Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + '_>> {
                Box::pin(async move {
                    u.#method_name().await;
                })
            }
            async_tasks.push(rocust_lib::tasks::AsyncTask::new(#priority, #method_name));
        }

    });

    //now we can implement the function in the User trait that will inject the tasks in the user struct
    let expanded = quote! {
        #impl_block

        impl rocust_lib::traits::HasTask for #struct_name {
            fn get_async_tasks() -> Vec<rocust_lib::tasks::AsyncTask<Self>> where Self: Sized {
                let mut async_tasks: Vec<rocust_lib::tasks::AsyncTask<Self>> = Vec::new();
                #(#methods)*;
                async_tasks
            }

            fn get_between() -> (u64, u64) {
                (#min, #max)
            }

            fn get_results_sender(&self) -> &rocust_lib::results::ResultsSender{
                &self.results_sender
            }

            fn set_sender(&mut self, sender: tokio::sync::mpsc::UnboundedSender<rocust_lib::results::ResultMessage>){
                self.results_sender.set_sender(sender);
            }
        }
    };

    expanded.into()
}
