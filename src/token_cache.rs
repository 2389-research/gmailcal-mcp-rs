use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{decode, encode};
use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::errors::{ConfigError, GmailApiError, GmailResult};

/// Cached token information
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expiry_timestamp: u64, // Unix timestamp when token expires
}

/// Configuration for the token cache
#[derive(Debug, Clone)]
pub struct TokenCacheConfig {
    pub enabled: bool,
    pub cache_file_path: PathBuf,
    pub encryption_key: Vec<u8>,
}

impl TokenCacheConfig {
    /// Create a new TokenCacheConfig from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        // Check if token caching is enabled
        let enabled = std::env::var("TOKEN_CACHE_ENABLED")
            .map(|s| s.to_lowercase() == "true" || s == "1")
            .unwrap_or(false);

        if !enabled {
            debug!("Token caching is disabled");
            // Return with default values even though it's disabled
            return Ok(Self {
                enabled: false,
                cache_file_path: default_cache_path(),
                encryption_key: generate_encryption_key("default_unused_key"),
            });
        }

        // Get cache file path
        let cache_file_path = match std::env::var("TOKEN_CACHE_FILE") {
            Ok(path) => PathBuf::from(path),
            Err(_) => default_cache_path(),
        };

        // Get encryption key - NEVER log this
        let encryption_key = match std::env::var("TOKEN_CACHE_ENCRYPTION_KEY") {
            Ok(key) => generate_encryption_key(&key),
            Err(_) => {
                warn!("TOKEN_CACHE_ENCRYPTION_KEY not found, using less secure device-derived key");
                fallback_encryption_key()
            }
        };

        debug!(
            "Token cache configured to use file: {}",
            cache_file_path.display()
        );

        Ok(Self {
            enabled,
            cache_file_path,
            encryption_key,
        })
    }
}

/// TokenCache provides secure persistence of OAuth tokens between application runs
#[derive(Clone)]
pub struct TokenCache {
    config: TokenCacheConfig,
    cipher: Aes256Gcm,
}

// Manual Debug implementation since Aes256Gcm doesn't implement Debug
impl std::fmt::Debug for TokenCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenCache")
            .field("config", &self.config)
            .field("cipher", &"<AES-GCM cipher>")
            .finish()
    }
}

