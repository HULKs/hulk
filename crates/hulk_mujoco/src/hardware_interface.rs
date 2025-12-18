use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use booster::{
    ButtonEventMsg, FallDownState, LowCommand, LowState, RemoteControllerState, TransformMessage,
};
use color_eyre::eyre::{eyre, Context, OptionExt};
use color_eyre::Result;
use futures_util::SinkExt;
use futures_util::StreamExt;
use hardware::{
    ButtonEventMsgInterface, CameraInterface, IdInterface, MicrophoneInterface, NetworkInterface,
    PathsInterface, RecordingInterface, SpeakerInterface, TimeInterface,
};
use hardware::{
    FallDownStateInterface, LowCommandInterface, LowStateInterface, RemoteControllerStateInterface,
    TransformMessageInterface,
};
use hula_types::hardware::{Ids, Paths};
use log::{error, info, warn};
use parking_lot::Mutex;
use serde::Deserialize;
use simulation_message::{ClientMessageKind, ConnectionInfo, ServerMessageKind, SimulatorMessage};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use types::audio::SpeakerRequest;
use types::messages::{IncomingMessage, OutgoingMessage};
use types::samples::Samples;
use zed::RGBDSensors;

use crate::HardwareInterface;

const CHANNEL_CAPACITY: usize = 32;

struct WorkerChannels {
    low_state_sender: Sender<LowState>,
    low_command_receiver: Receiver<LowCommand>,
    fall_down_sender: Sender<FallDownState>,
    button_event_msg_sender: Sender<ButtonEventMsg>,
    remote_controller_state_sender: Sender<RemoteControllerState>,
    transform_stamped_sender: Sender<TransformMessage>,
    rgbd_sensors_sender: Sender<RGBDSensors>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub paths: Paths,
    pub mujoco_websocket_address: String,
}

pub struct MujocoHardwareInterface {
    paths: Paths,
    enable_recording: AtomicBool,
    time: Arc<Mutex<SystemTime>>,

    low_state_receiver: Mutex<Receiver<LowState>>,
    low_command_sender: Sender<LowCommand>,
    fall_down_receiver: Mutex<Receiver<FallDownState>>,
    button_event_msg_receiver: Mutex<Receiver<ButtonEventMsg>>,
    remote_controller_state_receiver: Mutex<Receiver<RemoteControllerState>>,
    transform_stamped_receiver: Mutex<Receiver<TransformMessage>>,
    rgbd_sensors_receiver: Mutex<Receiver<RGBDSensors>>,
}

impl MujocoHardwareInterface {
    pub fn new(keep_running: CancellationToken, parameters: Parameters) -> Result<Self> {
        let (low_state_sender, low_state_receiver) = channel(CHANNEL_CAPACITY);
        let (low_command_sender, low_command_receiver) = channel(CHANNEL_CAPACITY);
        let (fall_down_sender, fall_down_receiver) = channel(CHANNEL_CAPACITY);
        let (button_event_msg_sender, button_event_msg_receiver) = channel(CHANNEL_CAPACITY);
        let (remote_controller_state_sender, remote_controller_state_receiver) =
            channel(CHANNEL_CAPACITY);
        let (transform_stamped_sender, transform_stamped_receiver) = channel(CHANNEL_CAPACITY);
        let (rgbd_sensors_sender, rgbd_sensors_receiver) = channel(CHANNEL_CAPACITY);

        let worker_channels = WorkerChannels {
            low_state_sender,
            low_command_receiver,
            fall_down_sender,
            button_event_msg_sender,
            remote_controller_state_sender,
            transform_stamped_sender,
            rgbd_sensors_sender,
        };

        let time = Arc::new(Mutex::new(SystemTime::UNIX_EPOCH));
        tokio::spawn(keep_running.clone().run_until_cancelled_owned(worker(
            time.clone(),
            parameters.mujoco_websocket_address,
            keep_running.clone(),
            worker_channels,
        )));

        Ok(Self {
            paths: parameters.paths,
            enable_recording: AtomicBool::new(false),
            time,

            low_state_receiver: Mutex::new(low_state_receiver),
            low_command_sender,
            fall_down_receiver: Mutex::new(fall_down_receiver),
            button_event_msg_receiver: Mutex::new(button_event_msg_receiver),
            remote_controller_state_receiver: Mutex::new(remote_controller_state_receiver),
            transform_stamped_receiver: Mutex::new(transform_stamped_receiver),
            rgbd_sensors_receiver: Mutex::new(rgbd_sensors_receiver),
        })
    }
}

async fn worker(
    time: Arc<Mutex<SystemTime>>,
    address: String,
    keep_running: CancellationToken,
    mut worker_channels: WorkerChannels,
) -> Result<()> {
    let mut websocket = loop {
        let websocket = tokio_tungstenite::connect_async(&address).await;
        if let Ok((mut websocket, _)) = websocket {
            let connection_info = ConnectionInfo::control_only();
            log::info!("connected to mujoco websocket at {address}");
            log::info!("sending ConnectionInfo");
            websocket
                .send(Message::binary(bincode::serialize(&connection_info)?))
                .await?;
            break websocket;
        };
        log::info!("connecting to websocket failed, retrying...");
        sleep(Duration::from_secs_f32(1.0)).await;
    };

    loop {
        tokio::select! {
            maybe_websocket_event = websocket.next() => {
                match maybe_websocket_event {
                    Some(Ok(message)) => handle_message(time.clone(), message, &worker_channels).await?,
                    Some(Err(error)) => error!("socket error {error}"),
                    None => break,
                }
            },
            maybe_low_command_event = worker_channels.low_command_receiver.recv() => {
                match maybe_low_command_event {
                    Some(low_command) => websocket.send(Message::binary(bincode::serialize(&ClientMessageKind::LowCommand(low_command))?)).await?,
                    None => break,
                };
            },
            _ = keep_running.cancelled() => break
        }
    }
    keep_running.cancel();
    Ok(())
}

