use std::{str::FromStr, sync::Arc};

use color_eyre::{
    eyre::{bail, eyre},
    Result,
};
use communication::client::CyclerOutput;
use eframe::egui::{InnerResponse, Label, Response, ScrollArea, TextEdit, Ui, Widget};
use log::error;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use tokio::{runtime::Runtime, sync::mpsc, task::block_in_place};

use crate::{nao::Nao, panel::Panel, value_buffer::ValueBuffer};
use repository::{get_repository_root, HardwareIds, Repository};

// TODO move this elsewhere
fn get_last_octet_from_connection_url(connection_address: &str) -> Option<String> {
    // //  lazy_static! {
    //     static ref RE: Regex = Regex::new("...").unwrap();
    // }
    // RE.is_match(text)

    // Extract the ip address from a url like "ws://{ip_address}:1337"
    // pass: ws://10.12.34.13 OR ws://10.12.34.13:1234
    // fail: 10.12.34.13... ws://localhost OR ws://localhost:1234
    let re = Regex::new(
        r"(?m)^ws://(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)
    .(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)
    .(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)
    .(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)
    (?::[\d]*)?$",
    )
    .unwrap();

    let captures = re.captures(connection_address);
    captures.and_then(|capture| capture.get(1).and_then(|v| Some(v.as_str().to_string())))
}

struct RepoConfigurationHandler {
    repository: Repository,
    file_io_runtime: Runtime, // to call async functions.
}

impl RepoConfigurationHandler {
    fn new() -> Self {
        let runtime = Runtime::new().unwrap();
        let repo_root = runtime.block_on(get_repository_root()).unwrap();

        Self {
            repository: Repository::new(repo_root),
            file_io_runtime: runtime,
        }
    }

    fn get_hardware_ids_from_url(&self, connection_url: &str) -> Result<HardwareIds> {
        let nao_id_from_last_octet = get_last_octet_from_connection_url(connection_url)
            .and_then(|nao_id_str| nao_id_str.parse::<u8>().ok());

        match nao_id_from_last_octet {
            Some(nao_id) => {
                let ids = self
                    .file_io_runtime
                    .block_on(self.repository.get_hardware_ids())
                    .unwrap();

                ids.get(&nao_id).map_or(
                    Err(eyre!("Nao ID not found in hardware ID list.")),
                    |hardware_ids| -> Result<HardwareIds> { Ok(hardware_ids.clone()) },
                )
            }
            None => Err(eyre!("Nao ID couldn't be extracted from connection url")),
        }
    }
}

pub struct ManualCalibrationPanel {
    nao: Arc<Nao>,
    paths: [String; 2],
    camera_value_buffers: [ValueBuffer; 2],
    camera_rotation: [String; 2],
    update_notify_sender: mpsc::Sender<()>,
    update_notify_receiver: mpsc::Receiver<()>,
}

const CAMERA_KEY_BASE: &'static str = "camera_matrix_parameters.vision_";
const ROTATIONS: &'static str = ".extrinsic_rotations";

impl Panel for ManualCalibrationPanel {
    const NAME: &'static str = "Parameter";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let paths = ["top", "bottom"].map(|name| (CAMERA_KEY_BASE.to_owned() + name + ROTATIONS));
        let value_buffers = paths.clone().map(|path| nao.subscribe_parameter(&path));

        let (update_notify_sender, update_notify_receiver) = mpsc::channel(1);

        let connection_url = nao.get_address();
        println!("Address {:?}", connection_url);

        if let Some(url) = connection_url {
            let config_handler = RepoConfigurationHandler::new();
            let nao_hardware_ids = config_handler.get_hardware_ids_from_url(&url);

            if let Ok(hwardware_ids) = nao_hardware_ids {
                println!(
                    "Nao ID {:?} {:?}",
                    hwardware_ids.head_id, hwardware_ids.body_id
                );
            }
        }

        Self {
            nao,
            paths,
            camera_value_buffers: value_buffers,
            camera_rotation: [String::new(), String::new()],
            update_notify_sender,
            update_notify_receiver,
        }
    }
    // fn save(&self) -> Value {
    //     json!({
    //         "subscribe_key": self.path.clone()
    //     })
    // }
}

fn add_edit_components_for_one_camera(
    ui: &mut Ui,
    camera_index: usize,
    panel: &mut ManualCalibrationPanel,
) -> InnerResponse<Response> {
    let value_buffer = &panel.camera_value_buffers[camera_index];
    let rotation_value = &mut panel.camera_rotation[camera_index];
    let path = &panel.paths[camera_index];

    ui.horizontal(|ui| {
        let settable = !rotation_value.is_empty();
        ui.add_enabled_ui(settable, |ui| {
            if ui.button(format!("Set {}", camera_index)).clicked() {
                match serde_json::value::to_value(&rotation_value) {
                    Ok(value) => {
                        panel.nao.update_parameter_value(&path, value);
                    }
                    Err(error) => error!("Failed to serialize parameter value: {error:#?}"),
                }
            }
        });
        match value_buffer.get_latest() {
            Ok(value) => {
                if panel.update_notify_receiver.try_recv().is_ok() {
                    *rotation_value = serde_json::to_string_pretty(&value).unwrap();
                }
                // ScrollArea::vertical().show(ui, |ui: &mut Ui| {
                ui.add(Label::new(path));
                ui.add(Label::new("Rotations"));
                ui.add(
                    TextEdit::multiline(&mut rotation_value.clone())
                        .code_editor()
                        .desired_width(f32::INFINITY),
                )
                // });
            }
            Err(error) => ui.label(format!("{error:#?}")),
        }
    })
}

impl Widget for &mut ManualCalibrationPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            // ui.horizontal(|ui| {
            //     ui.label(format!("NAO Number: {}, head id {}, body id {}"));
            // });
            add_edit_components_for_one_camera(ui, 0, self);
            add_edit_components_for_one_camera(ui, 1, self);
        })
        .response
    }
}
