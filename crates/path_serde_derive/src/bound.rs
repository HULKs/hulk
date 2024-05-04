use syn::{
    punctuated::Punctuated, token::Plus, BoundLifetimes, DeriveInput, Generics, PredicateType,
    Type, TypeParamBound, TypePath, WherePredicate,
};
use syn::{GenericParam, Token};

pub trait ExtendGenerics {
    fn remove_defaults(&mut self);
    fn extend_with_bounds(&mut self, bounds: Vec<WherePredicate>);
}

impl ExtendGenerics for Generics {
    fn remove_defaults(&mut self) {
        self.params.iter_mut().for_each(|param| {
            if let GenericParam::Type(param) = param {
                param.eq_token = None;
                param.default = None;
            }
        });
    }

    fn extend_with_bounds(&mut self, bounds: Vec<WherePredicate>) {
        self.make_where_clause().predicates.extend(bounds);
    }
}

pub fn infer_predicates(
    input: &DeriveInput,
    bounds: Punctuated<TypeParamBound, Plus>,
    lifetimes: Option<BoundLifetimes>,
) -> Vec<WherePredicate> {
    input
        .generics
        .type_params()
        .map(|param| {
            Type::Path(TypePath {
                qself: None,
                path: param.ident.clone().into(),
            })
        })
        .map(|bounded_ty| {
            WherePredicate::Type(PredicateType {
                lifetimes: lifetimes.clone(),
                bounded_ty,
                colon_token: <Token![:]>::default(),
                bounds: bounds.clone(),
            })
        })
        .collect()
}
