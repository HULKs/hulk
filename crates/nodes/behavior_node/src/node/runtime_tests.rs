use super::*;
use ros_z::parameter::RemoteParameterClient;
use types::heatmap::{SEARCH_CELL_SIZE_TOPIC, grid_cell_size};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_search_and_behavior_nodes_start_and_share_live_cell_geometry() -> Result<()> {
    let layer = tempfile::tempdir()?;
    std::fs::write(
        layer.path().join("behavior_node.json5"),
        include_str!("../../../../../etc/parameters/base/behavior_node.json5"),
    )?;
    std::fs::write(
        layer.path().join("search_suggestor.json5"),
        include_str!("../../../../../etc/parameters/base/search_suggestor.json5"),
    )?;
    let context = Arc::new(
        ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints(["tcp/127.0.0.1:0"])
            .with_parameter_layers([layer.path().to_path_buf()])
            .build()
            .await?,
    );
    let fixture = Arc::new(context.create_node("search_startup_test").build().await?);
    let retained = QosProfile {
        durability: QosDurability::TransientLocal,
        history: QosHistory::KeepLast(NonZeroUsize::MIN),
        ..Default::default()
    };
    let field_pub = fixture
        .publisher::<FieldDimensions>("field_dimensions")
        .qos(retained.clone())
        .build()
        .await?;
    let player_pub = fixture
        .publisher::<PlayerNumber>("player_number")
        .qos(retained.clone())
        .build()
        .await?;
    let sizes = fixture
        .subscriber::<Vector2<Field>>(SEARCH_CELL_SIZE_TOPIC)
        .qos(retained)
        .build()
        .await?;
    let blackboards = fixture
        .subscriber::<Blackboard>("behavior/blackboard")
        .build()
        .await?;
    let field = FieldDimensions {
        length: 14.0,
        width: 9.0,
        ..FieldDimensions::SPL_2025
    };
    field_pub.publish(&field).await?;
    player_pub.publish(&PlayerNumber::One).await?;
    let mut nodes = tokio::task::JoinSet::new();
    nodes.spawn(search_suggestor::run_boxed(context.clone()));
    let size = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::select! {
            size = sizes.recv() => size.unwrap(),
            failed = nodes.join_next() => panic!("search node exited during startup: {failed:?}"),
        }
    })
    .await?;
    assert_eq!(size, grid_cell_size(field, 28));
    // Behavior starts after publication and must receive the retained geometry.
    nodes.spawn(run(context));
    let client = RemoteParameterClient::new(fixture, "/search_suggestor")?;
    for cells in [28, 30] {
        if cells == 30 {
            let response = client
                .set_json(
                    "cells_along_field_length",
                    &30.into(),
                    layer.path().to_string_lossy().into_owned(),
                    None,
                )
                .await?;
            assert!(response.success);
        }
        let expected = grid_cell_size(field, cells);
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                tokio::select! {
                    blackboard = blackboards.recv() => {
                        if blackboard.unwrap().search_cell_size == Some(expected) { break; }
                    }
                    failed = nodes.join_next() => panic!("robot node exited: {failed:?}"),
                }
            }
        })
        .await?;
    }
    nodes.abort_all();
    while nodes.join_next().await.is_some() {}
    Ok(())
}
