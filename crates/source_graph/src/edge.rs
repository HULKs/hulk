use module_attributes2::Attribute;

#[derive(Clone, Debug)]
pub enum Edge {
    Contains,
    ConsumesFrom { attribute: Attribute },
    ReadsFrom { attribute: Attribute },
    WritesTo { attribute: Attribute },
    ReadsFromOrWritesTo { attribute: Attribute },
}
