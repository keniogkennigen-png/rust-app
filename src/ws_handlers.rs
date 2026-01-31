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
use bcrypt;

#[derive(Debug)]
pub struct AuthError;
impl warp::reject::Reject for AuthError {}

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

async fn handle_ws(ws: WebSocket, session: UserSession, app_state: Arc<AppState>) {
    let (mut ws_sender, mut ws_receiver) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    app_state.active_connections.lock().await.insert(session.session_key.clone(), tx);
    
    tokio::spawn(async move {
        while let Some(message_to_send) = rx.recv().await {
            if ws_sender.send(message_to_send).await.is_err() { break; }
        }
    });

    while let Some(Ok(msg)) = ws_receiver.next().await {
        // Handle incoming messages...
    }

    app_state.active_connections.lock().await.remove(&session.session_key);
}

// --- HTTP Handlers ---

#[derive(Deserialize)]
pub struct AuthPayload { 
    pub username: String, 
    pub password: String 
}

#[derive(Deserialize)]
pub struct AddContactPayload { 
    pub contact_username: String 
}

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
    users.insert(payload.username.clone(), user);
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
