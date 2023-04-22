use serde_json::Value;

pub mod client;
pub mod messages;
#[cfg(feature = "server")]
pub mod server;

pub fn merge_json(own: &mut Value, other: &Value) {
    match (own, other) {
        (&mut Value::Object(ref mut own), Value::Object(other)) => {
            for (key, value) in other {
                merge_json(own.entry(key.clone()).or_insert(Value::Null), value);
            }
        }
        (own, other) => {
            *own = other.clone();
        }
    }
}