async fn handle_message(
    hardware_interface_time: Arc<Mutex<SystemTime>>,
    message: Message,
    worker_channels: &WorkerChannels,
) -> Result<()> {
    let message = match message {
        Message::Binary(data) => bincode::deserialize(&data)?,
        Message::Close(maybe_frame) => {
            warn!("server closed connections: {maybe_frame:#?}");
            return Ok(());
        }
        _ => return Ok(()),
    };
    match message {
        SimulatorMessage {
            payload: ServerMessageKind::LowState(low_state),
            time,
        } => {
            *hardware_interface_time.lock() = time;
            worker_channels.low_state_sender.send(low_state).await?
        }
        SimulatorMessage {
            payload: ServerMessageKind::FallDownState(fall_down_state),
            time,
        } => {
            *hardware_interface_time.lock() = time;
            worker_channels
                .fall_down_sender
                .send(fall_down_state)
                .await?
        }
        SimulatorMessage {
            payload: ServerMessageKind::ButtonEventMsg(button_event_msg),
            time,
        } => {
            *hardware_interface_time.lock() = time;
            worker_channels
                .button_event_msg_sender
                .send(button_event_msg)
                .await?
        }
        SimulatorMessage {
            payload: ServerMessageKind::RemoteControllerState(remote_controller_state),
            time,
        } => {
            *hardware_interface_time.lock() = time;
            worker_channels
                .remote_controller_state_sender
                .send(remote_controller_state)
                .await?
        }
        SimulatorMessage {
            payload: ServerMessageKind::TransformMessage(transform_stamped),
            time,
        } => {
            *hardware_interface_time.lock() = time;
            worker_channels
                .transform_stamped_sender
                .send(transform_stamped)
                .await?
        }
        SimulatorMessage {
            payload: ServerMessageKind::RGBDSensors(rgbd_sensors),
            time,
        } => {
            *hardware_interface_time.lock() = time;
            worker_channels
                .rgbd_sensors_sender
                .send(*rgbd_sensors)
                .await?
        }
        _ => {
            info!("Received unexpected simulator data")
        }
    };

    Ok(())
}

impl LowStateInterface for MujocoHardwareInterface {
    fn read_low_state(&self) -> Result<LowState> {
        self.low_state_receiver
            .lock()
            .blocking_recv()
            .ok_or_eyre("low state channel closed")
    }
}

impl LowCommandInterface for MujocoHardwareInterface {
    fn write_low_command(&self, low_command: LowCommand) -> Result<()> {
        self.low_command_sender
            .blocking_send(low_command)
            .wrap_err("low command send error")
    }
}

impl FallDownStateInterface for MujocoHardwareInterface {
    fn read_fall_down_state(&self) -> Result<FallDownState> {
        self.fall_down_receiver
            .lock()
            .blocking_recv()
            .ok_or_eyre("fall down state channel closed")
    }
}

impl ButtonEventMsgInterface for MujocoHardwareInterface {
    fn read_button_event_msg(&self) -> Result<ButtonEventMsg> {
        self.button_event_msg_receiver
            .lock()
            .blocking_recv()
            .ok_or_eyre("button event msg channel closed")
    }
}

impl RemoteControllerStateInterface for MujocoHardwareInterface {
    fn read_remote_controller_state(&self) -> Result<RemoteControllerState> {
        self.remote_controller_state_receiver
            .lock()
            .blocking_recv()
            .ok_or_eyre("channel closed")
    }
}

impl TransformMessageInterface for MujocoHardwareInterface {
    fn read_transform_message(&self) -> Result<TransformMessage> {
        self.transform_stamped_receiver
            .lock()
            .blocking_recv()
            .ok_or_eyre("channel closed")
    }
}

impl CameraInterface for MujocoHardwareInterface {
    fn read_rgbd_sensors(&self) -> Result<RGBDSensors> {
        self.rgbd_sensors_receiver
            .lock()
            .blocking_recv()
            .ok_or_eyre("channel closed")
    }
}

impl TimeInterface for MujocoHardwareInterface {
    fn get_now(&self) -> SystemTime {
        *self.time.lock()
    }
}

impl PathsInterface for MujocoHardwareInterface {
    fn get_paths(&self) -> Paths {
        self.paths.clone()
    }
}

impl NetworkInterface for MujocoHardwareInterface {
    fn read_from_network(&self) -> Result<IncomingMessage> {
        todo!()
    }

    fn write_to_network(&self, _message: OutgoingMessage) -> Result<()> {
        todo!()
    }
}

impl SpeakerInterface for MujocoHardwareInterface {
    fn write_to_speakers(&self, _request: SpeakerRequest) {
        todo!()
    }
}

impl IdInterface for MujocoHardwareInterface {
    fn get_ids(&self) -> Ids {
        let name = "Booster K1";
        Ids {
            body_id: name.to_string(),
            head_id: name.to_string(),
        }
    }
}

impl MicrophoneInterface for MujocoHardwareInterface {
    fn read_from_microphones(&self) -> Result<Samples> {
        Err(eyre!("microphone interface is not implemented"))
    }
}

impl RecordingInterface for MujocoHardwareInterface {
    fn should_record(&self) -> bool {
        self.enable_recording.load(Ordering::SeqCst)
    }

    fn set_whether_to_record(&self, enable: bool) {
        self.enable_recording.store(enable, Ordering::SeqCst)
    }
}

impl HardwareInterface for MujocoHardwareInterface {}
