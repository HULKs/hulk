#[derive(Debug)]
pub struct AdditionalOutput<'context, DataType> {
    is_subscribed: bool,
    data: &'context mut Option<DataType>,
}

impl<'context, DataType> AdditionalOutput<'context, DataType> {
    pub fn new(is_subscribed: bool, data: &'context mut Option<DataType>) -> Self {
        Self {
            is_subscribed,
            data,
        }
    }

    pub fn fill_if_subscribed<Callback>(&mut self, callback: Callback)
    where
        Callback: FnOnce() -> DataType,
    {
        if self.is_subscribed {
            *self.data = Some(callback())
        }
    }

    pub fn mutate_if_subscribed<Callback>(&mut self, callback: Callback)
    where
        Callback: FnOnce(&mut Option<DataType>),
    {
        if self.is_subscribed {
            callback(self.data);
        }
    }

    pub fn is_subscribed(&self) -> bool {
        self.is_subscribed
    }
}

pub fn should_be_filled(subscribed_output: &str, additional_output_path: &str) -> bool {
    let (longer_path, shorter_path) = if subscribed_output.len() >= additional_output_path.len() {
        (subscribed_output, additional_output_path)
    } else {
        (additional_output_path, subscribed_output)
    };
    longer_path == shorter_path || longer_path.starts_with(&format!("{shorter_path}."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_be_filled_is_correct_for_type_hierarchy() {
        let cases = [
            ("a", "a", true),
            ("a.b", "a", true),
            ("a.b.c", "a", true),
            ("a", "a.b", true),
            ("a.b", "a.b", true),
            ("a.b.c", "a.b", true),
            ("a", "a.b.c", true),
            ("a.b", "a.b.c", true),
            ("a.b.c", "a.b.c", true),
            ("a.d", "a", true),
            ("a.d", "a.b", false),
            ("a.d", "a.b.c", false),
            ("a.b.d", "a", true),
            ("a.b.d", "a.b", true),
            ("a.b.d", "a.b.c", false),
            ("a.bd", "a.b", false),
        ];
        for (subscribed_output, additional_output_path, expected_should_be_filled) in cases {
            assert_eq!(
                should_be_filled(subscribed_output, additional_output_path),
                expected_should_be_filled,
                "subscribed_output={subscribed_output:?}, additional_output_path={additional_output_path:?}",
            );
        }
    }
}
