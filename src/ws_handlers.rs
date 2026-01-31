// src/ws_handlers.rs

use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use warp::{
    http::StatusCode,
    ws::{Message, WebSocket},
    Rejection, Reply,
};
use bcrypt;
use warp::reject::Reject; // Import the Reject trait

/// Global application state, shared across all handlers.
#[derive(Debug)]
pub struct AppState {
    // Stores registered users: username -> User struct
    pub users: Mutex<HashMap<String, User>>,
    // Stores active user sessions: session_key (UUID string) -> UserSession struct
    // The key here is the unique session_key itself.
    pub user_sessions: Mutex<HashMap<String, UserSession>>,
    // Stores active WebSocket connections: session_key (String) -> mpsc sender channel
    // Now keyed by the unique session_key, allowing multiple connections per user.
    pub active_connections: Mutex<HashMap<String, mpsc::UnboundedSender<Message>>>,
}
#[derive(Serialize)]
pub struct UserDTO {
    pub id: Uuid,
    pub username: String,
}
/// Represents a registered user in the system.
#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    // Stores contacts: contact_user_id (UUID) -> contact_username (String)
    pub contacts: Arc<Mutex<HashMap<Uuid, String>>>,
}

/// Represents an active user session, holding basic user information
/// that's validated with a session key.
#[derive(Clone, Debug, Serialize)]
pub struct UserSession {
    pub user_id: Uuid,
    pub username: String,
    pub session_key: String, // Added session_key to UserSession
}

/// Custom error response struct for consistent API error messages.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub message: String,
}

// REQUIRED: Implement the `warp::reject::Reject` trait for your custom error.
// This allows `warp::reject::custom` to accept `ErrorResponse`.
impl Reject for ErrorResponse {}


// --- WebSocket Message Structures ---

/// Messages sent FROM the client TO the server.
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ClientMessage {
    ChatMessage {
        to_user_id: Uuid,
        message: String,
    },
    TypingIndicator {
        to_user_id: Uuid,
        is_typing: bool,
    },
    // The client sends this when it has displayed a message.
    ReadReceipt {
        // This is the *original sender* of the message that was just read.
        to_user_id: Uuid,
        message_id: String,
    },
}

/// Messages sent FROM the server TO the clients.
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ServerMessage {
    ChatMessage {
        from_user_id: Uuid,
        from_username: String,
        to_user_id: Uuid,
        message_id: String,
        timestamp: String,
        // Emojis are supported natively by Rust's UTF-8 String type.
        message: String,
    },
    StatusMessage {
        user_id: Uuid,
        username: String,
        status: String, // "online" or "offline"
    },
    // The server forwards this receipt to the original message sender.
    ReadReceipt {
        from_user_id: Uuid, // The user who just read the message.
        message_id: String,
    },
    TypingIndicator {
        from_user_id: Uuid,
        is_typing: bool,
    },
}

/// Main handler for an active WebSocket connection.
pub async fn handle_ws(ws: WebSocket, session: UserSession, app_state: Arc<AppState>) {
    // The `.split()` method is now available because `StreamExt` is in scope.
    let (mut ws_sender, mut ws_receiver) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // Add this user's sending channel to the global map of active connections,
    // using the unique session_key as the identifier for this specific connection.
    app_state
        .active_connections
        .lock()
        .await
        .insert(session.session_key.clone(), tx);
    
    // Announce to everyone that this user is now online.
    // This will broadcast the status based on the user_id,
    // which should update all instances of that user in others' contact lists.
    broadcast_status(&app_state, &session, "online").await;

    // This task forwards messages from the channel to the client's WebSocket sender.
    tokio::spawn(async move {
        while let Some(message_to_send) = rx.recv().await {
            if ws_sender.send(message_to_send).await.is_err() {
                // Client disconnected.
                break;
            }
        }
    });

    // This loop handles incoming messages from the client.
    while let Some(Ok(msg)) = ws_receiver.next().await {
        if let Ok(text) = msg.to_str() {
            match serde_json::from_str::<ClientMessage>(text) { // Changed ClientWebSocketMessage to ClientMessage
                Ok(client_msg) => {
                    handle_client_message(client_msg, &session, &app_state).await;
                }
                Err(e) => {
                    eprintln!("Error deserializing client message: {}", e);
                }
            }
        }
    }

    // -- Cleanup on Disconnect --
    println!("User '{}' (session: {}) disconnected.", session.username, session.session_key);
    // Remove the connection using its unique session key.
    app_state
        .active_connections
        .lock()
        .await
        .remove(&session.session_key);
    
    // Announce to everyone that this user is now offline.
    // This will broadcast the status based on the user_id.
    // Note: A user is only truly "offline" if ALL their sessions are disconnected.
    // For simplicity here, we broadcast if *this* session disconnects.
    // A more robust solution would track active session count per user.
    broadcast_status(&app_state, &session, "offline").await;
}

