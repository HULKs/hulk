use std::collections::BTreeMap;

pub type ParameterKey = String;
pub type FieldPath = String;
pub type LayerPath = String;
pub type ProvenanceMap = BTreeMap<FieldPath, LayerPath>;
