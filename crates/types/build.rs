use std::{collections::HashMap, env::var, fs::File, io::Write, path::PathBuf, process::Command};

use petgraph::visit::EdgeRef;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use source_graph::{source_graph_from, Edge, Node};

fn main() {
    let manifest_directory = PathBuf::from(var("CARGO_MANIFEST_DIR").unwrap());
    let out_path = PathBuf::from(var("OUT_DIR").unwrap());
    let project_root = manifest_directory
        .parent()
        .and_then(|crates_directory| crates_directory.parent())
        .expect("types crate must be located in crates/types subdirectory");

    let source_graph = source_graph_from(project_root.join("src/spl_network2"))
        .expect("Failed to generate source graph");

    let mut struct_index_stack: Vec<_> = source_graph
        .edge_references()
        .filter_map(|edge_reference| {
            match (
                &source_graph[edge_reference.source()],
                edge_reference.weight(),
                &source_graph[edge_reference.target()],
            ) {
                (Node::CyclerModule { module, .. }, Edge::Contains, Node::Struct { .. }) => {
                    Some((edge_reference.target(), Some(module)))
                }
                _ => None,
            }
        })
        .collect();
    struct_index_stack.push(
        source_graph
            .node_indices()
            .find_map(|node_index| match &source_graph[node_index] {
                Node::Struct { name } if name == "Configuration" => Some((node_index, None)),
                _ => None,
            })
            .expect("Failed to find Configuration struct in source graph"),
    );

    let mut structs: HashMap<Option<&String>, Vec<TokenStream>> = HashMap::new();
    while let Some((struct_index, cycler_module)) = struct_index_stack.pop() {
        let struct_fields = source_graph
            .edges(struct_index)
            .filter_map(|edge_reference| match edge_reference.weight() {
                Edge::ContainsField { name } => Some((edge_reference, name)),
                _ => None,
            })
            .map(|(edge_reference, struct_field_name)| {
                match &source_graph[edge_reference.target()] {
                    Node::Struct { name } => {
                        struct_index_stack.push((edge_reference.target(), cycler_module));
                        let name = format_ident!("{}", name);
                        quote! { pub #struct_field_name: #name }
                    }
                    Node::StructField { data_type } => {
                        quote! { pub #struct_field_name: #data_type }
                    }
                    _ => panic!(
                        "edge_reference.target() should refer to Node::Struct or Node::StructField"
                    ),
                }
            });
        let struct_name = match &source_graph[struct_index] {
            Node::Struct { name } => format_ident!("{}", name),
            _ => panic!("struct_index should refer to Node::Struct"),
        };
        structs.entry(cycler_module).or_default().push(quote! {
            #[derive(Clone, Debug, Deserialize, Serialize)]
            struct #struct_name {
                #(#struct_fields,)*
            }
        });
    }

    let items = structs
        .into_iter()
        .map(|(cycler_module, structs)| match cycler_module {
            Some(cycler_module) => {
                let cycler_module = format_ident!("{}", cycler_module);
                quote! {
                    mod #cycler_module {
                        #(#structs)*
                    }
                }
            }
            None => quote! {
                #(#structs)*
            },
        });

    let token_stream = quote! {
        use serde::{Deserialize, Serialize};

        #(#items)*
    };

    let file_path = out_path.join("structs.rs");
    {
        let mut file = File::create(&file_path)
            .unwrap_or_else(|_| panic!("Failed create file {:?}", file_path));
        write!(file, "{}", token_stream)
            .unwrap_or_else(|_| panic!("Failed to write to file {:?}", file_path));
    }

    let status = Command::new("rustfmt")
        .arg(file_path)
        .status()
        .expect("Failed to execute rustfmt");
    if !status.success() {
        panic!("rustfmt did not exit with success");
    }
}