impl TokenCache {
    /// Create a new TokenCache with the provided configuration
    pub fn new(config: TokenCacheConfig) -> Result<Self, GmailApiError> {
        if !config.enabled {
            debug!("Creating TokenCache (disabled)");
            // Even though it's disabled, create a valid instance
            let cipher = match Aes256Gcm::new_from_slice(&config.encryption_key) {
                Ok(cipher) => cipher,
                Err(e) => {
                    error!("Failed to initialize encryption: {}", e);
                    return Err(GmailApiError::CacheError(format!(
                        "Failed to initialize encryption: {}",
                        e
                    )));
                }
            };
            return Ok(Self { config, cipher });
        }

        debug!("Creating TokenCache (enabled)");

        // Ensure parent directory exists
        if let Some(parent) = config.cache_file_path.parent() {
            if !parent.exists() {
                debug!("Creating parent directory for token cache");
                match fs::create_dir_all(parent) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Failed to create cache directory: {}", e);
                        return Err(GmailApiError::CacheError(format!(
                            "Failed to create cache directory: {}",
                            e
                        )));
                    }
                }
            }
        }

        // Initialize AES-GCM cipher
        let cipher = match Aes256Gcm::new_from_slice(&config.encryption_key) {
            Ok(cipher) => cipher,
            Err(e) => {
                error!("Failed to initialize encryption: {}", e);
                return Err(GmailApiError::CacheError(format!(
                    "Failed to initialize encryption: {}",
                    e
                )));
            }
        };

        Ok(Self { config, cipher })
    }

    /// Save token to the cache file
    pub fn save_token(
        &self,
        access_token: &str,
        refresh_token: &str,
        expiry: SystemTime,
    ) -> GmailResult<()> {
        if !self.config.enabled {
            debug!("Token caching disabled, not saving token");
            return Ok(());
        }

        debug!("Saving token to cache");

        // Convert expiry to Unix timestamp
        let expiry_timestamp = match expiry.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(duration) => duration.as_secs(),
            Err(e) => {
                error!("Invalid expiry time: {}", e);
                return Err(GmailApiError::CacheError(
                    "Invalid expiry timestamp".to_string(),
                ));
            }
        };

        // Create token data
        let token = CachedToken {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.to_string(),
            expiry_timestamp,
        };

        // Serialize token
        let token_json = match serde_json::to_string(&token) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize token: {}", e);
                return Err(GmailApiError::CacheError(format!(
                    "Failed to serialize token: {}",
                    e
                )));
            }
        };

        // Encrypt token
        let encrypted_data = self.encrypt_data(token_json.as_bytes())?;

        // Write to file
        let encrypted_b64 = encode(&encrypted_data);
        match File::create(&self.config.cache_file_path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(encrypted_b64.as_bytes()) {
                    error!("Failed to write token cache: {}", e);
                    return Err(GmailApiError::CacheError(format!(
                        "Failed to write token cache: {}",
                        e
                    )));
                }
            }
            Err(e) => {
                error!("Failed to create token cache file: {}", e);
                return Err(GmailApiError::CacheError(format!(
                    "Failed to create token cache file: {}",
                    e
                )));
            }
        }

        debug!(
            "Token successfully cached to {}",
            self.config.cache_file_path.display()
        );
        Ok(())
    }

    /// Load token from the cache file
    pub fn load_token(&self) -> GmailResult<Option<CachedToken>> {
        if !self.config.enabled {
            debug!("Token caching disabled, not loading token");
            return Ok(None);
        }

        // Check if cache file exists
        if !self.config.cache_file_path.exists() {
            debug!("Token cache file not found");
            return Ok(None);
        }

        debug!("Loading token from cache");

        // Read encrypted data
        let mut file = match File::open(&self.config.cache_file_path) {
            Ok(file) => file,
            Err(e) => {
                error!("Failed to open token cache file: {}", e);
                return Err(GmailApiError::CacheError(format!(
                    "Failed to open token cache file: {}",
                    e
                )));
            }
        };

        let mut encrypted_b64 = String::new();
        if let Err(e) = file.read_to_string(&mut encrypted_b64) {
            error!("Failed to read token cache: {}", e);
            return Err(GmailApiError::CacheError(format!(
                "Failed to read token cache: {}",
                e
            )));
        }

        // Decode base64
        let encrypted_data = match decode(&encrypted_b64) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to decode cached token data: {}", e);
                return Err(GmailApiError::CacheError(format!(
                    "Invalid token cache format: {}",
                    e
                )));
            }
        };

        // Decrypt data
        let decrypted_data = match self.decrypt_data(&encrypted_data) {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to decrypt token cache: {}", e);
                // If decryption fails, the cache is corrupt or key changed - delete it
                if let Err(e) = fs::remove_file(&self.config.cache_file_path) {
                    debug!("Failed to delete corrupt token cache: {}", e);
                }
                return Ok(None);
            }
        };

        // Deserialize token
        match serde_json::from_slice::<CachedToken>(&decrypted_data) {
            Ok(token) => {
                debug!("Successfully loaded token from cache");
                // Check if token is expired
                let now = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                    Ok(duration) => duration.as_secs(),
                    Err(_) => {
                        error!("System time error when checking token expiry");
                        return Ok(Some(token)); // Return token anyway, expiry will be checked elsewhere
                    }
                };

                if token.expiry_timestamp <= now {
                    debug!("Cached token has expired, will need refreshing");
                }

                Ok(Some(token))
            }
            Err(e) => {
                error!("Failed to deserialize cached token: {}", e);
                // Delete corrupt cache
                if let Err(e) = fs::remove_file(&self.config.cache_file_path) {
                    debug!("Failed to delete corrupt token cache: {}", e);
                }
                Ok(None)
            }
        }
    }

    /// Check if the token is still valid
    pub fn is_token_valid(&self, token: &CachedToken) -> bool {
        let now = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(duration) => duration.as_secs(),
            Err(_) => {
                error!("System time error when checking token validity");
                return false;
            }
        };

        // Consider token valid if it has at least 5 minutes left
        token.expiry_timestamp > now + 300
    }

    /// Delete the token cache file
    pub fn clear_cache(&self) -> GmailResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if self.config.cache_file_path.exists() {
            debug!("Clearing token cache");
            match fs::remove_file(&self.config.cache_file_path) {
                Ok(_) => {
                    debug!("Token cache cleared successfully");
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to clear token cache: {}", e);
                    Err(GmailApiError::CacheError(format!(
                        "Failed to clear token cache: {}",
                        e
                    )))
                }
            }
        } else {
            debug!("No token cache to clear");
            Ok(())
        }
    }

    // Encrypt data using AES-GCM
    fn encrypt_data(&self, data: &[u8]) -> GmailResult<Vec<u8>> {
        // Use random 12-byte nonce (IV)
        let nonce_value = rand::random::<[u8; 12]>();
        let nonce = Nonce::from_slice(&nonce_value);

        // Encrypt the data
        let ciphertext = match self.cipher.encrypt(nonce, data) {
            Ok(ciphertext) => ciphertext,
            Err(e) => {
                error!("Encryption failed: {}", e);
                return Err(GmailApiError::CacheError(format!(
                    "Encryption failed: {}",
                    e
                )));
            }
        };

        // Combine nonce and ciphertext (nonce needs to be stored for decryption)
        let mut result = nonce_value.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    // Decrypt data using AES-GCM
    fn decrypt_data(&self, data: &[u8]) -> GmailResult<Vec<u8>> {
        if data.len() < 12 {
            return Err(GmailApiError::CacheError(
                "Invalid encrypted data: too short".to_string(),
            ));
        }

        // Split data into nonce and ciphertext
        let nonce = Nonce::from_slice(&data[0..12]);
        let ciphertext = &data[12..];

        // Decrypt the data
        match self.cipher.decrypt(nonce, ciphertext) {
            Ok(plaintext) => Ok(plaintext),
            Err(e) => {
                error!("Decryption failed: {}", e);
                Err(GmailApiError::CacheError(format!(
                    "Decryption failed: {}",
                    e
                )))
            }
        }
    }
}

