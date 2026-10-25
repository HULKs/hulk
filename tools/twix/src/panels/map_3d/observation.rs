use std::{num::NonZeroUsize, sync::Arc, time::Duration};

use color_eyre::Result;
use ros_z::{Message, time::Time};
use ros_z_debug::{
    ObservationPolicy, RetentionPolicy, SampleRecord, TargetIdentity, TopicObservation,
    TopicReference,
};
use types::time_wrapper::TimeWrapper;

use crate::repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates};

pub(super) struct Observation<T> {
    observation: TopicObservation<T>,
    topic: TopicReference,
    _repaint: ObservationRepaint,
}

impl<T> Observation<T>
where
    T: Message + Send + Sync + 'static,
    T::Codec: Send + Sync,
{
    pub(super) fn new(
        context: &impl ObservationContext,
        topic: &'static str,
        capacity: usize,
        policy: ObservationPolicy,
    ) -> Result<Self> {
        let _runtime = context.backend().runtime_handle().enter();
        let observation = context
            .backend()
            .observer()
            .observe_typed::<T>(topic)?
            .policy(policy)
            .retention(RetentionPolicy::time_window_with_max_samples(
                Duration::from_secs(2),
                NonZeroUsize::new(capacity).expect("nonzero history capacity"),
            )?)
            .spawn();
        let repaint = observation.repaint_on_updates(context);
        Ok(Self {
            observation,
            topic: TopicReference::new(topic)?,
            _repaint: repaint,
        })
    }
}

impl<T> Observation<T> {
    pub(super) fn latest(&self, namespace: &str) -> Option<Arc<SampleRecord<T>>> {
        let topic = self
            .topic
            .resolve(&TargetIdentity::new(namespace).ok()?)
            .ok()?;
        self.observation
            .latest()
            .filter(|record| record.metadata.resolved_topic == topic)
    }

    pub(super) fn all(&self, namespace: &str) -> Vec<Arc<SampleRecord<T>>> {
        let Ok(topic) =
            TargetIdentity::new(namespace).and_then(|target| self.topic.resolve(&target))
        else {
            return Vec::new();
        };
        self.observation
            .get_all()
            .into_iter()
            .filter(|record| record.metadata.resolved_topic == topic)
            .collect()
    }
}

impl<T> Observation<TimeWrapper<T>> {
    pub(super) fn aligned(
        &self,
        namespace: &str,
        time: Option<Time>,
    ) -> Option<Arc<SampleRecord<TimeWrapper<T>>>> {
        let records = self.all(namespace);
        match time {
            Some(time) => nearest(records, time, Duration::from_millis(100), |record| {
                record.value.time
            }),
            None => records.into_iter().max_by_key(|record| record.value.time),
        }
    }
}

// Observer ordering is transport/source time, not the timestamp inside the payload.
pub(super) fn nearest<T>(
    records: impl IntoIterator<Item = T>,
    time: Time,
    tolerance: Duration,
    stamp: impl Fn(&T) -> Time,
) -> Option<T> {
    let record = records
        .into_iter()
        .min_by_key(|record| stamp(record).abs_diff(time))?;
    (stamp(&record).abs_diff(time) <= tolerance).then_some(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{backend::RobotBackend, panel::PanelCreationContext};
    use ros_z::context::ContextBuilder;
    use ros_z_debug::{TopicObserver, TopicObserverOptions};
    use ros2::std_msgs::header::Header;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn latest_and_all_resolve_normalized_namespaces_and_absolute_topics() {
        let context = PanelCreationContext {
            backend: Arc::new(
                RobotBackend::new(tokio::runtime::Handle::current(), None, "/42".to_string())
                    .await
                    .unwrap(),
            ),
            value: None,
            egui_context: eframe::egui::Context::default(),
            render_state: None,
        };
        let ros = ContextBuilder::default().build().await.unwrap();
        let node = Arc::new(
            ros.create_node("map_3d_namespace_test")
                .build()
                .await
                .unwrap(),
        );
        let observer = TopicObserver::new(
            Arc::clone(&node),
            TopicObserverOptions::with_namespace("/42").unwrap(),
        );
        let observe = |topic: &str| {
            let observation = observer
                .observe_typed::<Header>(topic)
                .unwrap()
                .retention(RetentionPolicy::time_window(Duration::from_secs(5)).unwrap())
                .spawn();
            Observation {
                topic: TopicReference::new(topic).unwrap(),
                _repaint: observation.repaint_on_updates(&context),
                observation,
            }
        };
        let relative = observe("map_3d_namespace_sample");
        let absolute = observe("/42/map_3d_namespace_sample");
        let publisher = node
            .publisher::<Header>("/42/map_3d_namespace_sample")
            .build()
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while relative.observation.latest().is_none() || absolute.observation.latest().is_none()
            {
                publisher.publish(&Header::default()).await.unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("both observations should receive the sample");

        for namespace in ["42", "/42", "42/", "/42/", "/42///", "/43"] {
            for (observation, expected) in [(&relative, namespace != "/43"), (&absolute, true)] {
                assert_eq!(
                    observation.latest(namespace).is_some(),
                    expected,
                    "{namespace}"
                );
                assert_eq!(
                    !observation.all(namespace).is_empty(),
                    expected,
                    "{namespace}"
                );
            }
        }
    }

    #[test]
    fn alignment_uses_payload_time_and_bounds_distance() {
        let records = [(900, 10), (10, 900), (800, 20)];
        let stamp = |record: &(i64, i64)| Time::from_nanos(record.1);
        assert_eq!(
            nearest(
                records,
                Time::from_nanos(12),
                Duration::from_nanos(3),
                stamp
            ),
            Some((900, 10))
        );
        assert_eq!(
            nearest(
                records,
                Time::from_nanos(12),
                Duration::from_nanos(1),
                stamp
            ),
            None
        );
        assert_eq!(
            nearest(records, Time::from_nanos(20), Duration::ZERO, stamp),
            Some((800, 20))
        );
    }
}