/// Processes a deserialized message from a client and forwards it appropriately.
async fn handle_client_message(
    msg: ClientMessage,
    sender_session: &UserSession,
    app_state: &Arc<AppState>,
) {
    let connections_lock = app_state.active_connections.lock().await;
    let user_sessions_lock = app_state.user_sessions.lock().await;


    match msg {
        ClientMessage::ChatMessage { to_user_id, message } => {
            let server_msg = ServerMessage::ChatMessage {
                from_user_id: sender_session.user_id,
                from_username: sender_session.username.clone(),
                to_user_id,
                message_id: Uuid::new_v4().to_string(),
                timestamp: Utc::now().to_rfc3339(),
                message,
            };

            if let Ok(json) = serde_json::to_string(&server_msg) {
                // Send to ALL active sessions belonging to the recipient user
                for (session_key, tx) in connections_lock.iter() {
                    if let Some(target_session) = user_sessions_lock.get(session_key) {
                        if target_session.user_id == to_user_id {
                             let _ = tx.send(Message::text(json.clone()));
                        }
                    }
                }
                // Also send back to all sessions of the sender for UI sync
                for (session_key, tx) in connections_lock.iter() {
                    if let Some(target_session) = user_sessions_lock.get(session_key) {
                        if target_session.user_id == sender_session.user_id {
                             let _ = tx.send(Message::text(json.clone()));
                        }
                    }
                }
            }
        }
        ClientMessage::TypingIndicator { to_user_id, is_typing } => {
            let server_msg = ServerMessage::TypingIndicator {
                from_user_id: sender_session.user_id,
                is_typing,
            };
            if let Ok(json) = serde_json::to_string(&server_msg) {
                for (session_key, tx) in connections_lock.iter() {
                    if let Some(target_session) = user_sessions_lock.get(session_key) {
                        // Typing indicators only go to sessions of the recipient user
                        if target_session.user_id == to_user_id {
                             let _ = tx.send(Message::text(json.clone()));
                        }
                    }
                }
            }
        }
        ClientMessage::ReadReceipt { to_user_id, message_id } => {
            let server_msg = ServerMessage::ReadReceipt {
                from_user_id: sender_session.user_id, // The user who just read the message.
                message_id,
            };
            if let Ok(json) = serde_json::to_string(&server_msg) {
                for (session_key, tx) in connections_lock.iter() {
                    if let Some(target_session) = user_sessions_lock.get(session_key) {
                        // Read receipts only go to sessions of the original message sender (to_user_id here refers to the original sender's ID)
                        if target_session.user_id == to_user_id {
                             let _ = tx.send(Message::text(json.clone()));
                        }
                    }
                }
            }
        }
    }
}


