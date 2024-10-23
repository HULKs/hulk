use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn scenario(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    let function_item = parse_macro_input!(item as ItemFn);
    let function_name = function_item.sig.ident.clone();

    quote! {
        #function_item

        fn main() -> color_eyre::Result<()> {
            use clap::Parser;
            use hulk_behavior_simulator::simulator::{AppExt, SimulatorPlugin};

            let args = hulk_behavior_simulator::scenario::Arguments::parse();

            App::new()
                .add_plugins(SimulatorPlugin::default().with_recording(!args.run))
                .add_plugins(#function_name)
                .run_to_completion()
        }

        #[cfg(test)]
        mod test {
            #[test]
            fn #function_name() -> color_eyre::Result<()> {
                use hulk_behavior_simulator::simulator::{AppExt, SimulatorPlugin};

                bevy::app::App::new()
                    .add_plugins(SimulatorPlugin::default())
                    .add_plugins(super::#function_name)
                    .run_to_completion()
            }
        }
    }
    .into()
}
