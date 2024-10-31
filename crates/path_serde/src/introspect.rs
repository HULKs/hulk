use std::collections::HashSet;

pub trait PathIntrospect {
    fn get_fields() -> HashSet<String> {
        let mut fields = HashSet::default();
        Self::extend_with_fields(&mut fields, "");
        fields
    }

    fn get_children() -> HashSet<String> {
        let mut fields = HashSet::default();
        Self::extend_with_children(&mut fields, "");
        fields
    }

    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str);
    fn extend_with_children(fields: &mut HashSet<String>, prefix: &str);
}
