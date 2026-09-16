//! Live ROS-Z parameter clients. The UI never waits for a service on the render thread.
use std::{collections::BTreeMap, sync::Arc, time::Duration};

use color_eyre::{Result, eyre::eyre};
use ros_z::{
    node::Node,
    parameter::{NodeParameterWriteJson, RemoteParameterClient},
};
use serde_json::Value;
use tokio::{
    runtime::Handle,
    sync::{mpsc, watch},
    task::JoinHandle,
};

pub const GROUPS: [(&str, &str, &str, &str); 4] = [
    ("head_motion", "Head", "head_motion", ""),
    ("motion_inference", "Inference", "motion_inference", ""),
    ("hardware_interface", "Hardware", "hardware_interface", ""),
    (
        "joint_limits",
        "Joint limits",
        "simulator_joint_limits",
        "joint_limits",
    ),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub value: Value,
    pub revision: u64,
    pub layer: String,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub snapshots: BTreeMap<String, Snapshot>,
    pub errors: BTreeMap<String, String>,
    pub busy: bool,
    pub completion: u64,
    pub result: Option<Result<String, String>>,
}

struct Edit {
    group: &'static str,
    value: Value,
    revision: u64,
    layer: String,
}

pub struct ParameterClient {
    state: watch::Receiver<State>,
    edits: mpsc::Sender<Edit>,
    task: JoinHandle<()>,
}

impl ParameterClient {
    pub fn start(runtime: &Handle, node: Arc<Node>, namespace: &str) -> Self {
        let clients: Vec<_> = GROUPS
            .iter()
            .map(|&(key, _, target, path)| {
                (
                    key,
                    path,
                    RemoteParameterClient::new(
                        node.clone(),
                        format!("{}/{target}", namespace.trim_end_matches('/')),
                    )
                    .expect("absolute robotics namespace"),
                )
            })
            .collect();
        let (sender, state) = watch::channel(State::default());
        let (edits, mut requests) = mpsc::channel::<Edit>(1);
        let task = runtime.spawn(async move {
            let mut current = State::default();
            let mut refresh = tokio::time::interval(Duration::from_secs(1));
            refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    edit = requests.recv() => {
                        let Some(edit) = edit else { break; };
                        current.busy = true;
                        sender.send_replace(current.clone());
                        let (_, path, client) = clients.iter().find(|(key, _, _)| *key == edit.group).expect("known parameter group");
                        let result = async {
                            let writes = if path.is_empty() {
                                edit.value.as_object().ok_or_else(|| eyre!("Expected parameter object"))?.iter().map(|(field, value)| {
                                    Ok(NodeParameterWriteJson { path: field.clone(), value_json: serde_json::to_string(value)?, target_layer: edit.layer.clone() })
                                }).collect::<Result<Vec<_>>>()?
                            } else {
                                vec![NodeParameterWriteJson { path: path.to_string(), value_json: serde_json::to_string(&edit.value)?, target_layer: edit.layer }]
                            };
                            let response = tokio::time::timeout(Duration::from_secs(3), client.set_json_atomically(writes, Some(edit.revision))).await??;
                            if !response.success { return Err(eyre!("{}", response.message)); }
                            Ok(format!("{} parameters applied live", GROUPS.iter().find(|g| g.0 == edit.group).unwrap().1))
                        }.await;
                        current.result = Some(result.map_err(|error: color_eyre::Report| format!("{error:#}")));
                        current.completion += 1;
                        refresh_snapshots(&clients, &mut current).await;
                        current.busy = false;
                        sender.send_replace(current.clone());
                    }
                    _ = refresh.tick() => {
                        refresh_snapshots(&clients, &mut current).await;
                        sender.send_replace(current.clone());
                    }
                }
            }
        });
        Self { state, edits, task }
    }

    pub fn state(&self) -> State {
        self.state.borrow().clone()
    }

    pub fn apply(&self, group: &'static str, value: Value, snapshot: &Snapshot) -> Result<()> {
        validate(group, &value)?;
        self.edits
            .try_send(Edit {
                group,
                value,
                revision: snapshot.revision,
                layer: snapshot.layer.clone(),
            })
            .map_err(|_| eyre!("A parameter update is already pending"))?;
        Ok(())
    }
}

