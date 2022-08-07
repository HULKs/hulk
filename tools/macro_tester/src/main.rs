use quote::ToTokens;
use source_graph::{source_graph_from, Edge, Node};

fn main() {
    // let file = source_graph::parse_file("src/spl_network2/mod.rs").unwrap();
    // println!("file: {file:#?}");
    // let cycler_instance = source_graph::get_cycler_instance_enum(&file);
    // println!("cycler_instance: {cycler_instance:#?}");
    // let module_implementation = source_graph::get_module_implementation(&file);
    // println!("module_implementation: {module_implementation:#?}");
    // let file = source_graph::parse_file("src/spl_network2/message_receiver.rs").unwrap();
    // println!("file: {file:#?}");
    // let cycler_instance = source_graph::get_cycler_instance_enum(&file);
    // println!("cycler_instance: {cycler_instance:#?}");
    // let module_implementation = source_graph::get_module_implementation(&file);
    // println!("module_implementation: {module_implementation:#?}");
    let graph = source_graph_from("src/spl_network2").unwrap();
    println!("digraph {{");
    for node_index in graph.node_indices() {
        println!(
            "\t{} [ label = \"{}\" ]",
            node_index.index(),
            match &graph[node_index] {
                Node::Configuration => "Configuration".to_string(),
                Node::CyclerInstance { instance } => format!("CyclerInstance({instance})"),
                Node::CyclerModule { module, path } =>
                    format!("CyclerModule({module}, {})", path.display()),
                Node::HardwareInterface => "HardwareInterface".to_string(),
                Node::Module { module } => format!("Module({})", module.module_identifier),
                Node::ParsedRustFile { file: _ } => "ParsedRustFile".to_string(),
                Node::RustFilePath { path } => format!("RustFilePath({})", path.display()),
                Node::Struct { name } => format!("Struct({name})"),
                Node::StructField { data_type } =>
                    format!("StructField({})", data_type.to_token_stream()),
                Node::Uses { .. } => "Uses".to_string(),
            }
        );
    }
    for edge_index in graph.edge_indices() {
        let (from, to) = graph.edge_endpoints(edge_index).unwrap();
        println!(
            "\t{} -> {} [ label = \"{}\" ]",
            from.index(),
            to.index(),
            match &graph[edge_index] {
                Edge::ConsumesFrom { attribute } => format!("ConsumesFrom({attribute})"),
                Edge::Contains => "Contains".to_string(),
                Edge::ContainsField { name } => format!("ContainsField({name})"),
                Edge::ReadsFrom { attribute } => format!("ReadsFrom({attribute})"),
                Edge::WritesTo { attribute } => format!("WritesTo({attribute})"),
                Edge::ReadsFromOrWritesTo { attribute } =>
                    format!("ReadsFromOrWritesTo({attribute})"),
            }
        );
    }
    println!("}}");
    // let mut file = File::open("src/spl_network2/message_receiver.rs").unwrap();
    // let mut buffer = String::new();
    // file.read_to_string(&mut buffer).unwrap();
    // let ast = parse_file(&buffer).unwrap();
    // let uses = uses_from_items(&ast.items);
    // println!("uses: {uses:?}");
    // for item in ast.items {
    //     let impl_item = match item {
    //         Item::Impl(impl_item) => impl_item,
    //         _ => continue,
    //     };
    //     if impl_item.attrs.is_empty() {
    //         continue;
    //     }
    //     let first_is_module_identifier = impl_item
    //         .attrs
    //         .first()
    //         .unwrap()
    //         .path
    //         .get_ident()
    //         .map_or(false, |identifier| identifier == "perception_module");
    //     if !first_is_module_identifier {
    //         continue;
    //     }
    //     match Module::from_implementation(impl_item) {
    //         Ok(module) => {
    //             println!("module: {:#?}", module);
    //             generate_main_outputs_database(&module, &uses)
    //         }
    //         Err(error) => println!("{:#?} {}", error.span(), error.to_compile_error()),
    //     }
    // }
}

// fn generate_main_outputs_database(module: &Module, uses: &Uses) {
//     let main_outputs = module.generate_main_output_fields(uses);
//     let database = quote! {
//         struct MainOutputs {
//             #(#main_outputs,)*
//         }
//     };

//     {
//         let mut file = File::create("database.rs").unwrap();
//         write!(file, "{}", database).unwrap();
//     }

//     let status = Command::new("rustfmt")
//         .arg("database.rs")
//         .status()
//         .expect("Failed to execute rustfmt");
//     if !status.success() {
//         panic!("rustfmt did not exit with success");
//     }
// }
