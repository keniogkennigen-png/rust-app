// src/ws_handlers.rs
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use warp::{
    ws::{Message, WebSocket},
    Rejection, Reply,
    http::StatusCode,
};

#[derive(Debug)]
pub struct AuthError;
impl warp::reject::Reject for AuthError {}

/// Global application state
#[derive(Debug)]
pub struct AppState {
    pub users: Mutex<HashMap<String, User>>,
    pub user_sessions: Mutex<HashMap<String, UserSession>>,
    pub active_connections: Mutex<HashMap<String, mpsc::UnboundedSender<Message>>>,
}

#[derive(Serialize, Clone, Debug)]
pub struct UserDTO {
    pub id: String,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub contacts: Arc<Mutex<HashMap<Uuid, String>>>,
}

#[derive(Clone, Debug)]
pub struct UserSession {
    pub user_id: Uuid,
    pub username: String,
    pub session_key: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub message: String,
}
impl warp::reject::Reject for ErrorResponse {}

// --- WebSocket Handlers ---

pub async fn chat_handler(
    ws: warp::ws::Ws,
    session_key: String,
    app_state: Arc<AppState>,
) -> Result<impl Reply, Rejection> {
    let sessions = app_state.user_sessions.lock().await;
    if let Some(session) = sessions.get(&session_key).cloned() {
        Ok(ws.on_upgrade(move |socket| handle_ws(socket, session, app_state)))
    } else {
        Err(warp::reject::custom(AuthError))
    }
}

// --- Internal Message Types ---

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ClientMessage {
    ChatMessage { to_user_id: String, message: String },
    TypingIndicator { to_user_id: String, is_typing: bool },
    ReadReceipt { to_user_id: String, message_id: String },
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ServerMessage {
    ChatMessage {
        from_user_id: String,
        from_username: String,
        to_user_id: String,
        message_id: String,
        timestamp: String,
        message: String,
    },
    StatusMessage { user_id: String, username: String, status: String },
    ReadReceipt { from_user_id: String, message_id: String },
    TypingIndicator { from_user_id: String, is_typing: bool },
}

// --- WebSocket Logic ---

pub async fn handle_ws(ws: WebSocket, session: UserSession, app_state: Arc<AppState>) {
    let (mut ws_sender, mut ws_receiver) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    app_state.active_connections.lock().await.insert(session.session_key.clone(), tx);
    broadcast_status(&app_state, &session, "online").await;

    tokio::spawn(async move {
        while let Some(message_to_send) = rx.recv().await {
            if ws_sender.send(message_to_send).await.is_err() { break; }
        }
    });

    while let Some(Ok(msg)) = ws_receiver.next().await {
        if let Ok(text) = msg.to_str() {
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(text) {
                handle_client_message(client_msg, &session, &app_state).await;
            }
        }
    }

    app_state.active_connections.lock().await.remove(&session.session_key);
    broadcast_status(&app_state, &session, "offline").await;
}

async fn handle_client_message(msg: ClientMessage, sender_session: &UserSession, app_state: &Arc<AppState>) {
    let connections_lock = app_state.active_connections.lock().await;
    let user_sessions_lock = app_state.user_sessions.lock().await;

    match msg {
        ClientMessage::ChatMessage { to_user_id, message } => {
            if let Ok(target_uuid) = Uuid::parse_str(&to_user_id) {
                let server_msg = ServerMessage::ChatMessage {
                    from_user_id: sender_session.user_id.to_string(),
                    from_username: sender_session.username.clone(),
                    to_user_id: to_user_id.clone(),
                    message_id: Uuid::new_v4().to_string(),
                    timestamp: Utc::now().to_rfc3339(),
                    message,
                };
                if let Ok(json) = serde_json::to_string(&server_msg) {
                    for (session_key, tx) in connections_lock.iter() {
                        if let Some(target_session) = user_sessions_lock.get(session_key) {
                            if target_session.user_id == target_uuid || target_session.user_id == sender_session.user_id {
                                 let _ = tx.send(Message::text(json.clone()));
                            }
                        }
                    }
                }
            }
        }
        _ => {} // Handle other cases similarly
    }
}

async fn broadcast_status(app_state: &Arc<AppState>, session: &UserSession, status: &str) {
    let status_msg = ServerMessage::StatusMessage {
        user_id: session.user_id.to_string(),
        username: session.username.clone(),
        status: status.to_string(),
    };
    if let Ok(text) = serde_json::to_string(&status_msg) {
        let msg = Message::text(text);
        let connections = app_state.active_connections.lock().await;
        for (other_session_key, tx) in connections.iter() {
            if *other_session_key != session.session_key {
                let _ = tx.send(msg.clone());
            }
        }
    }
}

// --- HTTP Handlers ---

#[derive(Deserialize)]
pub struct AuthPayload { pub username: String, pub password: String }

#[derive(Deserialize)]
pub struct AddContactPayload { pub contact_username: String }

#[derive(Serialize)]
pub struct AuthResponse {
    pub message: String,
    pub session_key: String,
    pub user_id: String,
    pub username: String,
}

pub async fn register_handler(payload: AuthPayload, app_state: Arc<AppState>) -> Result<impl Reply, Rejection> {
    let mut users = app_state.users.lock().await;
    if users.contains_key(&payload.username) {
        return Err(warp::reject::custom(ErrorResponse { message: "User exists.".into() }));
    }
    let password_hash = bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST).unwrap();

    let user = User {
        id: Uuid::new_v4(),
        username: payload.username.clone(),
        password_hash,
        contacts: Arc::new(Mutex::new(HashMap::new())),
    };
    
    let response = create_session(&user, app_state.clone()).await;
    users.insert(payload.username, user);
    Ok(warp::reply::json(&response))
}

