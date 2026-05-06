use ros_z_protocol::qos::{
    QosDurability, QosDuration, QosHistory, QosLiveliness, QosProfile, QosReliability,
};

#[test]
fn qos_roundtrip_preserves_all_fields() {
    let qos = QosProfile {
        reliability: QosReliability::BestEffort,
        durability: QosDurability::TransientLocal,
        history: QosHistory::KeepLast(7),
        deadline: QosDuration { sec: 1, nsec: 2 },
        lifespan: QosDuration { sec: 3, nsec: 4 },
        liveliness: QosLiveliness::ManualByTopic,
        liveliness_lease_duration: QosDuration { sec: 5, nsec: 6 },
    };

    let encoded = qos.encode();
    assert_eq!(encoded, "2:1:1,7:1,2:3,4:3,5,6");
    assert_eq!(QosProfile::decode(&encoded).unwrap(), qos);
}

#[test]
fn qos_decode_rejects_malformed_duration() {
    assert!(QosProfile::decode("1:2:1,10:not-a-duration:,:1,2,3").is_err());
}

#[test]
fn qos_decode_rejects_extra_fields() {
    assert!(QosProfile::decode("1:2:1,10:,:,:,extra:field").is_err());
}