/// Broadcasts a user's status to all other connected clients.
async fn broadcast_status(app_state: &Arc<AppState>, session: &UserSession, status: &str) {
    let status_msg = ServerMessage::StatusMessage {
        user_id: session.user_id,
        username: session.username.clone(),
        status: status.to_string(),
    };
    if let Ok(text) = serde_json::to_string(&status_msg) {
        let msg = Message::text(text);
        
        let connections = app_state.active_connections.lock().await;

        for (other_session_key, tx) in connections.iter() {
            // Send to all *other* sessions of *other* users, or other sessions of the same user.
            // A status update (online/offline) should typically be seen by everyone.
            // The logic here is to send to all connections EXCEPT the one that triggered the broadcast.
            if *other_session_key != session.session_key {
                let _ = tx.send(msg.clone());
            }
        }
    }
}


// --- HTTP Handlers ---

// Structs for strongly-typed request bodies.
#[derive(Deserialize)]
pub struct AuthPayload {
    username: String,
    password: String,
}

#[derive(Deserialize)]
pub struct AddContactPayload {
    contact_username: String,
}

// Struct for a consistent successful authentication response.
#[derive(Serialize)]
pub struct AuthResponse {
    message: String,
    session_key: String,
    user_id: Uuid,
    username: String,
}


pub async fn register_handler(
    payload: AuthPayload,
    app_state: Arc<AppState>,
) -> Result<impl Reply, Rejection> {
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(warp::reject::custom(ErrorResponse {
            message: "Username and password are required.".into(),
        }));
    }

    let mut users = app_state.users.lock().await;
    if users.contains_key(&payload.username) {
        return Err(warp::reject::custom(ErrorResponse {
            message: "Username already exists.".into(),
        }));
    }

    // Securely hash the password before storing.
    let password_hash = match bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST) {
        Ok(hash) => hash,
        Err(_) => return Err(warp::reject::custom(ErrorResponse {
            message: "Failed to hash password.".into()
        })),
    };

    let user = User {
        id: Uuid::new_v4(),
        username: payload.username.clone(),
        password_hash,
        contacts: Arc::new(Mutex::new(HashMap::new())),
    };

    let response = create_session(&user, app_state.clone()).await;
    users.insert(payload.username.to_string(), user);
    println!("Registered user: {} ({})", payload.username, response.user_id); // Added log
    Ok(warp::reply::json(&response))
}


pub async fn login_handler(
    payload: AuthPayload,
    app_state: Arc<AppState>,
) -> Result<impl Reply, Rejection> {
     if payload.username.is_empty() || payload.password.is_empty() {
        return Err(warp::reject::custom(ErrorResponse { message: "Username and password are required.".into() }));
    }

    let users = app_state.users.lock().await;
    match users.get(&payload.username) {
        Some(user) => {
            // Securely verify the password against the stored hash.
            let is_valid = bcrypt::verify(&payload.password, &user.password_hash).unwrap_or(false);

            if is_valid {
                let response = create_session(user, app_state.clone()).await;
                println!("Logged in user: {} ({})", payload.username, response.user_id); // Added log
                Ok(warp::reply::json(&response))
            } else {
                Err(warp::reject::custom(ErrorResponse { message: "Invalid username or password.".into() }))
            }
        }
        None => Err(warp::reject::custom(ErrorResponse { message: "Invalid username or password.".into() })),
    }
}

/// Helper function to create a new session for a user.
async fn create_session(user: &User, app_state: Arc<AppState>) -> AuthResponse {
    let new_session_key = Uuid::new_v4().to_string();
    
    // --- Invalidate all old sessions and their WebSocket connections for this user_id ---
    let mut user_sessions_guard = app_state.user_sessions.lock().await;
    let mut active_connections_guard = app_state.active_connections.lock().await;

    // Collect session keys to remove
    let session_keys_to_remove: Vec<String> = user_sessions_guard
        .iter()
        .filter(|(_, session)| session.user_id == user.id)
        .map(|(session_key, _)| session_key.clone())
        .collect();

    for old_session_key in session_keys_to_remove {
        user_sessions_guard.remove(&old_session_key);
        if active_connections_guard.remove(&old_session_key).is_some() {
            println!("Closed old WebSocket connection for user {} (session: {})", user.username, old_session_key);
        }
    }
    // --- End Invalidation ---

    let new_session = UserSession {
        user_id: user.id,
        username: user.username.clone(),
        session_key: new_session_key.clone(),
    };
    user_sessions_guard.insert(new_session_key.clone(), new_session);

    AuthResponse {
        message: "Authentication successful".to_string(),
        session_key: new_session_key,
        user_id: user.id,
        username: user.username.clone(),
    }
}

