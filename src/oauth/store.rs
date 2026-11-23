// Copyright 2026 Alexey Chernyshov
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Encrypted, persistent storage for registered OAuth clients, in-flight
//! authorization codes, and per-user sessions (a Gramps refresh token plus a
//! cached Gramps access token). Short-lived access tokens we hand out are
//! deliberately *not* persisted — only the long-lived Gramps refresh token
//! needs to survive a server restart; a client just gets a fresh access token
//! via the refresh grant if one is needed.

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::Engine as _;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("storage encryption error: {0}")]
    Crypto(String),
    #[error("storage serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("invalid MCP_OAUTH_STORAGE_KEY: {0}")]
    InvalidKey(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRecord {
    pub redirect_uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCode {
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub gramps_refresh_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRow {
    pub gramps_refresh_token: String,
    pub cached_access_token: Option<String>,
    pub cached_expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Persisted {
    clients: HashMap<String, ClientRecord>,
    codes: HashMap<String, AuthCode>,
    /// our (opaque, rotating) refresh token -> session id
    refresh_tokens: HashMap<String, String>,
    /// session id -> Gramps identity for that session
    sessions: HashMap<String, SessionRow>,
}

#[derive(Debug, Clone)]
struct AccessTokenRow {
    session_id: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Default)]
struct State {
    persisted: Persisted,
    /// our (opaque, short-lived) access token -> session id + expiry. Never
    /// written to disk.
    access_tokens: HashMap<String, AccessTokenRow>,
}

pub struct Store {
    path: PathBuf,
    key: [u8; 32],
    state: Mutex<State>,
}

fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::rng().fill_bytes(&mut buf);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
}

impl Store {
    pub async fn open(path: impl Into<PathBuf>, key_b64: &str) -> Result<Arc<Self>> {
        let key_bytes = base64::engine::general_purpose::STANDARD
            .decode(key_b64.trim())
            .map_err(|e| Error::InvalidKey(format!("not valid base64: {e}")))?;
        let key: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| Error::InvalidKey("must decode to exactly 32 bytes".into()))?;
        let path = path.into();

        let persisted = match tokio::fs::read(&path).await {
            Ok(bytes) => decrypt(&key, &bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Persisted::default(),
            Err(e) => return Err(e.into()),
        };

        Ok(Arc::new(Self {
            path,
            key,
            state: Mutex::new(State {
                persisted,
                access_tokens: HashMap::new(),
            }),
        }))
    }

    async fn persist(&self, persisted: &Persisted) -> Result<()> {
        let bytes = encrypt(&self.key, persisted)?;
        let tmp = self.path.with_extension("tmp");
        tokio::fs::write(&tmp, bytes).await?;
        tokio::fs::rename(&tmp, &self.path).await?;
        Ok(())
    }

    pub async fn register_client(&self, redirect_uris: Vec<String>) -> Result<String> {
        let client_id = random_token(16);
        let mut state = self.state.lock().await;
        state
            .persisted
            .clients
            .insert(client_id.clone(), ClientRecord { redirect_uris });
        self.persist(&state.persisted).await?;
        Ok(client_id)
    }

    pub async fn get_client(&self, client_id: &str) -> Option<ClientRecord> {
        self.state
            .lock()
            .await
            .persisted
            .clients
            .get(client_id)
            .cloned()
    }

    pub async fn put_code(&self, code: AuthCode) -> Result<String> {
        let id = random_token(24);
        let mut state = self.state.lock().await;
        state.persisted.codes.insert(id.clone(), code);
        self.persist(&state.persisted).await?;
        Ok(id)
    }

    /// Consumes a code: single-use, and rejected once past `expires_at`.
    pub async fn take_code(&self, code: &str) -> Result<Option<AuthCode>> {
        let mut state = self.state.lock().await;
        let Some(rec) = state.persisted.codes.remove(code) else {
            return Ok(None);
        };
        self.persist(&state.persisted).await?;
        if rec.expires_at < chrono::Utc::now() {
            return Ok(None);
        }
        Ok(Some(rec))
    }

    /// Creates a brand-new session (first login) and returns its initial
    /// (access_token, refresh_token) pair.
    pub async fn create_session(
        &self,
        gramps_refresh_token: String,
        access_ttl: chrono::Duration,
    ) -> Result<(String, String)> {
        let session_id = random_token(16);
        let refresh_token = random_token(32);
        let access_token = random_token(32);

        let mut state = self.state.lock().await;
        state.persisted.sessions.insert(
            session_id.clone(),
            SessionRow {
                gramps_refresh_token,
                cached_access_token: None,
                cached_expires_at: None,
            },
        );
        state
            .persisted
            .refresh_tokens
            .insert(refresh_token.clone(), session_id.clone());
        self.persist(&state.persisted).await?;
        state.access_tokens.insert(
            access_token.clone(),
            AccessTokenRow {
                session_id,
                expires_at: chrono::Utc::now() + access_ttl,
            },
        );
        Ok((access_token, refresh_token))
    }

    /// Rotates a refresh token per OAuth 2.1 (old one stops working), issuing
    /// a fresh access token for the same session.
    pub async fn rotate_refresh_token(
        &self,
        old_refresh_token: &str,
        access_ttl: chrono::Duration,
    ) -> Result<Option<(String, String)>> {
        let mut state = self.state.lock().await;
        let Some(session_id) = state.persisted.refresh_tokens.remove(old_refresh_token) else {
            return Ok(None);
        };
        let new_refresh_token = random_token(32);
        state
            .persisted
            .refresh_tokens
            .insert(new_refresh_token.clone(), session_id.clone());
        self.persist(&state.persisted).await?;

        let new_access_token = random_token(32);
        state.access_tokens.insert(
            new_access_token.clone(),
            AccessTokenRow {
                session_id,
                expires_at: chrono::Utc::now() + access_ttl,
            },
        );
        Ok(Some((new_access_token, new_refresh_token)))
    }

    /// Resolves an incoming Bearer access token to its session's current
    /// Gramps identity, sweeping it out if expired.
    pub async fn session_for_access_token(&self, access_token: &str) -> Option<SessionRow> {
        let mut state = self.state.lock().await;
        let row = state.access_tokens.get(access_token)?;
        if row.expires_at < chrono::Utc::now() {
            state.access_tokens.remove(access_token);
            return None;
        }
        let session_id = row.session_id.clone();
        state.persisted.sessions.get(&session_id).cloned()
    }

    /// Updates a session's cached Gramps access token (called after a Gramps
    /// `/api/token/refresh/` round trip). Looked up the same way as above —
    /// by the caller's own Bearer access token, so it shares exactly one
    /// session resolution path with `session_for_access_token`.
    pub async fn update_cached_gramps_token(
        &self,
        access_token: &str,
        cached_access_token: String,
        cached_expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let mut state = self.state.lock().await;
        let Some(session_id) = state
            .access_tokens
            .get(access_token)
            .map(|r| r.session_id.clone())
        else {
            return Ok(());
        };
        if let Some(session) = state.persisted.sessions.get_mut(&session_id) {
            session.cached_access_token = Some(cached_access_token);
            session.cached_expires_at = Some(cached_expires_at);
        }
        self.persist(&state.persisted).await
    }
}

fn encrypt(key: &[u8; 32], data: &Persisted) -> Result<Vec<u8>> {
    let plaintext = serde_json::to_vec(data)?;
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::try_from(key.as_slice()).expect("32-byte key"));
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::try_from(nonce_bytes.as_slice()).expect("12-byte nonce");
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_ref())
        .map_err(|e| Error::Crypto(e.to_string()))?;
    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn decrypt(key: &[u8; 32], blob: &[u8]) -> Result<Persisted> {
    if blob.len() < 12 {
        return Err(Error::Crypto("storage file too short".into()));
    }
    let (nonce_bytes, ciphertext) = blob.split_at(12);
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::try_from(key.as_slice()).expect("32-byte key"));
    let nonce = Nonce::try_from(nonce_bytes).expect("12-byte nonce");
    let plaintext = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|e| Error::Crypto(e.to_string()))?;
    Ok(serde_json::from_slice(&plaintext)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> String {
        base64::engine::general_purpose::STANDARD.encode([7u8; 32])
    }

    #[tokio::test]
    async fn round_trips_through_disk() {
        let dir = tempfile_dir();
        let path = dir.join("store.json");
        let key = test_key();

        let store = Store::open(&path, &key).await.unwrap();
        let client_id = store
            .register_client(vec!["http://localhost:1234/callback".into()])
            .await
            .unwrap();

        // Re-open from disk: a fresh Store must see the same client.
        let reopened = Store::open(&path, &key).await.unwrap();
        let rec = reopened.get_client(&client_id).await.unwrap();
        assert_eq!(rec.redirect_uris, vec!["http://localhost:1234/callback"]);

        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn wrong_key_fails_to_decrypt() {
        let dir = tempfile_dir();
        let path = dir.join("store.json");
        let store = Store::open(&path, &test_key()).await.unwrap();
        store
            .register_client(vec!["http://x".into()])
            .await
            .unwrap();

        let other_key = base64::engine::general_purpose::STANDARD.encode([9u8; 32]);
        let result = Store::open(&path, &other_key).await;
        assert!(result.is_err());

        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn refresh_token_rotation_invalidates_the_old_one() {
        let dir = tempfile_dir();
        let path = dir.join("store.json");
        let store = Store::open(&path, &test_key()).await.unwrap();

        let (access1, refresh1) = store
            .create_session("gramps-refresh-abc".into(), chrono::Duration::minutes(15))
            .await
            .unwrap();
        assert!(store.session_for_access_token(&access1).await.is_some());

        let (access2, refresh2) = store
            .rotate_refresh_token(&refresh1, chrono::Duration::minutes(15))
            .await
            .unwrap()
            .expect("rotation should succeed for a valid refresh token");
        assert_ne!(refresh1, refresh2);

        // Old refresh token is now dead.
        assert!(store
            .rotate_refresh_token(&refresh1, chrono::Duration::minutes(15))
            .await
            .unwrap()
            .is_none());
        // New access token resolves to the same underlying Gramps identity.
        let session = store.session_for_access_token(&access2).await.unwrap();
        assert_eq!(session.gramps_refresh_token, "gramps-refresh-abc");

        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn auth_code_is_single_use() {
        let dir = tempfile_dir();
        let path = dir.join("store.json");
        let store = Store::open(&path, &test_key()).await.unwrap();

        let code = store
            .put_code(AuthCode {
                client_id: "c1".into(),
                redirect_uri: "http://localhost/cb".into(),
                code_challenge: "chal".into(),
                gramps_refresh_token: "grt".into(),
                expires_at: chrono::Utc::now() + chrono::Duration::seconds(60),
            })
            .await
            .unwrap();

        assert!(store.take_code(&code).await.unwrap().is_some());
        assert!(store.take_code(&code).await.unwrap().is_none());

        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn expired_auth_code_is_rejected() {
        let dir = tempfile_dir();
        let path = dir.join("store.json");
        let store = Store::open(&path, &test_key()).await.unwrap();

        let code = store
            .put_code(AuthCode {
                client_id: "c1".into(),
                redirect_uri: "http://localhost/cb".into(),
                code_challenge: "chal".into(),
                gramps_refresh_token: "grt".into(),
                expires_at: chrono::Utc::now() - chrono::Duration::seconds(1),
            })
            .await
            .unwrap();

        assert!(store.take_code(&code).await.unwrap().is_none());

        std::fs::remove_dir_all(dir).ok();
    }

    fn tempfile_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("oauth-store-test-{}", random_token(8)));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
