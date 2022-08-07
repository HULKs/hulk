use module_attributes2::Attribute;
use syn::Ident;

#[derive(Clone, Debug)]
pub enum Edge {
    ConsumesFrom { attribute: Attribute },
    Contains,
    ContainsField { name: Ident },
    ReadsFrom { attribute: Attribute },
    ReadsFromOrWritesTo { attribute: Attribute },
    WritesTo { attribute: Attribute },
}
