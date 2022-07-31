use quote::ToTokens;
use syn::{
    bracketed, parenthesized,
    parse::{Parse, ParseStream},
    parse2, Error, Ident, ItemImpl, Token, TypePath,
};

#[derive(Debug)]
pub struct Module {}

impl Module {
    pub fn from_implementation(implementation: ItemImpl) -> syn::Result<Self> {
        println!("attributes: {:#?}", implementation.attrs);
        let foo = implementation.attrs.first().unwrap().clone();
        let foo = foo.to_token_stream();
        println!("foo: {:#?}", foo);
        let attribute: Attribute = parse2(foo)?;
        println!("attribute: {:#?}", attribute);
        Ok(Self {})
    }
}

macro_rules! attribute_parser {
    (@field $($field_name:ident: $field_type:ty),+) => {
        $field_name: $field_type
    };

    (@field_parsers $input_in_parentheses:ident $valid_comma:ident $($field_name:ident: $field_type:ty),+) => {
        $(
            let attribute_parameter_name: Ident = $input_in_parentheses.parse()?;
            if attribute_parameter_name != stringify!($field_name) {
                return Err(Error::new(
                    attribute_parameter_name.span(),
                    format!("Unexpected parameter {attribute_parameter_name}, expected {}", stringify!($field_name)),
                ));
            }
            $input_in_parentheses.parse::<Token![=]>()?;
            let $field_name: $field_type = $input_in_parentheses.parse()?;
            if !$input_in_parentheses.is_empty() {
                $input_in_parentheses.parse::<Token![,]>()?;
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

        impl Parse for Attribute {
            fn parse(input: ParseStream) -> syn::Result<Self> {
                use convert_case::Casing;

                input.parse::<Token![#]>()?;
                let input_in_brackets;
                bracketed!(input_in_brackets in input);
                let name: Ident = input_in_brackets.parse()?;
                let name_camel_case = name.to_string().to_case(convert_case::Case::Pascal);
                let input_in_parentheses;
                parenthesized!(input_in_parentheses in input_in_brackets);
                match name_camel_case.as_str() {
                    $(stringify!($variant) => {
                        attribute_parser!(@field_parsers input_in_parentheses valid_comma $($field)+);
                        attribute_parser!(@initializer $variant $($field)+)
                    },)+
                    _ => Err(Error::new(
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
