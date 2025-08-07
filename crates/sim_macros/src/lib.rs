use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident};

#[proc_macro_derive(SimDomain)]
pub fn derive_sim_domain(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    let domain_name_str = struct_name.to_string().strip_suffix("Domain").unwrap_or(&struct_name.to_string()).to_string();
    let domain_variant = Ident::new(&domain_name_str, struct_name.span());
    let domain_result = Ident::new(&format!("{}Result", domain_name_str), struct_name.span());

    let expanded = quote! {
        impl crate::prelude::Domain for #struct_name {
            fn name(&self) -> &'static str {
                #domain_name_str
            }

            fn execute(&self, action: &sim_core::SimAction, state: &sim_core::SimState) -> crate::prelude::DomainResult {
                match action {
                    sim_core::SimAction::#domain_variant(domain_action) => {
                        let result: crate::prelude::#domain_result = self.execute(domain_action, state);

                        crate::prelude::DomainResult {
                            success: result.success,
                            effects: result.effects,
                            errors: result.errors,
                        }
                    }
                    _ => crate::prelude::DomainResult {
                        success: false,
                        effects: vec![],
                        errors: vec![format!("{} domain received wrong action type", self.name())],
                    },
                }
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }

        ::inventory::submit! {
            crate::prelude::DomainRegistration {
                name: #domain_name_str,
                constructor: || Box::new(#struct_name::new()) as Box<dyn crate::prelude::Domain>,
            }
        }
    };

    TokenStream::from(expanded)
}
