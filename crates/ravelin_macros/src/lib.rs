use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, Meta, parse_macro_input};

#[proc_macro_derive(CollectAgents, attributes(agent_collection))]
pub fn collect_agents_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let fields = match &ast.data {
        Data::Struct(s) => &s.fields,
        _ => panic!("#[derive(CollectAgents)] is only supported on structs"),
    };

    let extend_calls = generate_extend_calls(fields);

    let generated_impl = quote! {
        impl #name {
            pub fn collect_all_agents(&self) -> Vec<Box<dyn ravelin_traits::AbstractAgent<Core = crate::RavelinCore>>> {
                let mut agents: Vec<Box<dyn ravelin_traits::AbstractAgent<Core = crate::RavelinCore>>> = Vec::new();
                #(#extend_calls)*
                agents
            }
        }
    };

    generated_impl.into()
}

fn generate_extend_calls(fields: &Fields) -> Vec<TokenStream2> {
    fields.iter().filter_map(|f| {
        let field_name = f.ident.as_ref().expect("Field must have a name");

        f.attrs.iter().find(|attr| attr.path().is_ident("agent_collection")).map(|attr| {
            let access_path = match &attr.meta {
                Meta::Path(_) => {
                    quote! { self.#field_name }
                }
                Meta::List(meta_list) => {
                    if let Ok(path_lit) = meta_list.parse_args::<syn::LitStr>() {
                        let path_str = path_lit.value();
                        let span = path_lit.span();
                        let segments = path_str.split('.').map(|s| Ident::new(s, span));
                        quote! { self.#(#segments).* }
                    } else {
                        panic!("Expected a string literal for path, e.g., #[agent_collection(path = \"field.subfield\")]");
                    }
                }
                _ => panic!("Unsupported agent_collection attribute format"),
            };

            let field_type = &f.ty;
            let type_str = quote!(#field_type).to_string();

            if type_str.starts_with("Vec") {
                quote! {
                    agents.extend(
                        #access_path
                            .iter()
                            .map(|agent| Box::new(crate::AgentAdapter(agent.clone())) as Box<dyn ravelin_traits::AbstractAgent<Core = crate::RavelinCore>>)
                    );
                }
            } else if type_str.contains("HashMap") {
                 quote! {
                    agents.extend(
                        #access_path
                            .values()
                            .map(|agent| Box::new(crate::AgentAdapter(agent.clone())) as Box<dyn ravelin_traits::AbstractAgent<Core = crate::RavelinCore>>)
                    );
                }
            } else {
                quote! {
                    agents.extend(#access_path.collect_all_agents());
                }
            }
        })
    }).collect()
}
