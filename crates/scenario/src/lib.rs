use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn scenario(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let function_item = parse_macro_input!(item as ItemFn);
    let function_name = function_item.sig.ident.clone();

    quote! {
        #function_item

        fn main() -> color_eyre::Result<()> {
            use bevyhavior_simulator::behavior_tree_simulator::{AppExt, BehaviorTreeSimulatorPlugin};

            App::new()
                .add_plugins(BehaviorTreeSimulatorPlugin::default())
                .add_plugins(#function_name)
                .run_to_completion_with_viewer()
        }

        #[cfg(test)]
        mod test {
            #[test]
            fn #function_name() -> color_eyre::Result<()> {
                use bevyhavior_simulator::behavior_tree_simulator::{AppExt, BehaviorTreeSimulatorPlugin};

                bevy::app::App::new()
                    .add_plugins(BehaviorTreeSimulatorPlugin::default())
                    .add_plugins(super::#function_name)
                    .run_to_completion()
            }
        }
    }
    .into()
}
