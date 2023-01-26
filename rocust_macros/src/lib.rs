use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use quote::quote;
use syn::{parse::Parser, parse_macro_input, DeriveInput};

#[proc_macro_attribute]
pub fn user(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    let mut derive_block = parse_macro_input!(item as DeriveInput);
    match &mut derive_block.data {
        syn::Data::Struct(ref mut struct_data) => {
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => {
                    fields.named.push(
                        syn::Field::parse_named
                            .parse2(quote! { pub results: rocust_lib::results::Results })
                            .unwrap(),
                    );
                    fields.named.push(
                        syn::Field::parse_named
                            .parse2(quote! { pub tasks: Vec<rocust_lib::tasks::Task<Self>> })
                            .unwrap(),
                    );
                    fields.named.push(
                        syn::Field::parse_named
                            .parse2(
                                quote! { pub async_tasks: Vec<rocust_lib::tasks::AsyncTask<Self>> },
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
pub fn has_task(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    let mut impl_block = syn::parse_macro_input!(item as syn::ItemImpl);
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
                            let priority = syn::LitInt::new(&priority.to_string(), lit.span());
                            methods.push((
                                method.sig.ident.clone(),
                                priority,
                                method.sig.asyncness.is_some(),
                            ));
                        } else {
                            panic!("Only i32 is supported");
                        }
                    } else {
                        panic!("Only Literal is supported");
                    };
                }
            }

            //remove the attribute
            method.attrs.retain(|attr| {
                if attr.path.is_ident("task") {
                    return false;
                }
                true
            });
        }
    }

    let methods = methods.iter().map(|(method_name, priority, is_async)| {
        if *is_async {
            quote! {
                fn #method_name(u: &mut #struct_name) -> ::core::pin::Pin<Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + '_>> {
                    Box::pin(async move {
                        u.#method_name().await;
                    })
                }
                self.async_tasks.push(rocust_lib::tasks::AsyncTask::new(#priority, #method_name));
            }
        }
        else {
            quote! {
                fn #method_name(u: &mut #struct_name) {
                    u.#method_name();
                }
                self.tasks.push(rocust_lib::tasks::Task::new(#priority, #method_name));
            }
        }

    });

    //now we can implement the function in the User trait that will inject the tasks in the user struct
    let expanded = quote! {
        #impl_block

        impl rocust_lib::traits::HasTask for #struct_name {
            fn inject_tasks(&mut self) {
                #(#methods)*
            }

            fn add_succ(&mut self, dummy: i32) {
                self.results.add_succ(dummy);
            }

            fn add_fail(&mut self, dummy: i32) {
                self.results.add_fail(dummy);
            }

            fn get_async_tasks(&self) -> Vec<rocust_lib::tasks::AsyncTask<Self>> where Self: Sized {
                self.async_tasks.clone()
            }

            fn get_tasks(&self) -> Vec<rocust_lib::tasks::Task<Self>> where Self: Sized {
                self.tasks.clone()
            }

        }
    };

    expanded.into()
}
