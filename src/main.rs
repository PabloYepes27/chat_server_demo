use std::collections::HashMap;
use std::sync::Arc;
use rand::{thread_rng, Rng};
use tokio::sync::{Mutex, broadcast};

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    routing::{get, post},
    Router,
    Server,
};

type Sender = broadcast::Sender<String>;

// Define a struct containing user data like ID and a sender for broadcasting messages.
struct Client {
    user_id: usize,
    sender: Sender,
    // channel: ,
    // topic: 
}

type Clients = Arc<Mutex<HashMap<String, Client>>>; // Data structure to store connected Clients

async fn health_handler() -> StatusCode {
    StatusCode::OK
}

// Implement Register Handler:
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

// Implement WebSocket Handler
async fn ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    Extension(clients): Extension<Arc<Mutex<HashMap<String, Client>>>>,
    Path(client_id): Path<String>,
) -> impl axum::response::IntoResponse {
    let client = Client {
        user_id: 1 /* assign a valid user_id here */,
        sender: broadcast::channel(100).0,
    };

    let mut clients = clients.lock().await;
    clients.insert(client_id.clone(), client);

    // Accept websocket connection upgrades
    ws.on_upgrade(|mut socket| async move {
        while let Some(msg) = socket.recv().await {
            match msg {
                Ok(msg) => {
                    let data = msg.into_data();
                    let message = String::from_utf8_lossy(&data);


                    // Broadcast message to other clients
                    let clients = clients.lock().await;

                    // Start a loop to receive messages
                    for (_, client) in clients.iter_mut() {
                        if client.user_id != client_id {  // Exclude sender
                            client.sender.send(message.clone()).unwrap();
                        }
                    }
                }
                // Handle websocket errors gracefully
                Err(err) => {
                    eprintln!("WebSocket error: {}", err);
                    break;
                }
            }
        }
    });

    // Close the connection on disconnect
    // ws.on_closed()
}

#[tokio::main]
async fn main() {
    let clients = Arc::new(Mutex::new(HashMap::<String, Client>::new()));
    let app = Router::new()
        .route("/health", get(health_handler)) // To test the healt of the server
        .route("/register", post(register_handler)) // To register a client
        .route("/ws/:client_id", get(ws_handler))  // To handle web socket connections, including opening, receiving messages, broadcasting messages to other clients, and closing connections
        .layer(Extension(clients));

    Server::bind(&"127.0.0.1:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
