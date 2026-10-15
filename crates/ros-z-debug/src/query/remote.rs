use std::{sync::Arc, time::Duration};

use bincode::Options;
use ros_z::{node::Node, time::Time};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use super::*;

pub const MAX_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;

pub fn query_key(namespace: &str) -> String {
    let namespace = namespace.trim_matches('/');
    if namespace.is_empty() {
        "hulk/mcap/query".into()
    } else {
        format!("hulk/mcap/{namespace}/query")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordingInfo {
    pub instance: String,
    pub name: String,
    pub start: Time,
    pub end: Time,
    pub topics: Vec<TopicInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Request {
    Catalog,
    Plan {
        instance: String,
        cursor: Time,
        plan: plan::QueryPlan,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    RecordingChanged,
    Catalog(RecordingInfo),
    Error(String),
    PlanData(plan::Evaluation),
}

pub fn encode(response: &Response) -> Result<Vec<u8>> {
    bincode::DefaultOptions::new()
        .with_limit(MAX_RESPONSE_BYTES)
        .serialize(response)
        .map_err(query_error)
}

pub fn decode(bytes: &[u8]) -> Result<Response> {
    bincode::DefaultOptions::new()
        .with_limit(MAX_RESPONSE_BYTES)
        .deserialize(bytes)
        .map_err(query_error)
}

async fn request_catalog(node: &Node, key: &str) -> Result<RecordingInfo> {
    let payload = serde_json::to_vec(&Request::Catalog).map_err(query_error)?;
    let replies = node
        .session()
        .get(key)
        .payload(payload)
        .consolidation(zenoh::query::ConsolidationMode::None)
        .timeout(Duration::from_secs(120))
        .await
        .map_err(query_error)?;
    let reply = replies.recv_async().await.map_err(query_error)?;
    let sample = reply
        .into_result()
        .map_err(|error| query_error(String::from_utf8_lossy(&error.payload().to_bytes())))?;
    match decode(&sample.payload().to_bytes())? {
        Response::Catalog(info) => Ok(info),
        Response::RecordingChanged => Err(Error::RecordingChanged),
        Response::Error(error) => Err(query_error(error)),
        _ => Err(query_error("expected recording catalog")),
    }
}

pub struct RemoteMcap {
    replaced: watch::Sender<bool>,
    catalog_check: tokio::task::JoinHandle<()>,
    bounds: watch::Sender<(Time, Time)>,
    namespace: String,
    node: Arc<Node>,
    key: String,
    info: Arc<RecordingInfo>,
    topics: TopicList,
}

impl RemoteMcap {
    pub async fn connect(node: Arc<Node>, namespace: &str) -> Result<Self> {
        let key = query_key(namespace);
        let info = request_catalog(&node, &key).await?;
        if info.start > info.end {
            return Err(query_error("invalid recording bounds"));
        }
        for topic in &info.topics {
            crate::TopicReference::new(&topic.name)?;
            let schema = topic
                .schema
                .as_ref()
                .ok_or_else(|| query_error("recording topic has no schema"))?;
            schema.validate().map_err(query_error)?;
            if ros_z_schema::compute_hash(schema.as_ref()).map_err(query_error)?
                != topic.type_info.hash
            {
                return Err(query_error("recording schema hash mismatch"));
            }
        }
        let (_, receiver) = watch::channel(Arc::new(TopicSnapshot {
            topics: info.topics.clone(),
            loading: false,
            error: None,
        }));
        let replaced = watch::channel(false).0;
        let catalog_check = {
            let (node, key, instance, replaced) = (
                Arc::clone(&node),
                key.clone(),
                info.instance.clone(),
                replaced.clone(),
            );
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    if let Ok(Ok(info)) =
                        tokio::time::timeout(Duration::from_secs(3), request_catalog(&node, &key))
                            .await
                        && info.instance != instance
                    {
                        replaced.send_replace(true);
                        break;
                    }
                }
            })
        };
        Ok(Self {
            replaced,
            catalog_check,
            bounds: watch::channel((info.start, info.end)).0,
            namespace: namespace.into(),
            node,
            key,
            info: Arc::new(info),
            topics: TopicList(receiver),
        })
    }

    pub fn recording(&self) -> &Arc<RecordingInfo> {
        &self.info
    }

    pub fn bounds(&self) -> (Time, Time) {
        *self.bounds.borrow()
    }

    pub fn needs_reconnect(&self) -> bool {
        *self.replaced.borrow()
    }
}

impl Drop for RemoteMcap {
    fn drop(&mut self) {
        self.catalog_check.abort();
    }
}

impl Backend for RemoteMcap {
    fn topics(&self) -> TopicList {
        self.topics.clone()
    }

    fn observe(&self, cursor: CursorHandle, mut plan: plan::QueryPlan) -> Result<PlanObservation> {
        plan.resolve_topics(&self.namespace)?;
        let node = Arc::clone(&self.node);
        let key = self.key.clone();
        let instance = self.info.instance.clone();
        let catalog = self.info.clone();
        let bounds = self.bounds.clone();
        let replaced = self.replaced.clone();
        let mut replacement = replaced.subscribe();
        let mut positions = cursor.subscribe();
        let (observation, sender) = PlanObservation::channel(cursor.clone());
        tokio::spawn(async move {
            let mut command = Request::Plan {
                instance,
                cursor: Time::zero(),
                plan,
            };
            loop {
                let position = *positions.borrow_and_update();
                let fail = |error: String| {
                    Arc::new(PlanSnapshot {
                        revision: position.revision,
                        evaluation: Arc::new(plan::Evaluation {
                            valid_for: CursorRange {
                                start: Time::zero(),
                                end: None,
                            },
                            ..plan::Evaluation::unresolved(
                                match position.cursor {
                                    Cursor::At(t) => t,
                                    Cursor::Live => Time::zero(),
                                },
                                plan::State::Failed(error),
                            )
                        }),
                    })
                };
                if *replacement.borrow() {
                    sender.send_replace(fail("recording changed; reconnect".into()));
                    break;
                }
                let Cursor::At(time) = position.cursor else {
                    sender.send_replace(fail("recordings require Cursor::At".into()));
                    tokio::select! { _ = sender.closed() => break, _ = positions.changed() => {}, _ = replacement.changed() => {} }
                    continue;
                };
                let previous = sender.borrow().clone();
                if matches!(previous.evaluation.state, plan::State::Ready(_))
                    && previous.evaluation.settled()
                    && previous.evaluation.valid_for.contains(time)
                {
                    if previous.revision != position.revision {
                        sender.send_replace(Arc::new(PlanSnapshot {
                            revision: position.revision,
                            evaluation: previous.evaluation.clone(),
                        }));
                    }
                    tokio::select! { _=sender.closed()=>break, _=replacement.changed()=>{}, changed=positions.changed()=>if changed.is_err(){break;} }
                    continue;
                }
                sender.send_replace(Arc::new(PlanSnapshot {
                    revision: position.revision,
                    evaluation: Arc::new(plan::Evaluation::unresolved(time, plan::State::Pending)),
                }));
                if let Request::Plan { cursor, .. } = &mut command {
                    *cursor = time;
                }
                let execute = async {
                    let payload = serde_json::to_vec(&command).map_err(query_error)?;
                    let replies = node
                        .session()
                        .get(&key)
                        .payload(payload)
                        .consolidation(zenoh::query::ConsolidationMode::None)
                        .timeout(Duration::from_secs(120))
                        .await
                        .map_err(query_error)?;
                    let mut settled = false;
                    while let Ok(reply) = replies.recv_async().await {
                        let sample = reply.into_result().map_err(|error| {
                            query_error(String::from_utf8_lossy(&error.payload().to_bytes()))
                        })?;
                        match decode(&sample.payload().to_bytes())? {
                            Response::PlanData(mut result) => {
                                if result.cursor != time {
                                    return Err(query_error("plan response cursor mismatch"));
                                }
                                if let Request::Plan { plan, .. } = &command {
                                    plan.validate_result(&mut result, &catalog.topics)?;
                                }
                                settled = result.settled();
                                if let Some((start, end)) = result.bounds {
                                    bounds.send_modify(|bounds| {
                                        bounds.0 = bounds.0.min(start);
                                        bounds.1 = bounds.1.max(end);
                                    });
                                }
                                if cursor.position().revision == position.revision {
                                    sender.send_replace(Arc::new(PlanSnapshot {
                                        revision: position.revision,
                                        evaluation: Arc::new(result),
                                    }));
                                }
                            }
                            Response::RecordingChanged => {
                                replaced.send_replace(true);
                                return Err(Error::RecordingChanged);
                            }
                            Response::Error(error) => return Err(query_error(error)),
                            _ => return Err(query_error("expected plan response")),
                        }
                    }
                    if settled {
                        Ok::<_, Error>(())
                    } else {
                        Err(query_error("plan response ended before completion"))
                    }
                };
                let result = tokio::select! {
                    _ = sender.closed() => break,
                    _ = replacement.changed() => continue,
                    _ = async {
                        while positions.changed().await.is_ok() {
                            if positions.borrow().revision != position.revision { break; }
                        }
                    } => continue,
                    result = execute => result,
                };
                if let Err(error) = &result
                    && cursor.position().revision == position.revision
                {
                    sender.send_replace(fail(error.to_string()));
                }
                if cursor.position().cursor != position.cursor {
                    continue;
                }
                tokio::select! {
                    _ = sender.closed() => break,
                    _ = replacement.changed() => {},
                    changed = positions.changed() => if changed.is_err() { break; },
                    _ = tokio::time::sleep(Duration::from_secs(1)), if result.is_err() => {},
                }
            }
        });
        Ok(observation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_key_resolves_namespace() {
        assert_eq!(query_key("/"), "hulk/mcap/query");
        assert_eq!(query_key("/replay/"), "hulk/mcap/replay/query");
    }
}
