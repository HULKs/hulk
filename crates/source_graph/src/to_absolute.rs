use syn::{GenericArgument, Path, PathArguments, PathSegment, ReturnType, Type, TypeParamBound};

use crate::parser::Uses;

pub trait ToAbsolute {
    fn to_absolute(&self, uses: &Uses) -> Self;
}

impl ToAbsolute for PathArguments {
    fn to_absolute(&self, uses: &Uses) -> Self {
        let mut path_arguments = self.clone();
        match &mut path_arguments {
            PathArguments::AngleBracketed(angle_bracketed) => {
                for argument in angle_bracketed.args.iter_mut() {
                    match argument {
                        GenericArgument::Lifetime(_) => {}
                        GenericArgument::Type(argument_type) => {
                            *argument_type = argument_type.to_absolute(uses);
                        }
                        GenericArgument::Binding(binding) => {
                            binding.ty = binding.ty.to_absolute(uses);
                        }
                        GenericArgument::Constraint(constraint) => {
                            for bound in constraint.bounds.iter_mut() {
                                if let TypeParamBound::Trait(trait_bound) = bound {
                                    trait_bound.path = trait_bound.path.to_absolute(uses);
                                }
                            }
                        }
                        GenericArgument::Const(_) => {}
                    }
                }
            }
            _ => {}
        }
        path_arguments
    }
}

impl ToAbsolute for Path {
    fn to_absolute(&self, uses: &Uses) -> Self {
        let prefix = self
            .segments
            .first()
            .and_then(|first_segment| uses.get(&first_segment.ident));
        Path {
            leading_colon: self.leading_colon,
            segments: match prefix {
                Some(prefix) => prefix
                    .iter()
                    .enumerate()
                    .map(|(index, identifier)| PathSegment {
                        ident: identifier.clone(),
                        arguments: if index < prefix.len() - 1 {
                            PathArguments::None
                        } else {
                            self.segments.first().unwrap().arguments.to_absolute(uses)
                        },
                    })
                    .chain(self.segments.iter().skip(1).map(|segment| PathSegment {
                        ident: segment.ident.clone(),
                        arguments: segment.arguments.to_absolute(uses),
                    }))
                    .collect(),
                None => self
                    .segments
                    .iter()
                    .map(|segment| PathSegment {
                        ident: segment.ident.clone(),
                        arguments: segment.arguments.to_absolute(uses),
                    })
                    .collect(),
            },
        }
    }
}

impl ToAbsolute for Type {
    fn to_absolute(&self, uses: &Uses) -> Self {
        let mut data_type = self.clone();
        match &mut data_type {
            Type::Array(array) => {
                array.elem = Box::new(array.elem.to_absolute(uses));
            }
            Type::BareFn(function) => {
                for input in function.inputs.iter_mut() {
                    input.ty = input.ty.to_absolute(uses);
                }
                if let ReturnType::Type(_arrow, return_type) = &mut function.output {
                    *return_type = Box::new(return_type.to_absolute(uses));
                }
            }
            Type::Group(group) => {
                group.elem = Box::new(group.elem.to_absolute(uses));
            }
            Type::ImplTrait(trait_implementation) => {
                for bound in trait_implementation.bounds.iter_mut() {
                    if let TypeParamBound::Trait(trait_bound) = bound {
                        trait_bound.path = trait_bound.path.to_absolute(uses);
                    }
                }
            }
            Type::Infer(_) => {}
            Type::Macro(macro_type) => {
                macro_type.mac.path = macro_type.mac.path.to_absolute(uses);
            }
            Type::Never(_) => {}
            Type::Paren(parenthesized) => {
                parenthesized.elem = Box::new(parenthesized.elem.to_absolute(uses));
            }
            Type::Path(path) => {
                if let Some(qself) = &mut path.qself {
                    qself.ty = Box::new(qself.ty.to_absolute(uses));
                }
                path.path = path.path.to_absolute(uses);
            }
            Type::Ptr(pointer) => {
                pointer.elem = Box::new(pointer.elem.to_absolute(uses));
            }
            Type::Reference(reference) => {
                reference.elem = Box::new(reference.elem.to_absolute(uses));
            }
            Type::Slice(slice) => {
                slice.elem = Box::new(slice.elem.to_absolute(uses));
            }
            Type::TraitObject(trait_object) => {
                for bound in trait_object.bounds.iter_mut() {
                    if let TypeParamBound::Trait(trait_bound) = bound {
                        trait_bound.path = trait_bound.path.to_absolute(uses);
                    }
                }
            }
            Type::Tuple(tuple) => {
                for element in tuple.elems.iter_mut() {
                    *element = element.to_absolute(uses);
                }
            }
            Type::Verbatim(_) => {}
            _ => panic!("Type not implemented"),
        }
        data_type
    }
}