// Generate a consistent encryption key from a password/secret
fn generate_encryption_key(secret: &str) -> Vec<u8> {
    // Simple key derivation - in a production system, use a proper KDF like PBKDF2
    let mut key = Vec::with_capacity(32); // 256 bits for AES-256
    let source = secret.as_bytes().to_vec();

    // Pad or truncate the key to exactly 32 bytes
    match source.len().cmp(&32) {
        std::cmp::Ordering::Less => {
            // If key is too short, repeat it
            while key.len() < 32 {
                key.extend_from_slice(&source);
            }
            key.truncate(32);
        }
        std::cmp::Ordering::Greater => {
            // If key is too long, truncate it
            key.extend_from_slice(&source[0..32]);
        }
        std::cmp::Ordering::Equal => {
            // Key is exactly right size
            key = source;
        }
    }

    key
}

// Fallback encryption key derived from machine-specific information
// This is less secure than using a provided key but better than nothing
fn fallback_encryption_key() -> Vec<u8> {
    // Combine hostname and username to create a device-specific key
    let hostname = match std::process::Command::new("hostname").output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(_) => "unknown-host".to_string(),
    };

    let username = match std::env::var("USER") {
        Ok(user) => user,
        Err(_) => "unknown-user".to_string(),
    };

    // Combine and hash to get a unique key
    let combined = format!("gmail-mcp-rs-{}-{}", hostname, username);
    generate_encryption_key(&combined)
}

// Get default cache file location (platform-specific)
fn default_cache_path() -> PathBuf {
    let mut path = match dirs::cache_dir() {
        Some(cache_dir) => cache_dir,
        None => {
            // Fallback to temp directory if cache dir not available
            if let Some(temp_dir) = std::env::temp_dir().to_str() {
                PathBuf::from(temp_dir)
            } else {
                PathBuf::from("/tmp") // Unix-like systems fallback
            }
        }
    };

    path.push("gmail-mcp-rs");
    path.push("token-cache.dat");
    path
}
