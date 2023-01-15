use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, parse::Parser, spanned::Spanned};
use quote::quote;

#[proc_macro_attribute]
pub fn be_user(_args: TokenStream, input: TokenStream) -> TokenStream  {
    let mut ast = parse_macro_input!(input as DeriveInput);
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {           
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => {
                    fields
                        .named
                        .push(syn::Field::parse_named.parse2(quote! { pub results: rocust_lib::results::Results }).unwrap());
                    fields
                        .named
                        .push(syn::Field::parse_named.parse2(quote! { pub tasks: Vec<rocust_lib::tasks::Task<Self>> }).unwrap());
                        
                }   
                _ => {
                    ()
                }
            }              
            
            return quote! {
                #ast
            }.into();
        }
        _ => panic!("`add_field` has to be used with structs "),
    }
}

#[proc_macro_attribute]
pub fn task(attr: TokenStream, item: TokenStream) -> TokenStream {
    let method = syn::parse_macro_input!(item as syn::ItemFn);

    // Break the function down into its parts
    let syn::ItemFn {
        attrs: _,
        vis: _,
        sig,
        block: _,
    } = &method;

    // Ensure that it isn't an `async fn`
    if let Some(async_token) = sig.asyncness {
        // Error out if so
        let error = syn::Error::new(
            async_token.span(),
            "async functions do not support caller tracking functionality
    help: consider returning `impl Future` instead",
        );

        return TokenStream::from(error.to_compile_error());
    }

    let struct_name = method.sig.ident.to_string();
    let new_struct_name = format!("{}_{}", struct_name, method.sig.ident);

    let new_struct_name = syn::Ident::new(&new_struct_name, method.sig.ident.span());
    
    // Extracting field name and value
    let attr_string = attr.to_string();
    let field_name_value:Vec<&str> = attr_string.split("=").collect();
    let field_name = field_name_value[0].trim();
    if field_name != "priority"{
        panic!("The only argument that can be passed to the macro is priority");
    }
    let field_value = field_name_value[1].trim();
    if field_value != "1" && field_value != "2" && field_value != "3"{
        panic!("The only values that can be passed to the macro are 1, 2, or 3");
    }

    let expanded = quote! {
        // struct #new_struct_name {
        //     #field_name: String = #field_value
        // }
        // the problem is: this macro will spit out the struct inside an impl block!
    };
    expanded.into()
}

#[proc_macro_derive(User)]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let expanded = quote! {
        impl #name {
            fn with_tasks(mut self, tasks: Vec<rocust_lib::tasks::Task<Self>>) -> Self {
                self.tasks = tasks;
                self
            }
        }

        impl rocust_lib::traits::User for #name {
            fn add_succ(&mut self, dummy: i32) {
                self.results.add_succ(dummy);
            }
            fn add_fail(&mut self, dummy: i32) {
                self.results.add_fail(dummy);
            }
        }
    };

    expanded.into()
}
