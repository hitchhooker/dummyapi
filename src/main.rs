use std::{net::SocketAddr, sync::Arc, collections::HashMap};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::Message,WebSocketStream};
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use serde::{Serialize, Deserialize};
use dashmap::DashMap;
use tokio::sync::broadcast;
use tokio::time::Duration;
use thiserror::Error;
use sp_core::{blake2_256, Encode};
use rand::Rng;

const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1 MB
const TIMEOUT_DURATION: Duration = Duration::from_secs(300); // 5 minutes
const OLC_ALPHABET: &str = "23456789CFGHJMPQRVWX"; // human-friendly secrets

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionedMessage {
    version: String,
    #[serde(flatten)]
    message: WebSocketMessage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ResponsePayload {
    AccountState(ResponseAccountState),
    Challenge(String),
    VerificationResult(bool),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
#[non_exhaustive]
pub enum WebSocketMessage {
    SubscribeAccountState(String),
    NotifyAccountState(NotifyAccountState),
    RequestVerificationSecret(RequestVerificationSecret),
    VerifyIdentity(VerifyIdentity),
    JsonResult(JsonResult<ResponsePayload>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NotifyAccountState {
    pub account: String,
    pub info: IdentityInfo,
    pub verification_state: VerificationState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResponseAccountState {
    pub account: String,
    pub hashed_info: String, // Changed from info: IdentityInfo
    pub verification_state: VerificationState,
    pub pending_verification_steps: Vec<(String, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerificationState {
    pub fields: HashMap<String, bool>, // Maps field name to a boolean indicating verification status
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RequestVerificationSecret {
    pub account: String,
    pub field: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerifyIdentity {
    pub account: String,
    pub field: String,
    pub secret: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "message")]
pub enum JsonResult<T> {
    Ok(T),
    Err(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Encode)]
pub enum Data {
    None,
    Raw(Vec<u8>),
    BlakeTwo256([u8; 32]),
    Sha256([u8; 32]),
    Keccak256([u8; 32]),
    ShaThree256([u8; 32]),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Encode)]
pub struct IdentityInfo {
    pub display: Data,
    pub legal: Data,
    pub web: Data,
    pub matrix: Data,
    pub email: Data,
    pub pgp_fingerprint: Option<[u8; 20]>,
    pub image: Data,
    pub twitter: Data,
    pub github: Data,
    pub discord: Data,
}

#[derive(Error, Debug)]
pub enum WebSocketError {
    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Send error: {0}")]
    SendError(#[from] tokio::sync::broadcast::error::SendError<WebSocketMessage>),
    #[error("Message too large")]
    MessageTooLarge,
    #[error("Connection timed out")]
    ConnectionTimeout,
    #[error("Connection closed")]
    ConnectionClosed,
}

pub struct WebSocketServer {
    sessions: Arc<DashMap<String, Vec<broadcast::Sender<WebSocketMessage>>>>,
    secrets: Arc<DashMap<(String, String), String>>,
    verification_states: Arc<DashMap<String, VerificationState>>,
}

impl WebSocketServer {
    pub fn new() -> Self {
        WebSocketServer {
            sessions: Arc::new(DashMap::new()),
            secrets: Arc::new(DashMap::new()),
            verification_states: Arc::new(DashMap::new()),
        }
    }

    pub async fn start(self: Arc<Self>, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = TcpListener::bind(&addr).await?;
        println!("WebSocket server listening on: {}", addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let server = self.clone();
            tokio::spawn(async move {
                if let Err(e) = server.handle_connection(stream).await {
                    eprintln!("Error in WebSocket connection: {:?}", e);
                }
            });
        }
    }

    async fn handle_connection(self: Arc<Self>, stream: TcpStream) -> Result<(), WebSocketError> {
        let ws_stream = tokio_tungstenite::accept_async(stream).await?;
        let (mut write, mut read) = ws_stream.split();
        let (tx, mut rx) = broadcast::channel(100);

        loop {
            tokio::select! {
                Some(message) = read.next() => {
                    if let Err(e) = self.process_incoming_message(message, &tx, &mut write).await {
                        eprintln!("Error processing message: {:?}", e);
                        break;
                    }
                }
                Ok(response) = rx.recv() => {
                    if let Err(e) = self.send_response(&mut write, response).await {
                        eprintln!("Error sending response: {:?}", e);
                        break;
                    }
                }
                _ = tokio::time::sleep(TIMEOUT_DURATION) => {
                    println!("Connection timed out");
                    break;
                }
            }
        }
        Ok(())
    }

    async fn process_incoming_message(
        self: &Arc<Self>,
        message: Result<Message, tokio_tungstenite::tungstenite::Error>,
        tx: &broadcast::Sender<WebSocketMessage>,
        write: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
    ) -> Result<(), WebSocketError> {
        match message? {
            Message::Text(text) => self.handle_text_message(text, tx, write).await?,
            Message::Close(_) => {
                println!("WebSocket connection closed");
                return Err(WebSocketError::ConnectionClosed);
            }
            _ => {} // Ignore other message types
        }
        Ok(())
    }

    async fn handle_text_message(
        self: &Arc<Self>,
        text: String,
        tx: &broadcast::Sender<WebSocketMessage>,
        write: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
    ) -> Result<(), WebSocketError> {
        if text.len() > MAX_MESSAGE_SIZE {
            return Err(WebSocketError::MessageTooLarge);
        }

        match serde_json::from_str::<VersionedMessage>(&text) {
            Ok(versioned_msg) => self.clone().handle_message(versioned_msg.message, tx.clone()).await?,
            Err(e) => {
                eprintln!("Error parsing message: {:?}\nMessage content: {}", e, text);
                self.send_error_response(write, format!("Invalid JSON: {:?}", e)).await?;
            }
        }
        Ok(())
    }

    async fn send_response(
        &self,
        write: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
        response: WebSocketMessage,
    ) -> Result<(), WebSocketError> {
        let response_json = serde_json::to_string(&response)?;
        write.send(Message::Text(response_json)).await?;
        Ok(())
    }

    async fn send_error_response(
        &self,
        write: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
        error: String,
    ) -> Result<(), WebSocketError> {
        let error_response = WebSocketMessage::JsonResult(JsonResult::Err(error));
        self.send_response(write, error_response).await
    }

    async fn handle_message(self: Arc<Self>, message: WebSocketMessage, sender: broadcast::Sender<WebSocketMessage>) -> Result<(), WebSocketError> {
        match message {
            WebSocketMessage::SubscribeAccountState(account) => {
                self.subscribe_account_state(account, sender).await?;
            }
            WebSocketMessage::RequestVerificationSecret(request) => {
                self.request_verification_secret(request, sender).await?;
            }
            WebSocketMessage::VerifyIdentity(verify) => {
                self.verify_identity(verify, sender).await?;
            }
            _ => {
                eprintln!("Unhandled message type received");
            }
        }
        Ok(())
    }

    async fn subscribe_account_state(&self, account: String, sender: broadcast::Sender<WebSocketMessage>) -> Result<(), WebSocketError> {
        let dummy_info = self.get_dummy_identity_info();
        let hashed_info = self.hash_identity_info(&dummy_info);

        let verification_state = self.verification_states
            .entry(account.clone())
            .or_insert_with(|| VerificationState { fields: HashMap::new() })
            .clone();

        let pending_verification_steps: Vec<(String, String)> = self.secrets
            .iter()
            .filter_map(|entry| {
                let ((acct, field), challenge) = entry.pair();
                if acct == &account {
                    Some((field.clone(), challenge.clone()))
                } else {
                    None
                }
            })
        .collect();

        let response = JsonResult::Ok(ResponsePayload::AccountState(ResponseAccountState {
            account: account.clone(),
            hashed_info,
            verification_state,
            pending_verification_steps,
        }));

        self.sessions
            .entry(account)
            .or_default()
            .push(sender.clone());

        sender.send(WebSocketMessage::JsonResult(response))?;
        Ok(())
    }

    fn hash_identity_info(&self, info: &IdentityInfo) -> String {
        let encoded_info = info.encode();
        let hash = blake2_256(&encoded_info);
        format!("0x{}", hex::encode(hash))
    }

    async fn request_verification_secret(&self, request: RequestVerificationSecret, sender: broadcast::Sender<WebSocketMessage>) -> Result<(), WebSocketError> {
        let secret = generate_base20_secret();
        self.secrets.insert((request.account.clone(), request.field.clone()), secret.clone());

        let response = JsonResult::Ok(ResponsePayload::Challenge(secret));
        sender.send(WebSocketMessage::JsonResult(response))?;
        Ok(())
    }

    // THIS IS ONLY A MOCK IMPLEMENTATION FOR TESTING FRONTEND
    async fn verify_identity(&self, verify: VerifyIdentity, sender: broadcast::Sender<WebSocketMessage>) -> Result<(), WebSocketError> {
        let stored_secret = self.secrets.get(&(verify.account.clone(), verify.field.clone()));

        let result = if let Some(stored_secret) = stored_secret {
            if *stored_secret == verify.secret {
                self.secrets.remove(&(verify.account.clone(), verify.field.clone()));
                self.update_verification_state(&verify.account, &verify.field, true);
                JsonResult::Ok(ResponsePayload::VerificationResult(true))
            } else {
                JsonResult::Err("Invalid secret".to_string())
            }
        } else {
            JsonResult::Err("No secret found".to_string())
        };

        sender.send(WebSocketMessage::JsonResult(result.clone()))?;

        if let JsonResult::Ok(ResponsePayload::VerificationResult(true)) = result {
            self.notify_account_state(verify.account.clone()).await?;
        }

        Ok(())
    }

    fn update_verification_state(&self, account: &str, field: &str, verified: bool) {
        let mut state = self.verification_states
            .entry(account.to_string())
            .or_insert_with(|| VerificationState { fields: HashMap::new() });

        state.fields.insert(field.to_string(), verified);
    }

    async fn notify_account_state(&self, account: String) -> Result<(), WebSocketError> {
        let dummy_info = self.get_dummy_identity_info();

        let verification_state = self.verification_states
            .get(&account)
            .map(|v| v.clone())
            .unwrap_or_else(|| VerificationState { fields: HashMap::new() });

        let notification = NotifyAccountState {
            account: account.clone(),
            info: dummy_info,
            verification_state,
        };

        if let Some(subscribers) = self.sessions.get(&account) {
            for subscriber in subscribers.value() {
                if let Err(e) = subscriber.send(WebSocketMessage::NotifyAccountState(notification.clone())) {
                    eprintln!("Failed to send notification: {:?}", e);
                }
            }
        }
        Ok(())
    }

    // binary bitflags 1010011001, so registrar fields: 665
    fn get_dummy_identity_info(&self) -> IdentityInfo {
        IdentityInfo {
            display: Data::Raw(b"Dummy Display".to_vec()),
            legal: Data::None,
            web: Data::None,
            matrix: Data::Raw(b"@dummy:matrix.org".to_vec()),
            email: Data::Raw(b"dummy@email.com".to_vec()),
            pgp_fingerprint: None,
            image: Data::None,
            twitter: Data::Raw(b"@dummy_twitter".to_vec()),
            github: Data::None,
            discord: Data::Raw(b"dummy_discord".to_vec()),
        }
    }
}

fn generate_base20_secret() -> String {
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| OLC_ALPHABET.chars().nth(rng.gen_range(0..20)).unwrap())
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = Arc::new(WebSocketServer::new());

    let dummy_account = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY".to_string();

    // generate dummy IdentityInfo (without judgements)
    let dummy_info = IdentityInfo {
        display: Data::Raw(b"Dummy Display".to_vec()),    // Instantly verified
        legal: Data::None,
        web: Data::None,
        matrix: Data::Raw(b"@dummy:matrix.org".to_vec()),
        email: Data::Raw(b"dummy@email.com".to_vec()),
        pgp_fingerprint: None,
        image: Data::None,
        twitter: Data::Raw(b"@dummy_twitter".to_vec()),
        github: Data::None,
        discord: Data::Raw(b"dummy_discord".to_vec()),
    };

    // create verification state, marking fields as verified based on existence of data
    let mut verification_state = VerificationState {
        fields: HashMap::new(),
    };

    // Mark `display` as instantly verified
    if let Data::Raw(_) = dummy_info.display {
        verification_state.fields.insert("display".to_string(), true);
    }

    // filter for fields we support
    for (field, data) in vec![
        ("matrix", &dummy_info.matrix),
        ("email", &dummy_info.email),
        ("twitter", &dummy_info.twitter),
        ("discord", &dummy_info.discord),
    ] {
        // if data is raw, mark as unverified and create challenge by generating secret
        if let Data::Raw(_) = data {
            verification_state.fields.insert(field.to_string(), false);
        }
    }

    // store the verification state for the dummy account
    server.verification_states.insert(dummy_account.clone(), verification_state);

    println!("Generated Identity Info: {:?}", dummy_info);
    println!("Hashed Info: {}", server.hash_identity_info(&dummy_info));
    println!("Dummy Account: {}", dummy_account);

    // pre-populate verification secrets for fields that are not instantly verified
    for (field, data) in vec![
        ("matrix", &dummy_info.matrix),
        ("email", &dummy_info.email),
        ("twitter", &dummy_info.twitter),
        ("discord", &dummy_info.discord),
    ] {
        if let Data::Raw(_) = data {
            let secret = generate_base20_secret();
            server.secrets.insert((dummy_account.clone(), field.to_string()), secret.clone());
            println!("Pre-generated secret for {}: {}", field, secret);
        }
    }

    // start the WebSocket server
    match server.start(8080).await {
        Ok(_) => println!("Server stopped normally"),
        Err(e) => eprintln!("Server error: {:?}", e),
    }

    Ok(())
}

