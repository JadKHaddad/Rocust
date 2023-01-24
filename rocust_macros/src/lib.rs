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
            method.attrs.retain(|attr| {
                if attr.path.is_ident("task") {
                    return false;
                }
                true
            });
        }
    }

    let methods = methods.iter().map(|(method_name, priority)| {
        quote! {
            self.tasks.push(rocust_lib::tasks::Task::new(#priority, #struct_name::#method_name));
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

            fn get_tasks(&self) -> Vec<rocust_lib::tasks::Task<Self>> where Self: Sized {
                self.tasks.clone()
            }

        }
    };

    expanded.into()
}

// #[proc_macro_attribute]
// pub fn task(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let method = syn::parse_macro_input!(item as syn::ItemFn);

//     // Break the function down into its parts
//     let syn::ItemFn {
//         attrs: _,
//         vis: _,
//         sig,
//         block: _,
//     } = &method;

//     // Ensure that it isn't an `async fn`
//     if let Some(async_token) = sig.asyncness {
//         // Error out if so
//         let error = syn::Error::new(
//             async_token.span(),
//             "async functions do not support caller tracking functionality
//     help: consider returning `impl Future` instead",
//         );

//         return TokenStream::from(error.to_compile_error());
//     }

//     let struct_name = method.sig.ident.to_string();
//     let new_struct_name = format!("{}_{}", struct_name, method.sig.ident);

//     let new_struct_name = syn::Ident::new(&new_struct_name, method.sig.ident.span());

//     // Extracting field name and value
//     let attr_string = attr.to_string();
//     let field_name_value:Vec<&str> = attr_string.split("=").collect();
//     let field_name = field_name_value[0].trim();
//     if field_name != "priority"{
//         panic!("The only argument that can be passed to the macro is priority");
//     }
//     let field_value = field_name_value[1].trim();
//     if field_value != "1" && field_value != "2" && field_value != "3"{
//         panic!("The only values that can be passed to the macro are 1, 2, or 3");
//     }

//     let expanded = quote! {
//         // struct #new_struct_name {
//         //     #field_name: String = #field_value
//         // }
//         // the problem is: this macro will spit out the struct inside an impl block!
//     };
//     expanded.into()
// }

// #[proc_macro_derive(User)]
// pub fn derive(input: TokenStream) -> TokenStream {
//     let ast = parse_macro_input!(input as DeriveInput);
//     let name = &ast.ident;

//     let expanded = quote! {
//         impl #name {
//             fn with_tasks(mut self, tasks: Vec<rocust_lib::tasks::Task<Self>>) -> Self {
//                 self.tasks = tasks;
//                 self
//             }
//         }

//         impl rocust_lib::traits::User for #name {
//             fn add_succ(&mut self, dummy: i32) {
//                 self.results.add_succ(dummy);
//             }
//             fn add_fail(&mut self, dummy: i32) {
//                 self.results.add_fail(dummy);
//             }
//         }
//     };

//     expanded.into()
// }
