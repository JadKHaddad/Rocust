use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs};

#[proc_macro_attribute]
pub fn has_task(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attrs as AttributeArgs);
    let mut impl_block = syn::parse_macro_input!(item as syn::ItemImpl);

    let mut min_sleep = 0;
    let mut max_sleep = 0;
    let mut weight = 1;

    for attr in attrs {
        if let syn::NestedMeta::Meta(syn::Meta::NameValue(name_value)) = attr {
            if name_value.path.get_ident().unwrap().to_string() == "min_sleep" {
                if let syn::Lit::Int(lit_str) = name_value.lit {
                    min_sleep = match lit_str.base10_digits().parse::<u64>() {
                        Ok(val) => val,
                        Err(_) => panic!("min_sleep has to be u64"),
                    };
                } else {
                    panic!("min_sleep has to be an integer");
                }
            } else if name_value.path.get_ident().unwrap().to_string() == "max_sleep" {
                if let syn::Lit::Int(lit_str) = name_value.lit {
                    max_sleep = match lit_str.base10_digits().parse::<u64>() {
                        Ok(val) => val,
                        Err(_) => panic!("max_sleep has to be u64"),
                    };
                } else {
                    panic!("max_sleep has to be an integer");
                }
            } else if name_value.path.get_ident().unwrap().to_string() == "weight" {
                if let syn::Lit::Int(lit_str) = name_value.lit {
                    weight = match lit_str.base10_digits().parse::<u64>() {
                        Ok(val) => val,
                        Err(_) => panic!("weight has to be u64"),
                    };
                } else {
                    panic!("weight has to be an integer");
                }
            } else {
                panic!("Only min_sleep, max_sleep and weight are supported");
            }
        } else {
            panic!("Only Meta is supported");
        }
    }

    if max_sleep < min_sleep {
        panic!("max_sleep cannot be smaller than min_sleep");
    }

    let min_sleep = syn::LitInt::new(&min_sleep.to_string(), proc_macro2::Span::call_site());
    let max_sleep = syn::LitInt::new(&max_sleep.to_string(), proc_macro2::Span::call_site());
    let weight = syn::LitInt::new(&weight.to_string(), proc_macro2::Span::call_site());

    let struct_name = if let syn::Type::Path(type_path) = &impl_block.self_ty.as_ref() {
        if let Some(ident) = type_path.path.get_ident() {
            ident.clone()
        } else {
            // here we have some generics and lifetimes, let's just deny them for now
            panic!("Generics and lifetimes are not supported");
        }
    } else {
        panic!("Could not get type path from self type");
    };

    let name = syn::LitStr::new(&struct_name.to_string(), proc_macro2::Span::call_site());
    let mut methods = Vec::new();

    // collect all the methods names if they have a "proiority" attribute and the value is a number (u64) and delete the attribute
    for item in impl_block.items.iter_mut() {
        if let syn::ImplItem::Method(method) = item {
            let task_attrs = method
                .attrs
                .iter()
                .filter(|attr| attr.path.segments[0].ident == "task");

            for attr in task_attrs {
                let mut token_stream = attr.tokens.clone().into_iter();
                if let TokenTree::Group(group) = token_stream.next().expect("No group found") {
                    let tokens = group.stream();
                    let mut iter = tokens.into_iter();

                    if let TokenTree::Ident(ident) = iter.next().expect("No ident found") {
                        if ident.to_string() != "priority" {
                            panic!("Only priority is supported");
                        }
                    } else {
                        panic!("Only Ident is supported");
                    };

                    if let TokenTree::Punct(punct) = iter.next().expect("No punct found") {
                        if punct.as_char() != '=' {
                            panic!("Only '=' is supported");
                        }
                    } else {
                        panic!("Only Punct is supported");
                    };

                    if let TokenTree::Literal(lit) = iter.next().expect("No literal found") {
                        if let Ok(priority) = lit.to_string().parse::<u64>() {
                            if method.sig.asyncness.is_none() {
                                panic!("Only async methods are supported");
                            }
                            let priority = syn::LitInt::new(&priority.to_string(), lit.span());
                            methods.push((method.sig.ident.clone(), priority));
                        } else {
                            panic!("Only u64 is supported");
                        }
                    } else {
                        //panic!("Only Literal is supported");
                        panic!("Only u64 is supported");
                    };
                }
            }

            // remove the attribute
            method.attrs.retain(|attr| !attr.path.is_ident("task"));
        }
    }

    let methods = methods.iter().map(|(method_name, priority)| {
        quote! {
            fn #method_name<'a>(u: &'a mut #struct_name, data: &'a rocust::rocust_lib::data::Data) -> ::core::pin::Pin<Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'a>> {
                Box::pin(async move {
                    u.#method_name(data).await;
                })
            }
            async_tasks.push(rocust::rocust_lib::tasks::AsyncTask::new(#priority, #method_name));
        }
    });

    // now we can implement the function in the User trait that will inject the tasks in the user struct
    let expanded = quote! {
        #impl_block

        impl rocust::rocust_lib::traits::HasTask for #struct_name {
            fn get_async_tasks() -> Vec<rocust::rocust_lib::tasks::AsyncTask<Self>> where Self: Sized {
                let mut async_tasks: Vec<rocust::rocust_lib::tasks::AsyncTask<Self>> = Vec::new();
                #(#methods)*;
                async_tasks
            }

            fn get_name() -> String {
                String::from(#name)
            }

            fn get_between() -> (u64, u64) {
                (#min_sleep, #max_sleep)
            }

            fn get_weight() -> u64 {
                #weight
            }
        }
    };

    expanded.into()
}