pub async fn login_handler(payload: AuthPayload, app_state: Arc<AppState>) -> Result<impl Reply, Rejection> {
    let users = app_state.users.lock().await;
    if let Some(user) = users.get(&payload.username) {
        if bcrypt::verify(&payload.password, &user.password_hash).unwrap_or(false) {
            let response = create_session(user, app_state.clone()).await;
            return Ok(warp::reply::json(&response));
        }
    }
    Err(warp::reject::custom(ErrorResponse { message: "Invalid credentials.".into() }))
}

async fn create_session(user: &User, app_state: Arc<AppState>) -> AuthResponse {
    let new_session_key = Uuid::new_v4().to_string();
    let mut sessions = app_state.user_sessions.lock().await;

    sessions.insert(new_session_key.clone(), UserSession {
        user_id: user.id,
        username: user.username.clone(),
        session_key: new_session_key.clone(),
    });

    AuthResponse {
        message: "Success".into(),
        session_key: new_session_key,
        user_id: user.id.to_string(),
        username: user.username.clone(),
    }
}

pub async fn add_contact_handler(payload: AddContactPayload, session: UserSession, app_state: Arc<AppState>) -> Result<impl Reply, Rejection> {
    let users = app_state.users.lock().await;
    let current_user = users.get(&session.username).cloned();
    let contact_user = users.get(&payload.contact_username).cloned();

    if let (Some(u), Some(c)) = (current_user, contact_user) {
        u.contacts.lock().await.insert(c.id, c.username.clone());
        c.contacts.lock().await.insert(u.id, u.username.clone());
        Ok(StatusCode::OK)
    } else {
        Err(warp::reject::custom(ErrorResponse { message: "User not found.".into() }))
    }
}

pub async fn get_contacts_handler(session: UserSession, app_state: Arc<AppState>) -> Result<impl Reply, Rejection> {
    let users = app_state.users.lock().await;
    if let Some(user) = users.get(&session.username) {
        let contacts_map = user.contacts.lock().await;
        let contacts_list: Vec<UserDTO> = contacts_map.iter()
            .map(|(id, name)| UserDTO { id: id.to_string(), username: name.clone() })
            .collect();
        Ok(warp::reply::json(&contacts_list))
    } else {
        Err(warp::reject::custom(ErrorResponse { message: "Not found.".into() }))
    }
}
