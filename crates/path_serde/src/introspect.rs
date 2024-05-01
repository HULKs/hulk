use std::collections::BTreeSet;

pub trait PathIntrospect {
    fn get_fields() -> BTreeSet<String> {
        let mut fields = BTreeSet::default();
        Self::extend_with_fields(&mut fields, "");
        fields
    }

    fn extend_with_fields(fields: &mut BTreeSet<String>, prefix: &str);
}
