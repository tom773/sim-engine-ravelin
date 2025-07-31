extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{ItemEnum, Lit, Meta, Token, parse_macro_input};

fn parse_domain(args: &Punctuated<Meta, Token![,]>) -> syn::Result<Option<String>> {
    for meta in args {
        if let Meta::NameValue(nv) = meta {
            if nv.path.is_ident("domain") {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(litstr) = &expr_lit.lit {
                        return Ok(Some(litstr.value()));
                    }
                }
            }
        }
    }
    Ok(None)
}
#[proc_macro_attribute]
pub fn sim_action(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = match Punctuated::<Meta, Token![,]>::parse_terminated.parse(attr) {
        Ok(args) => args,
        Err(err) => return err.to_compile_error().into(),
    };
    let item = parse_macro_input!(input as ItemEnum);
    let enum_ident = &item.ident;

    let _domain_str = match parse_domain(&args) {
        Ok(domain) => domain,
        Err(e) => return e.to_compile_error().into(),
    };

    let expanded = quote! {
        #item

        impl ::core::AnyAction for #enum_ident {
            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }

            fn validate(&self, world: &dyn ::core::WorldState) -> Result<(), ::core::ActionError> {
                <Self as ::core::Action>::validate(self, world)
            }

            fn execute(&self, world: &dyn ::core::WorldState) -> Vec<Box<dyn ::core::AnyEffect>> {
                <Self as ::core::Action>::execute(self, world)
                    .into_iter()
                    .map(|e| Box::new(e) as Box<dyn ::core::AnyEffect>)
                    .collect()
            }
        }
    };
    expanded.into()
}

#[proc_macro_attribute]
pub fn sim_decision(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemEnum);
    let enum_ident = &item.ident;
    let arms = item.variants.iter().map(|v| {
        let vident = &v.ident;
        quote!( #enum_ident::#vident => vec![] )
    });
    let expanded = quote! {
        #item

        impl ::core::Decision for #enum_ident {
            type Action = Box<dyn ::core::AnyAction>;
            fn into_actions(self) -> Vec<Self::Action> {
                match self { #( #arms ),* }
            }
        }
    };
    expanded.into()
}

#[proc_macro_attribute]
pub fn sim_effect(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemEnum);

    let expanded = quote! {
        #item
    };
    expanded.into()
}
