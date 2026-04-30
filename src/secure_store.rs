use std::{fs, path::PathBuf};

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, generic_array::GenericArray},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use directories::ProjectDirs;
use rand::RngCore;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct SecureStore {
    key: [u8; 32],
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedPayload {
    nonce: String,
    ciphertext: String,
}

impl SecureStore {
    pub fn new(base64_key: &str) -> Result<Self, AppError> {
        let decoded = STANDARD.decode(base64_key.trim()).map_err(|err| {
            AppError::Config(format!("invalid TWITCH_TOKEN_ENCRYPTION_KEY: {err}"))
        })?;
        let key: [u8; 32] = decoded.try_into().map_err(|_| {
            AppError::Config(
                "invalid TWITCH_TOKEN_ENCRYPTION_KEY: expected base64 for 32 raw bytes".to_string(),
            )
        })?;
        Ok(Self { key })
    }

    pub fn load_json<T: DeserializeOwned>(&self, path: &PathBuf) -> Result<Option<T>, String> {
        let raw = match fs::read_to_string(path) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("read secure file failed: {e}")),
        };

        let payload: EncryptedPayload =
            toml::from_str(&raw).map_err(|e| format!("decode encrypted payload failed: {e}"))?;

        let nonce_bytes = STANDARD
            .decode(payload.nonce)
            .map_err(|e| format!("decode nonce failed: {e}"))?;
        let ciphertext = STANDARD
            .decode(payload.ciphertext)
            .map_err(|e| format!("decode ciphertext failed: {e}"))?;

        let plaintext = self.decrypt(&nonce_bytes, &ciphertext)?;
        let decoded = serde_json::from_slice::<T>(&plaintext)
            .map_err(|e| format!("decode secure json failed: {e}"))?;
        Ok(Some(decoded))
    }

    pub fn save_json<T: Serialize>(&self, path: &PathBuf, value: &T) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("create secure store dir failed: {e}"))?;
        }

        let plaintext =
            serde_json::to_vec(value).map_err(|e| format!("encode secure json failed: {e}"))?;
        let mut nonce_bytes = [0_u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let ciphertext = self.encrypt(&nonce_bytes, &plaintext)?;

        let payload = EncryptedPayload {
            nonce: STANDARD.encode(nonce_bytes),
            ciphertext: STANDARD.encode(ciphertext),
        };

        let encoded = toml::to_string_pretty(&payload)
            .map_err(|e| format!("encode encrypted payload failed: {e}"))?;
        fs::write(path, encoded).map_err(|e| format!("write secure store failed: {e}"))
    }

    pub fn delete(&self, path: &PathBuf) -> Result<(), String> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("delete secure file failed: {e}")),
        }
    }

    fn encrypt(&self, nonce_bytes: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = Aes256Gcm::new(GenericArray::from_slice(&self.key));
        let nonce = Nonce::from_slice(nonce_bytes);
        cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| "encryption failed".to_string())
    }

    fn decrypt(&self, nonce_bytes: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = Aes256Gcm::new(GenericArray::from_slice(&self.key));
        let nonce = Nonce::from_slice(nonce_bytes);
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| "decryption failed".to_string())
    }
}

pub fn twitch_account_store_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "", "twitch-relay")?;
    Some(dirs.data_local_dir().join("twitch-account.toml"))
}