pub async fn add_contact_handler(
    payload: AddContactPayload,
    session: UserSession,
    app_state: Arc<AppState>,
) -> Result<impl Reply, Rejection> {
    let contact_username = payload.contact_username;

    if contact_username.is_empty() {
        eprintln!("Add contact failed: contact_username is empty for user {}", session.username);
        return Err(warp::reject::custom(ErrorResponse { message: "contact_username cannot be empty".to_string() }));
    }
    
    if contact_username == session.username {
        eprintln!("Add contact failed: user {} tried to add themselves as a contact", session.username);
        return Err(warp::reject::custom(ErrorResponse { message: "You cannot add yourself as a contact.".to_string() }));
    }

    let users_guard = app_state.users.lock().await; // Acquire read lock once
    
    let current_user_opt = users_guard.get(&session.username).cloned();
    let contact_to_add_opt = users_guard.get(&contact_username).cloned();

    // Explicitly drop the guard to release the read lock on the main `users` HashMap.
    drop(users_guard);

    let current_user = match current_user_opt {
        Some(u) => u,
        None => {
            eprintln!("Add contact failed: current user '{}' not found in users map (session might be invalid)", session.username);
            return Err(warp::reject::custom(ErrorResponse { message: "User session invalid or user data missing.".to_string() }));
        }
    };

    let contact_to_add = match contact_to_add_opt {
        Some(c) => c,
        None => {
            eprintln!("Add contact failed: contact user '{}' not found for user {}", contact_username, session.username);
            return Err(warp::reject::custom(ErrorResponse { message: "User not found".to_string() }));
        }
    };

    // Now, acquire mutable locks on the individual `contacts` HashMaps.
    let mut current_user_contacts = current_user.contacts.lock().await;
    let mut contact_to_add_contacts = contact_to_add.contacts.lock().await;

    // Add each user to the other's contact list for a mutual connection.
    current_user_contacts.insert(contact_to_add.id, contact_to_add.username.clone());
    contact_to_add_contacts.insert(current_user.id, current_user.username.clone());

    println!("User '{}' (ID: {}) successfully added '{}' (ID: {}) as a contact.", 
             session.username, session.user_id, contact_username, contact_to_add.id);
    
    // Debugging: Print current user's contacts after adding
    println!("{}'s contacts after adding {}: {:?}", session.username, contact_username, current_user_contacts.keys().collect::<Vec<_>>());

    Ok(StatusCode::OK)
}

pub async fn get_contacts_handler(
    session: UserSession,
    app_state: Arc<AppState>,
) -> Result<impl Reply, Rejection> {
    let users = app_state.users.lock().await;
    if let Some(user) = users.get(&session.username) {
        let contacts_map = user.contacts.lock().await;
        let contacts_list: Vec<_> = contacts_map.iter().map(|(id, username)| {
            serde_json::json!({ "id": id, "username": username })
        }).collect();
        println!("Retrieving contacts for user {}: {:?}", session.username, contacts_list); // Added log
        Ok(warp::reply::json(&contacts_list))
    } else {
        eprintln!("Get contacts failed: User '{}' not found in users map during contacts retrieval.", session.username);
        Err(warp::reject::custom(ErrorResponse { message: "User session invalid or user data missing.".to_string() }))
    }
}
