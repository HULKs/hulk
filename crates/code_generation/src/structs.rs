use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use source_analyzer::{struct_hierarchy::StructHierarchy, structs::Structs};

pub fn generate_structs(structs: &Structs) -> TokenStream {
    let derives = quote! {
        #[derive(
            Clone,
            Debug,
            Default,
            serde::Deserialize,
            serde::Serialize,
            serialize_hierarchy::SerializeHierarchy,
         )]
    };
    let parameters =
        hierarchy_to_token_stream(&structs.parameters, format_ident!("Parameters"), &derives);
    let cyclers = structs
        .cyclers
        .iter()
        .map(|(cycler_module, cycler_structs)| {
            let cycler_module_identifier = format_ident!("{}", cycler_module.to_case(Case::Snake));
            let main_outputs = hierarchy_to_token_stream(
                &cycler_structs.main_outputs,
                format_ident!("MainOutputs"),
                &derives,
            );
            let additional_outputs = hierarchy_to_token_stream(
                &cycler_structs.additional_outputs,
                format_ident!("AdditionalOutputs"),
                &derives,
            );
            let cycler_state = hierarchy_to_token_stream(
                &cycler_structs.cycler_state,
                format_ident!("CyclerState"),
                &derives,
            );

            quote! {
                pub mod #cycler_module_identifier {
                    #main_outputs
                    #additional_outputs
                    #cycler_state
                }
            }
        });

    let recording_trigger = recording_trigger();

    quote! {
        #parameters
        #(#cyclers)*
        #recording_trigger
    }
}

fn hierarchy_to_token_stream(
    hierarchy: &StructHierarchy,
    struct_name: Ident,
    derives: &TokenStream,
) -> TokenStream {
    let fields = match hierarchy {
        StructHierarchy::Struct { fields } => fields,
        StructHierarchy::Optional { .. } => panic!("option instead of struct"),
        StructHierarchy::Field { .. } => panic!("field instead of struct"),
    };
    let struct_fields = fields.iter().map(|(name, struct_hierarchy)| {
        let name_identifier = format_ident!("{}", name);
        match struct_hierarchy {
            StructHierarchy::Struct { .. } => {
                let struct_name_identifier =
                    format_ident!("{}{}", struct_name, name.to_case(Case::Pascal));
                quote! { pub #name_identifier: #struct_name_identifier }
            }
            StructHierarchy::Optional { child } => match &**child {
                StructHierarchy::Struct { .. } => {
                    let struct_name_identifier =
                        format_ident!("{}{}", struct_name, name.to_case(Case::Pascal));
                    quote! { pub #name_identifier: Option<#struct_name_identifier> }
                }
                StructHierarchy::Optional { .. } => {
                    panic!("unexpected optional in an optional struct")
                }
                StructHierarchy::Field { data_type } => {
                    quote! { pub #name_identifier: Option<#data_type> }
                }
            },
            StructHierarchy::Field { data_type } => {
                quote! { pub #name_identifier: #data_type }
            }
        }
    });
    let child_structs = fields.iter().map(|(name, struct_hierarchy)| {
        let struct_name = format_ident!("{}{}", struct_name, name.to_case(Case::Pascal));
        match struct_hierarchy {
            StructHierarchy::Struct { .. } => {
                hierarchy_to_token_stream(struct_hierarchy, struct_name, derives)
            }
            StructHierarchy::Optional { child } => match &**child {
                StructHierarchy::Struct { .. } => {
                    hierarchy_to_token_stream(child, struct_name, derives)
                }
                StructHierarchy::Optional { .. } => {
                    panic!("unexpected optional in an optional struct")
                }
                StructHierarchy::Field { .. } => quote! {},
            },
            StructHierarchy::Field { .. } => quote! {},
        }
    });
    quote! {
        #derives
        pub struct #struct_name {
            #(#struct_fields,)*
        }
        #(#child_structs)*
    }
}

fn recording_trigger() -> TokenStream {
    quote! {
        pub struct RecordingTrigger {
            recording_interval: usize,
            counter: usize
        }

        impl RecordingTrigger {
            pub fn new(recording_interval: usize) -> Self {
                Self { recording_interval, counter: 0 }
            }

            pub fn cycle_finished(&mut self) {
                self.counter = (self.counter + 1) % self.recording_interval;
            }

            pub fn should_record(&self) -> bool {
                self.recording_interval != 0 && self.counter == 0
            }
        }
    }
}