impl Drop for ParameterClient {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn refresh_snapshots(clients: &[(&str, &str, RemoteParameterClient)], state: &mut State) {
    // Poll independently: one unavailable node must not prevent editing the others.
    let mut tasks = tokio::task::JoinSet::new();
    for (key, path, client) in clients {
        let (key, path, client) = (key.to_string(), path.to_string(), client.clone());
        tasks.spawn(async move {
            let result = async {
                let response =
                    tokio::time::timeout(Duration::from_millis(500), client.get_snapshot())
                        .await??;
                if !response.success {
                    return Err(eyre!("{}", response.message));
                }
                let mut value: Value = serde_json::from_str(&response.value_json)?;
                if !path.is_empty() {
                    value = value[&path].take();
                }
                let layer = response
                    .layers
                    .last()
                    .ok_or_else(|| eyre!("Node has no writable parameter layer"))?
                    .clone();
                Ok(Snapshot {
                    value,
                    revision: response.revision,
                    layer,
                })
            }
            .await;
            (key, result)
        });
    }
    while let Some(Ok((key, result))) = tasks.join_next().await {
        match result {
            Ok(snapshot) => {
                state.errors.remove(&key);
                state.snapshots.insert(key, snapshot);
            }
            Err(error) => {
                state.errors.insert(key, format!("{error:#}"));
            }
        }
    }
}

fn validate(group: &str, value: &Value) -> Result<()> {
    match group {
        "head_motion" => {
            serde_json::from_value::<head_motion::parameters::Parameters>(value.clone())?
                .validate()
                .map_err(|e| eyre!(e))
        }
        "joint_limits" => {
            serde_json::from_value::<types::joint_limits::JointLimits>(value.clone())?
                .validate()
                .map_err(|e| eyre!(e))
        }
        "motion_inference" => {
            let p = serde_json::from_value::<motion_inference::config::Parameters>(value.clone())?;
            p.validate().map_err(|e| eyre!("{e:#}"))?;
            Ok(())
        }
        "hardware_interface" => {
            let p = serde_json::from_value::<hardware_interface::Parameters>(value.clone())?;
            if [
                p.joint_control_message_interval,
                p.rotate_head_message_interval,
                p.sdk_request_timeout,
            ]
            .iter()
            .any(Duration::is_zero)
            {
                return Err(eyre!("Hardware intervals must be greater than zero"));
            }
            Ok(())
        }
        _ => Err(eyre!("Unknown parameter group")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros_z::{
        prelude::*,
        time::{Clock, Time},
    };
    use serde_json::json;

    #[tokio::test(flavor = "multi_thread")]
    async fn live_writes_update_nodes_without_advancing_time_and_reject_stale_edits() {
        let layer = tempfile::tempdir().unwrap();
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let clock = Clock::logical(Time::from_nanos(4_000_000_000));
        let context = Arc::new(
            ContextBuilder::default()
                .with_namespace("/live_parameter_test")
                .with_mode("peer")
                .disable_multicast_scouting()
                .with_connect_endpoints(std::iter::empty::<&str>())
                .with_listen_endpoints(std::iter::empty::<&str>())
                .with_parameter_layers([
                    root.join("parameters"),
                    root.join("../../etc/parameters/base"),
                    layer.path().to_owned(),
                ])
                .with_clock(clock.clone())
                .build()
                .await
                .unwrap(),
        );
        let head = context.create_node("head_motion").build().await.unwrap();
        let head_parameters = head
            .bind_parameter_as::<head_motion::parameters::Parameters>("head_motion")
            .unwrap();
        head_parameters
            .add_validation_hook(head_motion::parameters::Parameters::validate)
            .unwrap();
        let inference = context
            .create_node("motion_inference")
            .build()
            .await
            .unwrap();
        let inference_parameters = inference
            .bind_parameter_as::<motion_inference::config::Parameters>("motion_inference")
            .unwrap();
        let ui = Arc::new(context.create_node("ui").build().await.unwrap());
        let client = ParameterClient::start(&Handle::current(), ui.clone(), "/live_parameter_test");
        let client_ref = &client;
        let wait = |completion| async move {
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    let state = client_ref.state();
                    if state.completion >= completion && state.snapshots.contains_key("head_motion")
                    {
                        break state;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap()
        };
        let initial = wait(0).await;
        let snapshot = &initial.snapshots["head_motion"];
        let mut value = snapshot.value.clone();
        value["joint_control"]["kp"]["yaw"] = json!(17.0);
        client
            .apply("head_motion", value.clone(), snapshot)
            .unwrap();
        assert_eq!(
            wait(1).await.result.unwrap(),
            Ok("Head parameters applied live".into())
        );
        assert_eq!(
            head_parameters.snapshot().typed().joint_control.kp.yaw,
            17.0
        );
        assert_eq!(clock.now(), Time::from_nanos(4_000_000_000));
        // Another editor must not silently overwrite an update using an old revision.
        value["joint_control"]["kp"]["yaw"] = json!(19.0);
        client.apply("head_motion", value, snapshot).unwrap();
        assert!(wait(2).await.result.unwrap().is_err());
        assert_eq!(
            head_parameters.snapshot().typed().joint_control.kp.yaw,
            17.0
        );
        // Inference parameters also update while the logical clock is paused.
        let remote =
            RemoteParameterClient::new(ui, "/live_parameter_test/motion_inference").unwrap();
        let response = remote
            .set_json(
                "locomotion.base_frequency",
                &json!(1.5),
                layer.path().to_string_lossy().into_owned(),
                None,
            )
            .await
            .unwrap();
        assert!(response.success);
        assert_eq!(
            inference_parameters
                .snapshot()
                .typed()
                .locomotion
                .base_frequency,
            1.5
        );
        drop(client);
        context.shutdown().unwrap();
    }
}
