use std::{collections::BTreeMap, fs::File, io::BufWriter};

use mcap::{WriteOptions, records::MessageHeader, write::Metadata};
use tempfile::tempdir;

use crate::inspect_file;

#[test]
fn inspects_ros_z_metadata_and_topic_counts() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("recorded.mcap");
    let file = File::create(&path).expect("output file");
    let mut writer = WriteOptions::new()
        .profile("ros-z")
        .library("ros-z-record/test")
        .create(BufWriter::new(file))
        .expect("writer");

    let schema_id = writer
        .add_schema(
            "std_msgs::String",
            "ros-z/schema+json;v=2",
            br#"{"definitions":{"std_msgs::String":{"kind":"struct","fields":[{"name":"data","shape":{"kind":"string"}}]}},"root":"std_msgs::String"}"#,
        )
        .expect("schema");
    let channel_id = writer
        .add_channel(
            schema_id,
            "/demo",
            "cdr",
            &BTreeMap::from([
                ("requested_topic".to_string(), "/demo".to_string()),
                ("qualified_topic".to_string(), "/demo".to_string()),
                ("type_name".to_string(), "std_msgs::String".to_string()),
                (
                    "schema_hash".to_string(),
                    "RZHS01_0000000000000000000000000000000000000000000000000000000000000000"
                        .to_string(),
                ),
                (
                    "source_id".to_string(),
                    "gid:01010101010101010101010101010101".to_string(),
                ),
            ]),
        )
        .expect("channel");

    writer
        .write_metadata(&Metadata {
            name: "ros-z.session".to_string(),
            metadata: BTreeMap::new(),
        })
        .expect("session metadata");
    writer
        .write_metadata(&Metadata {
            name: "ros-z.request".to_string(),
            metadata: BTreeMap::from([("topics".to_string(), r#"["/demo"]"#.to_string())]),
        })
        .expect("request metadata");
    writer
        .write_metadata(&Metadata {
            name: "ros-z.resolved_topics".to_string(),
            metadata: BTreeMap::from([(
                "topics".to_string(),
                r#"[{"requested_topic":"/demo","qualified_topic":"/demo","type":"std_msgs::String","schema_hash":"RZHS01_0000000000000000000000000000000000000000000000000000000000000000","schema_json":"{\"definitions\":{\"std_msgs::String\":{\"kind\":\"struct\",\"fields\":[{\"name\":\"data\",\"shape\":{\"kind\":\"string\"}}]}},\"root\":\"std_msgs::String\"}","publishers":[]}]"#
                    .to_string(),
            )]),
        })
        .expect("resolved topics metadata");

    writer
        .write_to_known_channel(
            &MessageHeader {
                channel_id,
                sequence: 1,
                log_time: 10,
                publish_time: 10,
            },
            &[1, 2, 3],
        )
        .expect("first message");
    writer
        .write_to_known_channel(
            &MessageHeader {
                channel_id,
                sequence: 2,
                log_time: 20,
                publish_time: 20,
            },
            &[4, 5],
        )
        .expect("second message");
    writer.finish().expect("finish writer");

    let report = inspect_file(&path).expect("inspect file");
    assert_eq!(report.message_count, 2);
    assert_eq!(report.message_start_time, Some(10));
    assert_eq!(report.message_end_time, Some(20));
    assert_eq!(report.topics.len(), 1);
    assert_eq!(report.topics[0].topic, "/demo");
    assert_eq!(report.topics[0].message_count, 2);
    assert_eq!(report.topics[0].byte_count, 5);
    assert_eq!(
        report.topics[0].schema_encoding.as_deref(),
        Some("ros-z/schema+json;v=2")
    );
    assert_eq!(
        report.topics[0].schema_hash.as_deref(),
        Some("RZHS01_0000000000000000000000000000000000000000000000000000000000000000")
    );
    assert_eq!(
        report.topics[0].source_ids,
        vec!["gid:01010101010101010101010101010101"]
    );
    assert_eq!(
        report.ros_z.requested_topics,
        Some(vec!["/demo".to_string()])
    );
}

#[test]
fn inspects_generic_mcap_without_ros_z_metadata() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("generic.mcap");
    let file = File::create(&path).expect("output file");
    let mut writer = WriteOptions::new()
        .create(BufWriter::new(file))
        .expect("writer");

    let channel_id = writer
        .add_channel(0, "/generic", "application/octet-stream", &BTreeMap::new())
        .expect("channel");
    writer
        .write_to_known_channel(
            &MessageHeader {
                channel_id,
                sequence: 7,
                log_time: 50,
                publish_time: 50,
            },
            &[9, 9, 9],
        )
        .expect("message");
    writer.finish().expect("finish writer");

    let report = inspect_file(&path).expect("inspect file");
    assert_eq!(report.message_count, 1);
    assert!(report.ros_z.session.is_none());
    assert!(report.ros_z.requested_topics.is_none());
    assert_eq!(report.topics.len(), 1);
    assert_eq!(report.topics[0].topic, "/generic");
}

#[test]
fn keeps_generic_summary_when_ros_z_metadata_is_malformed() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("broken.mcap");
    let file = File::create(&path).expect("output file");
    let mut writer = WriteOptions::new()
        .create(BufWriter::new(file))
        .expect("writer");

    let channel_id = writer
        .add_channel(0, "/broken", "cdr", &BTreeMap::new())
        .expect("channel");
    writer
        .write_metadata(&Metadata {
            name: "ros-z.request".to_string(),
            metadata: BTreeMap::from([("topics".to_string(), "{not-json}".to_string())]),
        })
        .expect("metadata");
    writer
        .write_to_known_channel(
            &MessageHeader {
                channel_id,
                sequence: 1,
                log_time: 1,
                publish_time: 1,
            },
            &[1],
        )
        .expect("message");
    writer.finish().expect("finish writer");

    let report = inspect_file(&path).expect("inspect file");
    assert_eq!(report.message_count, 1);
    assert_eq!(report.topics.len(), 1);
    assert!(!report.warnings.is_empty());
}

