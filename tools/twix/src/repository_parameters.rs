use color_eyre::{eyre::eyre, Result};
use communication::merge_json;
use itertools::Itertools;
use log::info;
use regex::Regex;
use repository::{get_repository_root, HardwareIds, Repository};
use serde_json::{json, Value};
use tokio::runtime::Runtime;

pub struct RepositoryParameters {
    repository: Repository,
    file_io_runtime: Runtime,
}

impl RepositoryParameters {
    pub fn new() -> Self {
        let runtime = Runtime::new().unwrap();
        let repo_root = runtime.block_on(get_repository_root()).unwrap();

        Self {
            repository: Repository::new(repo_root),
            file_io_runtime: runtime,
        }
    }

    pub fn get_hardware_ids_from_url(&self, connection_url: &str) -> Result<(HardwareIds, String)> {
        let nao_id_from_last_octet = get_last_octet_from_connection_url(connection_url);

        match nao_id_from_last_octet {
            Some(nao_id) => {
                let ids = self
                    .file_io_runtime
                    .block_on(self.repository.get_hardware_ids())
                    .unwrap();

                ids.get(&nao_id).map_or(
                    Err(eyre!("Nao ID not found in hardware ID list.")),
                    |hardware_ids| -> Result<(HardwareIds, String)> {
                        Ok((hardware_ids.clone(), nao_id.to_string()))
                    },
                )
            }
            None => match get_hardware_ids_if_webots(connection_url) {
                Some(hardware_ids) => Ok((hardware_ids, "webots".to_string())),
                None => Err(eyre!("Nao ID couldn't be extracted from connection url")),
            },
        }
    }

    pub fn print_nao_ids(&self, connection_url: Option<String>) {
        if let Some(url) = connection_url {
            let nao_hardware_ids = self.get_hardware_ids_from_url(&url);
            if let Ok((nao_hardware_ids, nao_id)) = nao_hardware_ids {
                info!(
                    "Connected to Nao {:?}, head: {:?}, body: {:?}",
                    nao_id, nao_hardware_ids.head_id, nao_hardware_ids.body_id
                );
            } else {
                println!("No hardware IDs associated with Nao ID found.");
            }
        } else {
            println!("Nao ID couldn't be determined from the URL");
        }
    }

    pub fn merge_head_configuration_to_repository(
        &self,
        head_id: &str,
        parameter_path: &str,
        value: &Value,
    ) -> Result<()> {
        let mut head_configuration_value: Value = self
            .file_io_runtime
            .block_on(self.repository.read_configuration(head_id))
            .unwrap_or_default();

        let tree_from_input_value = make_json_tree_from_path_and_value(parameter_path, value);

        merge_json(&mut head_configuration_value, &tree_from_input_value);

        self.file_io_runtime.block_on(
            self.repository
                .write_configuration(head_id, &head_configuration_value),
        )
    }
}

// Create tree structure from path and value points to the last key i.e. a.b.c -> { a: { b: { c: value } } }
fn make_json_tree_from_path_and_value(path: &str, value: &Value) -> Value {
    path.split('.')
        .collect_vec()
        .into_iter()
        .rev()
        .fold(value.clone(), |child_value: Value, key: &str| -> Value {
            json!({ key: child_value })
        })
}

fn get_last_octet_from_connection_url(connection_url: &str) -> Option<u8> {
    // Extract the ip address from a url like "ws://{ip_address}:1337"
    // pass: ws://10.12.34.13 OR ws://10.12.34.13:1234
    // fail: 10.12.34.13... ws://localhost OR ws://localhost:1234
    let re = Regex::new(
        r"(?x)^ws://(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)
            .(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)
            .(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)
            .(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)
            (?::[\d]*)?$",
    )
    .unwrap();

    let captures = re.captures(connection_url);
    captures.and_then(|capture| capture.get(1).and_then(|v| v.as_str().parse::<u8>().ok()))
}

fn get_hardware_ids_if_webots(connection_url: &str) -> Option<HardwareIds> {
    let re = Regex::new(r"(?x)^ws://localhost(?::[\d]*)?$").unwrap();

    if re.is_match(connection_url) {
        Some(HardwareIds {
            head_id: "webots".to_string(),
            body_id: "webots".to_string(),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use repository::HardwareIds;
    use serde_json::json;

    use super::{
        get_hardware_ids_if_webots, get_last_octet_from_connection_url,
        make_json_tree_from_path_and_value,
    };

    #[test]
    fn check_nao_id_extraction() {
        let dataset = [
            ("ws://10.12.34.13", Some(13)),
            ("ws://10.12.34.13:1234", Some(13)),
            ("10.12.34.13", None),
            ("ws://localhost", None),
            ("ws://localhost:1234", None),
        ];

        for (url, valid) in dataset {
            assert_eq!(get_last_octet_from_connection_url(url), valid);
        }
    }

    #[test]
    fn connection_url_is_webots() {
        let dataset = [
            ("ws://10.12.34.13", None),
            ("ws://10.12.34.13:1234", None),
            ("10.12.34.13", None),
            (
                "ws://localhost",
                Some(HardwareIds {
                    head_id: "webots".to_string(),
                    body_id: "webots".to_string(),
                }),
            ),
            (
                "ws://localhost:1234",
                Some(HardwareIds {
                    head_id: "webots".to_string(),
                    body_id: "webots".to_string(),
                }),
            ),
            ("localhost:1234", None),
        ];

        for (url, hardware_ids_option) in dataset {
            let ids_computed_option = get_hardware_ids_if_webots(url);

            assert_eq!(ids_computed_option.is_some(), hardware_ids_option.is_some());

            if let (Some(ids_computed), Some(ids_expected)) =
                (ids_computed_option, hardware_ids_option)
            {
                assert_eq!(ids_computed.head_id, ids_expected.head_id);
                assert_eq!(ids_computed.body_id, ids_expected.body_id);
            }
        }
    }

    #[test]
    fn check_json_structure_from_path_construction() {
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
                make_json_tree_from_path_and_value(path, &value),
                expected_output
            );
        }
    }
}
