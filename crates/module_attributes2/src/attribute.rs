use syn::{Ident, TypePath};

macro_rules! attribute_parser {
    (@field_parsers $input_in_parentheses:ident $($field_name:ident: $field_type:ty),+) => {
        $(
            let attribute_parameter_name: syn::Ident = $input_in_parentheses.parse()?;
            if attribute_parameter_name != stringify!($field_name) {
                return Err(syn::Error::new(
                    attribute_parameter_name.span(),
                    format!("Unexpected parameter {attribute_parameter_name}, expected {}", stringify!($field_name)),
                ));
            }
            $input_in_parentheses.parse::<syn::Token![=]>()?;
            let $field_name: $field_type = $input_in_parentheses.parse()?;
            if !$input_in_parentheses.is_empty() {
                $input_in_parentheses.parse::<syn::Token![,]>()?;
            }
        )+
    };

    (@initializer $variant:ident $($field_name:ident: $field_type:ty),+) => {
        Ok(Attribute::$variant {
            $($field_name),+
        })
    };

    (pub enum Attribute { $($variant:ident { $($field:tt)+ }),+ }) => {
        #[derive(Debug)]
        pub enum Attribute {
            $($variant { $($field)+ } ),+
        }

        impl syn::parse::Parse for Attribute {
            fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                use convert_case::Casing;

                input.parse::<syn::Token![#]>()?;
                let input_in_brackets;
                syn::bracketed!(input_in_brackets in input);
                let name: syn::Ident = input_in_brackets.parse()?;
                let name_camel_case = name.to_string().to_case(convert_case::Case::Pascal);
                let input_in_parentheses;
                syn::parenthesized!(input_in_parentheses in input_in_brackets);
                match name_camel_case.as_str() {
                    $(stringify!($variant) => {
                        attribute_parser!(@field_parsers input_in_parentheses $($field)+);
                        attribute_parser!(@initializer $variant $($field)+)
                    },)+
                    _ => Err(syn::Error::new(
                        name.span(),
                        format!("Unexpected attribute {name}"),
                    ))
                }
            }
        }
    };
}

attribute_parser! {
    pub enum Attribute {
        RealtimeModule { cycler_module: Ident },
        PerceptionModule { cycler_module: Ident },
        PersistentState { data_type: TypePath, path: TypePath },
        MainOutput { data_type: TypePath, name: TypePath }
    }
}
