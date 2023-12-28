use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use futures_util::{sink::SinkExt, stream::SplitSink, StreamExt};

use axum::{
    extract::{Extension, Path, ws::WebSocket},
    http::StatusCode,
    routing::get,
    Router,
    response::Response,
};

// Define a struct containing user data like ID and a sender for broadcasting messages.
/*
struct Client {
    user_id: usize,
    sender: Sender,
    // channel: ,
    // topic: 
}
*/

// Dtata structure for messages
#[derive(Deserialize, Serialize)]
struct Message {
    from: String,
    to: String,
    message: String
}

type Clients = Arc<RwLock<HashMap<String, Arc<RwLock<SplitSink<WebSocket, axum::extract::ws::Message>>>>>>; // Data structure to store connected Clients

async fn health_handler() -> StatusCode {
    StatusCode::OK
}

/* Implement Register Handler:
async fn register_handler(
    // Extract client details from the registration request
    extracted_client_id: String,
    clients: Arc<Mutex<HashMap<String, Client>>>,
) -> StatusCode {

    // Generate a unique user ID.
    let user_id = thread_rng().gen::<usize>();

    // Create a new Client instance with the generated ID and a sender channel.
    let new_client = Client {
        user_id,
        sender: broadcast::channel(100).0,
    };

    let mut clients = clients.lock().await;
    // Add the client to the shared clients HashMap.
    clients.insert(extracted_client_id, new_client);

    // Return a successful response
    StatusCode::CREATED
}
*/

// Implement WebSocket Handler
async fn ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    Extension(clients): Extension<Clients>,
    Path(client_id): Path<String>,
) -> Response {
    // Accept websocket connection upgrades
    ws.on_upgrade(move |socket|{upgrade(socket, clients, client_id)})
}

async fn upgrade(socket: WebSocket, clients: Clients, client_id: String){
        let (sender, mut receiver) = socket.split();

        let client_sender = Arc::new(RwLock::new(sender));
        clients.write().await.insert(client_id, client_sender);

        // Start a loop to receive messages
        while let Some(msg) = receiver.next().await {
            if let Ok(msg) = msg{
                match msg {
                    axum::extract::ws::Message::Text(text) => {
                        let message: Result<Message, _> = serde_json::from_str(&text);
                        match message {
                            Ok(message) => {
                                let to = message.to;
                                let clients = clients.read().await;
                                let client = clients.get(&to);
                                match client {
                                    Some(client) => {
                                        client.write().await.send(axum::extract::ws::Message::Text(text)).await;
                                    },
                                    None => {},
                                }
                            },
                            Err(err) => {
                                eprintln!("WebSocket error: {}", err);
                            }
                        }
                    },
                    axum::extract::ws::Message::Close(_) => todo!(),
                    _ => {}
                }
            }
        }
    }


#[tokio::main]
async fn main() {
    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
    let app = Router::new()
        .route("/health", get(health_handler)) // To test the healt of the server
        // .route("/register", post(register_handler)) // To register a client
        .route("/ws/:client_id", get(ws_handler))  // To handle web socket connections, including opening, receiving messages, broadcasting messages to other clients, and closing connections
        .layer(Extension(clients));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
