use color_eyre::{
    eyre::{eyre, WrapErr},
    Result,
};
use communication::merge_json;
use itertools::Itertools;
use log::error;
use repository::{get_repository_root, HardwareIds, Repository};
use serde_json::{json, Value};
use std::{collections::HashMap, net::Ipv4Addr};
use tokio::runtime::Runtime;

pub struct RepositoryParameters {
    repository: Repository,
    runtime: Runtime,
    ids: HashMap<u8, HardwareIds>,
}

impl RepositoryParameters {
    pub fn try_default() -> Result<Self> {
        let runtime = Runtime::new().unwrap();
        let repository_root = runtime.block_on(get_repository_root())?;
        let repository = Repository::new(repository_root);
        let ids = runtime.block_on(repository.get_hardware_ids())?;

        Ok(Self {
            repository,
            runtime,
            ids,
        })
    }

    pub fn write(&self, address: &str, path: String, value: Value) {
        let repository = self.repository.clone();
        let Ok(head_id) = self
            .head_id_from_address(address)
        else {
            error!("failed to get head ID from address {address}");
            return
        };
        self.runtime.spawn(async move {
            let mut stored_value = repository
                .read_configuration(&head_id)
                .await
                .unwrap_or_default();

            let nested_value_to_be_added = nest_value_at_path(path.as_str(), &value);

            merge_json(&mut stored_value, &nested_value_to_be_added);

            if let Err(error) = repository
                .write_configuration(&head_id, &stored_value)
                .await
            {
                error!("Failed to write value to repository: {error:#?}");
            }
        });
    }

    fn head_id_from_address(&self, address: &str) -> Result<String> {
        if address == "localhost" {
            return Ok("webots".to_string());
        }
        let nao_number =
            last_octet_from_ip_address(address.parse().wrap_err("failed to parse IP address")?);
        self.head_id_from_nao_number(nao_number)
            .wrap_err_with(|| format!("failed to get head ID from NAO number {nao_number}"))
    }

    fn head_id_from_nao_number(&self, nao_number: u8) -> Result<String> {
        self.ids
            .get(&nao_number)
            .ok_or_else(|| eyre!("no IDs known for NAO number {nao_number}"))
            .map(|id| id.head_id.clone())
    }
}

fn last_octet_from_ip_address(ip_address: Ipv4Addr) -> u8 {
    ip_address.octets()[3]
}

// Create tree structure from path and value points to the last key i.e. a.b.c -> { a: { b: { c: value } } }
fn nest_value_at_path(path: &str, value: &Value) -> Value {
    path.split('.')
        .collect_vec()
        .into_iter()
        .rev()
        .fold(value.clone(), |child_value: Value, key: &str| -> Value {
            json!({ key: child_value })
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::nest_value_at_path;

    #[test]
    fn values_are_nested_at_paths() {
        let dataset = [
            (
                ("config.a.b.c", json!(["p", "q", "r"])),
                json!({"config":{"a":{"b":{"c":["p", "q", "r"]}}}}),
            ),
            (
                ("top.rotations", json!([1, 2, 3])),
                json!({"top":{"rotations":[1,2,3]}}),
            ),
            (
                ("something.properties", json!({"k":"v"})),
                json!({"something":{"properties":{"k":"v"}}}),
            ),
        ];

        for ((path, value), expected_output) in dataset {
            assert_eq!(nest_value_at_path(path, &value), expected_output);
        }
    }
}
