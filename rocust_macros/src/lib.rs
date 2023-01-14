use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, parse::Parser};
use quote::quote;

#[proc_macro_attribute]
pub fn add_field(_args: TokenStream, input: TokenStream) -> TokenStream  {
    let mut ast = parse_macro_input!(input as DeriveInput);
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {           
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => {
                    fields
                        .named
                        .push(syn::Field::parse_named.parse2(quote! { pub results: rocust_lib::results::Results }).unwrap());
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

#[proc_macro_derive(User)]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let expanded = quote! {
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
