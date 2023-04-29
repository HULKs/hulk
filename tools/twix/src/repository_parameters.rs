use color_eyre::{
    eyre::{eyre, WrapErr},
    Result,
};
use itertools::Itertools;
use log::error;
use parameters::json::merge_json;
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
        let Ok(hardware_ids) = self
            .hardware_ids_from_address(address)
        else {
            error!("failed to get head ID from address {address}");
            return
        };
        self.runtime.spawn(async move {
            let stored_complete_parameter_tree: Value = parameters::directory::deserialize(
                repository.root_directory(),
                &hardware_ids.body_id,
                &hardware_ids.head_id,
            )
            .await
            .unwrap_or_default();

            // value is just the "leaf" of the "path", make the tree structure to diff/ merge against complete parameter tree.
            let supplied_value_as_sparse_tree =
                make_sparse_value_tree_from_path(path.as_str(), &value);
            let diff = get_diff_against_stored_value(
                &stored_complete_parameter_tree,
                &supplied_value_as_sparse_tree,
            );
            let mut head_parameters = repository
                .read_configuration(&hardware_ids.head_id)
                .await
                .unwrap_or_default();
            merge_json(&mut head_parameters, &diff);

            if let Err(error) = repository
                .write_configuration(&hardware_ids.head_id, &head_parameters)
                .await
            {
                error!("Failed to write value to repository: {error:#?}");
            }
        });
    }

    fn hardware_ids_from_address(&self, address: &str) -> Result<HardwareIds> {
        if address == "localhost" {
            return Ok(HardwareIds {
                body_id: "webots".to_string(),
                head_id: "webots".to_string(),
            });
        }
        let nao_number =
            last_octet_from_ip_address(address.parse().wrap_err("failed to parse IP address")?);
        self.hardware_ids_from_nao_number(nao_number)
            .wrap_err_with(|| format!("failed to get head ID from NAO number {nao_number}"))
    }

    fn hardware_ids_from_nao_number(&self, nao_number: u8) -> Result<HardwareIds> {
        self.ids
            .get(&nao_number)
            .ok_or_else(|| eyre!("no IDs known for NAO number {nao_number}"))
            .cloned()
    }
}

fn last_octet_from_ip_address(ip_address: Ipv4Addr) -> u8 {
    ip_address.octets()[3]
}

// Create tree structure from path and value points to the last key i.e. a.b.c -> { a: { b: { c: value } } }
fn make_sparse_value_tree_from_path(path: &str, value: &Value) -> Value {
    path.split('.')
        .collect_vec()
        .into_iter()
        .rev()
        .fold(value.clone(), |child_value: Value, key: &str| -> Value {
            json!({ key: child_value })
        })
}

fn get_diff_against_stored_value(stored_value: &Value, incoming_sparse_value: &Value) -> Value {
    let mut diff = stored_value.clone();
    merge_json(&mut diff, incoming_sparse_value);
    prune_equal_branches(&mut diff, stored_value);
    diff
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::repository_parameters::get_diff_against_stored_value;

    use super::make_sparse_value_tree_from_path;

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
            assert_eq!(
                make_sparse_value_tree_from_path(path, &value),
                expected_output
            );
        }
    }

    #[test]
    fn sparse_value_diff() {
        let stored_value = json!({"a":{"b":[1,2,3], "c":10}, "x":1000});
        let incoming_sparse_value = json!({"a":{"b":[1,4,3], "c":10}});

        // Only "b" has changes.
        let expected_diff = json!({"a":{"b":[1,4,3]}});

        assert_eq!(
            get_diff_against_stored_value(&stored_value, &incoming_sparse_value),
            expected_diff
        );
    }
}