#[test]
fn counts_multiple_channels_for_the_same_topic_even_if_one_is_silent() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("multi-channel.mcap");
    let file = File::create(&path).expect("output file");
    let mut writer = WriteOptions::new()
        .create(BufWriter::new(file))
        .expect("writer");

    let _silent_channel = writer
        .add_channel(
            0,
            "/shared",
            "cdr",
            &BTreeMap::from([(
                "source_id".to_string(),
                "gid:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            )]),
        )
        .expect("first channel");
    let second_channel = writer
        .add_channel(
            0,
            "/shared",
            "cdr",
            &BTreeMap::from([(
                "source_id".to_string(),
                "gid:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
            )]),
        )
        .expect("second channel");

    writer
        .write_to_known_channel(
            &MessageHeader {
                channel_id: second_channel,
                sequence: 1,
                log_time: 5,
                publish_time: 5,
            },
            &[1, 2],
        )
        .expect("message");
    writer.finish().expect("finish writer");

    let report = inspect_file(&path).expect("inspect file");
    assert_eq!(report.topics.len(), 1);
    assert_eq!(report.topics[0].topic, "/shared");
    assert_eq!(report.topics[0].channel_count, 2);
    assert_eq!(
        report.topics[0].source_ids,
        vec![
            "gid:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "gid:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ]
    );
}

#[test]
fn counts_duplicate_metadata_records_without_collapsing_them() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("duplicate-metadata.mcap");
    let file = File::create(&path).expect("output file");
    let mut writer = WriteOptions::new()
        .create(BufWriter::new(file))
        .expect("writer");

    let channel_id = writer
        .add_channel(0, "/meta", "cdr", &BTreeMap::new())
        .expect("channel");
    writer
        .write_metadata(&Metadata {
            name: "duplicate".to_string(),
            metadata: BTreeMap::from([("value".to_string(), "first".to_string())]),
        })
        .expect("first metadata");
    writer
        .write_metadata(&Metadata {
            name: "duplicate".to_string(),
            metadata: BTreeMap::from([("value".to_string(), "second".to_string())]),
        })
        .expect("second metadata");
    writer
        .write_to_known_channel(
            &MessageHeader {
                channel_id,
                sequence: 1,
                log_time: 1,
                publish_time: 1,
            },
            &[7],
        )
        .expect("message");
    writer.finish().expect("finish writer");

    let report = inspect_file(&path).expect("inspect file");
    assert_eq!(report.metadata_count, 2);
}
