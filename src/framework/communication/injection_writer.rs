use log::error;
use serde_json::Value;
use serialize_hierarchy::SerializeHierarchy;
use tokio::{
    spawn,
    sync::{mpsc::Receiver, oneshot},
    task::JoinHandle,
};

use crate::{control, vision};

use super::{receiver::respond_or_log_error, runtime::ChannelsForInjectedOutputs, Cycler};

#[derive(Debug)]
pub enum Request {
    SetInjectedOutput {
        cycler: Cycler,
        path: String,
        data: Value,
        response_sender: oneshot::Sender<Result<(), &'static str>>,
    },
    UnsetInjectedOutput {
        cycler: Cycler,
        path: String,
        response_sender: oneshot::Sender<Result<(), &'static str>>,
    },
}

pub async fn injection_writer(
    mut request_receiver: Receiver<Request>,
    channels_for_control: ChannelsForInjectedOutputs<control::Database>,
    channels_for_vision_top: ChannelsForInjectedOutputs<vision::Database>,
    channels_for_vision_bottom: ChannelsForInjectedOutputs<vision::Database>,
) -> JoinHandle<()> {
    spawn(async move {
        let mut control_injection_database = Default::default();
        let mut vision_top_injection_database = Default::default();
        let mut vision_bottom_injection_database = Default::default();
        while let Some(request) = request_receiver.recv().await {
            handle_request(
                request,
                &channels_for_control,
                &channels_for_vision_top,
                &channels_for_vision_bottom,
                &mut control_injection_database,
                &mut vision_top_injection_database,
                &mut vision_bottom_injection_database,
            )
            .await;
        }
    })
}

async fn handle_request(
    request: Request,
    channels_for_control: &ChannelsForInjectedOutputs<control::Database>,
    channels_for_vision_top: &ChannelsForInjectedOutputs<vision::Database>,
    channels_for_vision_bottom: &ChannelsForInjectedOutputs<vision::Database>,
    control_injection_database: &mut control::Database,
    vision_top_injection_database: &mut vision::Database,
    vision_bottom_injection_database: &mut vision::Database,
) {
    let (cycler, path, data, response_sender) = match request {
        Request::SetInjectedOutput {
            cycler,
            path,
            data,
            response_sender,
        } => (cycler, path, data, response_sender),
        Request::UnsetInjectedOutput {
            cycler,
            path,
            response_sender,
        } => (cycler, path, Value::Null, response_sender),
    };
    let result = match cycler {
        Cycler::Audio => unimplemented!(),
        Cycler::Control => control_injection_database
            .main_outputs
            .deserialize_hierarchy(&path, data),
        Cycler::SplNetwork => unimplemented!(),
        Cycler::VisionTop => vision_top_injection_database
            .main_outputs
            .deserialize_hierarchy(&path, data),
        Cycler::VisionBottom => vision_bottom_injection_database
            .main_outputs
            .deserialize_hierarchy(&path, data),
    };
    if let Err(error) = result {
        error!("Failed to deserialize by path: {:?}", error);
        respond_or_log_error(response_sender, Err("Failed to deserialize"));
        return;
    }
    match cycler {
        Cycler::Audio => unimplemented!(),
        Cycler::Control => {
            let mut slot = channels_for_control.injected_outputs.next();
            *slot = control_injection_database.clone();
        }
        Cycler::SplNetwork => unimplemented!(),
        Cycler::VisionTop => {
            let mut slot = channels_for_vision_top.injected_outputs.next();
            *slot = vision_top_injection_database.clone();
        }
        Cycler::VisionBottom => {
            let mut slot = channels_for_vision_bottom.injected_outputs.next();
            *slot = vision_bottom_injection_database.clone();
        }
    }
    respond_or_log_error(response_sender, Ok(()));
}
