use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    net::SocketAddr,
    num::Wrapping,
    sync::Arc,
};

use byteorder::{ByteOrder, LittleEndian};
use futures_util::future::join_all;
use image::codecs::jpeg::JpegEncoder;
use log::error;
use serde::Serialize;
use serialize_hierarchy::{HierarchyType, SerializeHierarchy};
use tokio::{
    select, spawn,
    sync::{
        mpsc::{self, Receiver},
        oneshot, Notify,
    },
    task::JoinHandle,
};
use types::{Rgb, YCbCr444};

use crate::{
    audio, control,
    framework::buffer::{Reader, Writer},
    spl_network, vision,
};

use super::{
    receiver::respond_or_log_error,
    sender::{Message, Payload, SubscribedOutput},
    ChannelsForDatabases, ChannelsForDatabasesWithImage, Cycler, CyclerOutput, Output,
};

#[derive(Debug)]
pub enum Request {
    GetOutputHierarchy {
        response_sender: oneshot::Sender<OutputHierarchy>,
    },
    SubscribeOutput {
        client: SocketAddr,
        output: CyclerOutput,
        response_sender: oneshot::Sender<Result<(), &'static str>>,
        output_sender: mpsc::Sender<Message>,
    },
    UnsubscribeOutput {
        client: SocketAddr,
        output: CyclerOutput,
        response_sender: oneshot::Sender<Result<(), &'static str>>,
    },
    UnsubscribeEverything {
        client: SocketAddr,
    },
}

#[derive(Debug)]
struct Peer {
    output_sender: mpsc::Sender<Message>,
    paths: HashSet<Output>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CyclerOutputsHierarchy {
    pub main: HierarchyType,
    pub additional: HierarchyType,
}

#[derive(Clone, Debug, Serialize)]
pub struct OutputHierarchy {
    pub audio: CyclerOutputsHierarchy,
    pub control: CyclerOutputsHierarchy,
    pub spl_network: CyclerOutputsHierarchy,
    pub vision_top: CyclerOutputsHierarchy,
    pub vision_bottom: CyclerOutputsHierarchy,
}

struct ChangingDatabase<T> {
    database: Reader<T>,
    changed: Arc<Notify>,
}

struct ChangingDatabases {
    audio: ChangingDatabase<audio::Database>,
    control: ChangingDatabase<control::Database>,
    spl_network: ChangingDatabase<spl_network::Database>,
    vision_top: ChangingDatabase<vision::Database>,
    vision_bottom: ChangingDatabase<vision::Database>,
}

struct SubscribedOutputs {
    additional_outputs_for_audio: Writer<HashSet<String>>,

    additional_outputs_for_spl_network: Writer<HashSet<String>>,

    additional_outputs_for_control: Writer<HashSet<String>>,

    additional_outputs_for_vision_top: Writer<HashSet<String>>,
    image_for_vision_top: Writer<bool>,

