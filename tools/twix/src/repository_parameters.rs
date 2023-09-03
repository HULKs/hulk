use color_eyre::{
    eyre::{eyre, WrapErr},
    Result,
};
use log::error;
use parameters::{
    directory::{serialize, Id, Location, Scope},
    json::nest_value_at_path,
};
use repository::{get_repository_root, HardwareIds, Repository};
use serde_json::Value;
use std::{collections::HashMap, net::Ipv4Addr};
use tokio::runtime::Runtime;

pub struct RepositoryParameters {
    repository: Repository,
    runtime: Runtime,
    ids: HashMap<u8, HardwareIds>,
}

impl RepositoryParameters {
    pub fn try_new() -> Result<Self> {
        let runtime = Runtime::new()?;
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
        let Ok(hardware_ids) = self.hardware_ids_from_address(address) else {
            error!("failed to get head ID from address {address}");
            return;
        };
        let parameters = nest_value_at_path(&path, value);
        self.runtime.spawn(async move {
            serialize(
                &parameters,
                Scope {
                    location: Location::All,
                    id: Id::Head,
                },
                &path,
                repository.parameters_root(),
                &hardware_ids.body_id,
                &hardware_ids.head_id,
            )
            .await
            .unwrap();
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
            .cloned()
    }

    fn hardware_ids_from_nao_number(&self, nao_number: u8) -> Result<&HardwareIds> {
        self.ids
            .get(&nao_number)
            .ok_or_else(|| eyre!("no IDs known for NAO number {nao_number}"))
    }
}

fn last_octet_from_ip_address(ip_address: Ipv4Addr) -> u8 {
    ip_address.octets()[3]
}
