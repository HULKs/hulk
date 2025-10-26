use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router,
};
use bytes::Bytes;
use tokio::sync::{
    broadcast::{Receiver, Sender},
    SetOnce,
};

pub fn setup() -> (Router, Arc<SceneState>) {
    let scene_sender = Sender::new(8);
    let state = Arc::new(SceneState {
        scene: SetOnce::new(),
        scene_sender,
    });

    let router = Router::new()
        .route("/", get(scene))
        .route("/subscribe", get(subscribe_scene))
        .layer(Extension(state.clone()));

    (router, state)
}

pub struct SceneState {
    pub scene: SetOnce<Bytes>,
    pub scene_sender: Sender<String>,
}

async fn scene(Extension(state): Extension<Arc<SceneState>>) -> Response {
    log::info!("Got scene request");
    state.scene.wait().await.clone().into_response()
}

async fn subscribe_scene(
    Extension(state): Extension<Arc<SceneState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    log::info!("New scene websocket connection");
    let receiver = state.scene_sender.subscribe();
    ws.on_upgrade(move |socket| handle_socket(socket, receiver))
}

async fn handle_socket(mut socket: WebSocket, mut scene_receiver: Receiver<String>) {
    loop {
        match scene_receiver.recv().await {
            Ok(scene) => {
                if let Err(error) = socket.send(Message::Text(scene.into())).await {
                    log::error!("Failed to send scene, closing connection: {error}");
                    return;
                }
            }
            Err(error) => {
                log::error!("Failed to receive scene, closing connection: {error}");
                return;
            }
        }
    }
}