    additional_outputs_for_vision_bottom: Writer<HashSet<String>>,
    image_for_vision_bottom: Writer<bool>,
}

impl SubscribedOutputs {
    fn write(&self, subscribed_peers: &HashMap<Cycler, HashMap<SocketAddr, Peer>>) {
        let mut additional_outputs_for_audio = HashSet::new();
        let mut additional_outputs_for_control = HashSet::new();
        let mut additional_outputs_for_spl_network = HashSet::new();
        let mut additional_outputs_for_vision_top = HashSet::new();
        let mut image_for_vision_top = false;
        let mut additional_outputs_for_vision_bottom = HashSet::new();
        let mut image_for_vision_bottom = false;
        for (cycler, peers) in subscribed_peers.iter() {
            for peer in peers.values() {
                for output in peer.paths.iter() {
                    match (cycler, output) {
                        (Cycler::Audio, Output::Additional { path }) => {
                            additional_outputs_for_audio.insert(path.clone());
                        }
                        (Cycler::Control, Output::Additional { path }) => {
                            additional_outputs_for_control.insert(path.clone());
                        }
                        (Cycler::SplNetwork, Output::Additional { path }) => {
                            additional_outputs_for_spl_network.insert(path.clone());
                        }
                        (Cycler::VisionTop, Output::Additional { path }) => {
                            additional_outputs_for_vision_top.insert(path.clone());
                        }
                        (Cycler::VisionTop, Output::Image) => {
                            image_for_vision_top = true;
                        }
                        (Cycler::VisionBottom, Output::Additional { path }) => {
                            additional_outputs_for_vision_bottom.insert(path.clone());
                        }
                        (Cycler::VisionBottom, Output::Image) => {
                            image_for_vision_bottom = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        let mut additional_outputs_for_audio_slot = self.additional_outputs_for_audio.next();
        *additional_outputs_for_audio_slot = additional_outputs_for_audio;
        let mut additional_outputs_for_control_slot = self.additional_outputs_for_control.next();
        *additional_outputs_for_control_slot = additional_outputs_for_control;
        let mut additional_outputs_for_spl_network_slot =
            self.additional_outputs_for_spl_network.next();
        *additional_outputs_for_spl_network_slot = additional_outputs_for_spl_network;
        let mut additional_outputs_for_vision_top_slot =
            self.additional_outputs_for_vision_top.next();
        *additional_outputs_for_vision_top_slot = additional_outputs_for_vision_top;
        let mut image_for_vision_top_slot = self.image_for_vision_top.next();
        *image_for_vision_top_slot = image_for_vision_top;
        let mut additional_outputs_for_vision_bottom_slot =
            self.additional_outputs_for_vision_bottom.next();
        *additional_outputs_for_vision_bottom_slot = additional_outputs_for_vision_bottom;
        let mut image_for_vision_bottom_slot = self.image_for_vision_bottom.next();
        *image_for_vision_bottom_slot = image_for_vision_bottom;
    }
}

fn split_channels(
    channels_for_audio: ChannelsForDatabases<audio::Database>,
    channels_for_control: ChannelsForDatabases<control::Database>,
    channels_for_spl_network: ChannelsForDatabases<spl_network::Database>,
    channels_for_vision_top: ChannelsForDatabasesWithImage<vision::Database>,
    channels_for_vision_bottom: ChannelsForDatabasesWithImage<vision::Database>,
) -> (ChangingDatabases, SubscribedOutputs) {
    (
        ChangingDatabases {
            audio: ChangingDatabase {
                database: channels_for_audio.database,
                changed: channels_for_audio.database_changed,
            },
            spl_network: ChangingDatabase {
                database: channels_for_spl_network.database,
                changed: channels_for_spl_network.database_changed,
            },
            control: ChangingDatabase {
                database: channels_for_control.database,
                changed: channels_for_control.database_changed,
            },
            vision_top: ChangingDatabase {
                database: channels_for_vision_top.database,
                changed: channels_for_vision_top.database_changed,
            },
            vision_bottom: ChangingDatabase {
                database: channels_for_vision_bottom.database,
                changed: channels_for_vision_bottom.database_changed,
            },
        },
        SubscribedOutputs {
            additional_outputs_for_audio: channels_for_audio.subscribed_additional_outputs,
            additional_outputs_for_control: channels_for_control.subscribed_additional_outputs,
            additional_outputs_for_spl_network: channels_for_spl_network
                .subscribed_additional_outputs,
            additional_outputs_for_vision_top: channels_for_vision_top
                .subscribed_additional_outputs,
            image_for_vision_top: channels_for_vision_top.subscribed_image,
            additional_outputs_for_vision_bottom: channels_for_vision_bottom
                .subscribed_additional_outputs,
            image_for_vision_bottom: channels_for_vision_bottom.subscribed_image,
        },
    )
}

pub async fn database_subscription_manager(
    mut request_receiver: Receiver<Request>,
    channels_for_audio: ChannelsForDatabases<audio::Database>,
    channels_for_control: ChannelsForDatabases<control::Database>,
    channels_for_spl_network: ChannelsForDatabases<spl_network::Database>,
    channels_for_vision_top: ChannelsForDatabasesWithImage<vision::Database>,
    channels_for_vision_bottom: ChannelsForDatabasesWithImage<vision::Database>,
) -> JoinHandle<()> {
    spawn(async move {
        let outputs_hierarchy = OutputHierarchy {
            audio: CyclerOutputsHierarchy {
                main: audio::MainOutputs::get_hierarchy(),
                additional: audio::AdditionalOutputs::get_hierarchy(),
            },
            control: CyclerOutputsHierarchy {
                main: control::MainOutputs::get_hierarchy(),
                additional: control::AdditionalOutputs::get_hierarchy(),
            },
            spl_network: CyclerOutputsHierarchy {
                main: spl_network::MainOutputs::get_hierarchy(),
                additional: spl_network::AdditionalOutputs::get_hierarchy(),
            },
            vision_top: CyclerOutputsHierarchy {
                main: vision::MainOutputs::get_hierarchy(),
                additional: vision::AdditionalOutputs::get_hierarchy(),
            },
            vision_bottom: CyclerOutputsHierarchy {
                main: vision::MainOutputs::get_hierarchy(),
                additional: vision::AdditionalOutputs::get_hierarchy(),
            },
        };
        let (databases, subscribed_outputs) = split_channels(
            channels_for_audio,
            channels_for_control,
            channels_for_spl_network,
            channels_for_vision_top,
            channels_for_vision_bottom,
        );
        let mut subscribed_peers = HashMap::new();
        let mut next_image_id = Wrapping(0);
        loop {
            select! {
                request = request_receiver.recv() => {
                    let request = match request {
                        Some(request) => request,
                        None => {
                            break;
                        },
                    };
                    handle_request(request, &outputs_hierarchy,&mut subscribed_peers).await;
                    subscribed_outputs.write(&subscribed_peers);
                },
                _ = databases.audio.changed.notified() => {
                    handle_audio_notification(&databases.audio.database, &subscribed_peers).await;
                },
                _ = databases.control.changed.notified() => {
                    handle_control_notification(&databases.control.database, &subscribed_peers).await;
                },
                _ = databases.spl_network.changed.notified() => {
                    handle_spl_network_notification(&databases.spl_network.database, &subscribed_peers).await;
                },
                _ = databases.vision_top.changed.notified() => {
                    handle_vision_notification(Cycler::VisionTop, &databases.vision_top.database, &mut next_image_id, &subscribed_peers).await;
                },
                _ = databases.vision_bottom.changed.notified() => {
                    handle_vision_notification(Cycler::VisionBottom, &databases.vision_bottom.database, &mut next_image_id, &subscribed_peers).await;
                },
            }
        }
    })
}

async fn handle_request(
    request: Request,
    output_hierarchy: &OutputHierarchy,
    subscribed_peers: &mut HashMap<Cycler, HashMap<SocketAddr, Peer>>,
) {
    match request {
        Request::GetOutputHierarchy { response_sender } => {
            respond_or_log_error(response_sender, output_hierarchy.clone());
        }
        Request::SubscribeOutput {
            client,
            output,
            response_sender,
            output_sender,
        } => {
            let response = handle_subscribe_output(client, output, output_sender, subscribed_peers);
            respond_or_log_error(response_sender, response);
        }
        Request::UnsubscribeOutput {
            client,
            output,
            response_sender,
        } => {
            let response = handle_unsubscribe_output(client, output, subscribed_peers);
            respond_or_log_error(response_sender, response);
        }
        Request::UnsubscribeEverything { client } => {
            handle_unsubscribe_everything(client, subscribed_peers);
        }
    }
}

fn handle_subscribe_output(
    client: SocketAddr,
    output: CyclerOutput,
    output_sender: mpsc::Sender<Message>,
    subscribed_peers: &mut HashMap<Cycler, HashMap<SocketAddr, Peer>>,
) -> Result<(), &'static str> {
    let path_exists = match output.cycler {
        Cycler::Audio => match &output.output {
            Output::Main { path } => audio::MainOutputs::exists(path),
            Output::Additional { path } => audio::AdditionalOutputs::exists(path),
            Output::Image => false,
        },
        Cycler::Control => match &output.output {
            Output::Main { path } => control::MainOutputs::exists(path),
            Output::Additional { path } => control::AdditionalOutputs::exists(path),
            Output::Image => false,
        },
        Cycler::SplNetwork => match &output.output {
            Output::Main { path } => spl_network::MainOutputs::exists(path),
            Output::Additional { path } => spl_network::AdditionalOutputs::exists(path),
            Output::Image => false,
        },
        Cycler::VisionTop => match &output.output {
            Output::Main { path } => vision::MainOutputs::exists(path),
            Output::Additional { path } => vision::AdditionalOutputs::exists(path),
            Output::Image => true,
        },
        Cycler::VisionBottom => match &output.output {
            Output::Main { path } => vision::MainOutputs::exists(path),
            Output::Additional { path } => vision::AdditionalOutputs::exists(path),
            Output::Image => true,
        },
    };
    if !path_exists {
        return Err("Path does not exist");
    }
    let peers = subscribed_peers.entry(output.cycler).or_default();
    let peer = peers.entry(client).or_insert_with(|| Peer {
        output_sender,
        paths: Default::default(),
    });
    if !peer.paths.insert(output.output) {
        return Err("Already subscribed");
    }
    Ok(())
}

fn handle_unsubscribe_output(
    client: SocketAddr,
    output: CyclerOutput,
    subscribed_peers: &mut HashMap<Cycler, HashMap<SocketAddr, Peer>>,
) -> Result<(), &'static str> {
    let peers = subscribed_peers
        .get_mut(&output.cycler)
        .ok_or("Not subscribed (cycler not registered)")?;
    let peer = peers
        .get_mut(&client)
        .ok_or("Not subscribed (client not registered)")?;
    if !peer.paths.remove(&output.output) {
        return Err("Not subscribed (path not registered)");
    }
    if peer.paths.is_empty() {
        peers.remove(&client);
    }
    if peers.is_empty() {
        subscribed_peers.remove(&output.cycler);
    }
    Ok(())
}

fn handle_unsubscribe_everything(
    client: SocketAddr,
    subscribed_peers: &mut HashMap<Cycler, HashMap<SocketAddr, Peer>>,
) {
    subscribed_peers.retain(|_cycler, peers| {
        peers.remove(&client);
        !peers.is_empty()
    });
}

async fn handle_audio_notification(
    database_reader: &Reader<audio::Database>,
    subscribed_peers: &HashMap<Cycler, HashMap<SocketAddr, Peer>>,
) {
    let peers = match subscribed_peers.get(&Cycler::Audio) {
        Some(peers) => peers,
        None => return,
    };
    let mut send_futures = vec![];
    {
        let database = database_reader.next();
        for peer in peers.values() {
            let mut outputs = vec![];
            for output in peer.paths.iter() {
                match output {
                    Output::Main { path } => {
                        let data = match database.main_outputs.serialize_hierarchy(path) {
                            Ok(data) => data,
                            Err(error) => {
                                error!("Failed to serialize by path: {:?}", error);
                                continue;
                            }
                        };
                        outputs.push(SubscribedOutput {
                            output: output.clone(),
                            data,
                        });
                    }
                    Output::Additional { path } => {
                        let data = match database.additional_outputs.serialize_hierarchy(path) {
                            Ok(data) => data,
                            Err(error) => {
                                error!("Failed to serialize by path: {:?}", error);
                                continue;
                            }
                        };
                        outputs.push(SubscribedOutput {
                            output: output.clone(),
                            data,
                        });
                    }
                    Output::Image => {
                        panic!("Unexpected subscription for image in audio cycler")
                    }
                }
            }
            send_futures.push(peer.output_sender.send(Message::Json {
                payload: Payload::OutputsUpdated {
                    cycler: Cycler::Audio,
                    outputs,
                    image_id: None,
                },
            }));
        }
    }
    for send_result in join_all(send_futures).await {
        if let Err(error) = send_result {
            error!(
                "Failed to send message into channel for sender: {:?}",
                error
            );
        }
    }
}

async fn handle_control_notification(
    database_reader: &Reader<control::Database>,
    subscribed_peers: &HashMap<Cycler, HashMap<SocketAddr, Peer>>,
) {
    let peers = match subscribed_peers.get(&Cycler::Control) {
        Some(peers) => peers,
        None => return,
    };
    let mut send_futures = vec![];
    {
        let database = database_reader.next();
        for peer in peers.values() {
            let mut outputs = vec![];
            for output in peer.paths.iter() {
                match output {
                    Output::Main { path } => {
                        let data = match database.main_outputs.serialize_hierarchy(path) {
                            Ok(data) => data,
                            Err(error) => {
                                error!("Failed to serialize by path: {:?}", error);
                                continue;
                            }
                        };
                        outputs.push(SubscribedOutput {
                            output: output.clone(),
                            data,
                        });
                    }
                    Output::Additional { path } => {
                        let data = match database.additional_outputs.serialize_hierarchy(path) {
                            Ok(data) => data,
                            Err(error) => {
                                error!("Failed to serialize by path: {:?}", error);
                                continue;
                            }
                        };
                        outputs.push(SubscribedOutput {
                            output: output.clone(),
                            data,
                        });
                    }
                    Output::Image => panic!("Unexpected subscription for image in control cycler"),
                }
            }
            send_futures.push(peer.output_sender.send(Message::Json {
                payload: Payload::OutputsUpdated {
                    cycler: Cycler::Control,
                    outputs,
                    image_id: None,
                },
            }));
        }
    }
    for send_result in join_all(send_futures).await {
        if let Err(error) = send_result {
            error!(
                "Failed to send message into channel for sender: {:?}",
                error
            );
        }
    }
}

async fn handle_spl_network_notification(
    database_reader: &Reader<spl_network::Database>,
    subscribed_peers: &HashMap<Cycler, HashMap<SocketAddr, Peer>>,
) {
    let peers = match subscribed_peers.get(&Cycler::SplNetwork) {
        Some(peers) => peers,
        None => return,
    };
    let mut send_futures = vec![];
    {
        let database = database_reader.next();
        for peer in peers.values() {
            let mut outputs = vec![];
            for output in peer.paths.iter() {
                match output {
                    Output::Main { path } => {
                        let data = match database.main_outputs.serialize_hierarchy(path) {
                            Ok(data) => data,
                            Err(error) => {
                                error!("Failed to serialize by path: {:?}", error);
                                continue;
                            }
                        };
                        outputs.push(SubscribedOutput {
                            output: output.clone(),
                            data,
                        });
                    }
                    Output::Additional { path } => {
                        let data = match database.additional_outputs.serialize_hierarchy(path) {
                            Ok(data) => data,
                            Err(error) => {
                                error!("Failed to serialize by path: {:?}", error);
                                continue;
                            }
                        };
                        outputs.push(SubscribedOutput {
                            output: output.clone(),
                            data,
                        });
                    }
                    Output::Image => {
                        panic!("Unexpected subscription for image in spl_network cycler")
                    }
                }
            }
            send_futures.push(peer.output_sender.send(Message::Json {
                payload: Payload::OutputsUpdated {
                    cycler: Cycler::SplNetwork,
                    outputs,
                    image_id: None,
                },
            }));
        }
    }
    for send_result in join_all(send_futures).await {
        if let Err(error) = send_result {
            error!(
                "Failed to send message into channel for sender: {:?}",
                error
            );
        }
    }
}

async fn handle_vision_notification(
    cycler: Cycler,
    database_reader: &Reader<vision::Database>,
    next_image_id: &mut Wrapping<u32>,
    subscribed_peers: &HashMap<Cycler, HashMap<SocketAddr, Peer>>,
) {
    let peers = match subscribed_peers.get(&cycler) {
        Some(peers) => peers,
        None => return,
    };
    let mut send_futures = vec![];
    {
        let database = database_reader.next();
        for peer in peers.values() {
            let mut outputs = vec![];
            let mut image_id = None;
            for output in peer.paths.iter() {
                match output {
                    Output::Main { path } => {
                        let data = match database.main_outputs.serialize_hierarchy(path) {
                            Ok(data) => data,
                            Err(error) => {
                                error!("Failed to serialize by path: {:?}", error);
                                continue;
                            }
                        };
                        outputs.push(SubscribedOutput {
                            output: output.clone(),
                            data,
                        });
                    }
                    Output::Additional { path } => {
                        let data = match database.additional_outputs.serialize_hierarchy(path) {
                            Ok(data) => data,
                            Err(error) => {
                                error!("Failed to serialize by path: {:?}", error);
                                continue;
                            }
                        };
                        outputs.push(SubscribedOutput {
                            output: output.clone(),
                            data,
                        });
                    }
                    Output::Image => {
                        if let Some(image422) = &database.image {
                            image_id = Some(next_image_id.0);
                            *next_image_id += Wrapping(1);

                            let mut rgb_image = image::RgbImage::new(
                                (image422.width() * 2) as u32,
                                image422.height() as u32,
                            );
                            for y in 0..image422.height() {
                                for x in 0..image422.width() {
                                    let pixels: [YCbCr444; 2] = image422[(x, y)].into();
                                    let left_rgb = Rgb::from(pixels[0]);
                                    rgb_image.put_pixel(
                                        (x * 2) as u32,
                                        y as u32,
                                        image::Rgb([left_rgb.r, left_rgb.g, left_rgb.b]),
                                    );
                                    let right_rgb = Rgb::from(pixels[1]);
                                    rgb_image.put_pixel(
                                        (x * 2 + 1) as u32,
                                        y as u32,
                                        image::Rgb([right_rgb.r, right_rgb.g, right_rgb.b]),
                                    );
                                }
                            }

                            let mut message_bytes = vec![0u8; 8];
                            let mut encoder = JpegEncoder::new_with_quality(&mut message_bytes, 40);
                            if let Err(error) = encoder.encode_image(&rgb_image) {
                                error!("Failed to encode image: {:?}", error);
                                image_id = None;
                                continue;
                            }
                            let length = (message_bytes.len() - 8) as u32;
                            LittleEndian::write_u32(&mut message_bytes[0..4], length);
                            LittleEndian::write_u32(&mut message_bytes[4..8], image_id.unwrap());

                            send_futures.push(peer.output_sender.send(Message::Binary {
                                payload: message_bytes,
                            }));
                        }
                    }
                }
            }

            send_futures.push(peer.output_sender.send(Message::Json {
                payload: Payload::OutputsUpdated {
                    cycler,
                    outputs,
                    image_id,
                },
            }));
        }
    }
    for send_result in join_all(send_futures).await {
        if let Err(error) = send_result {
            error!(
                "Failed to send message into channel for sender: {:?}",
                error
            );
        }
    }
}
