use anyhow::{anyhow, Result};
use ring::aead::{self, Aad, LessSafeKey, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};

pub struct EncryptionService;

impl EncryptionService {
    /// Initialize the encryption service asynchronously.
    pub async fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Encrypt data asynchronously using a provided key.
    pub async fn encrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        let unbound = UnboundKey::new(&aead::AES_256_GCM, key)
            .map_err(|_| anyhow!("AES-256-GCM requires a 32-byte encryption key."))?;
        let key = LessSafeKey::new(unbound);
        let mut nonce_bytes = [0u8; 12];
        SystemRandom::new()
            .fill(&mut nonce_bytes)
            .map_err(|_| anyhow!("Unable to generate encryption nonce."))?;
        let mut encrypted = data.to_vec();
        key.seal_in_place_append_tag(
            Nonce::assume_unique_for_key(nonce_bytes),
            Aad::empty(),
            &mut encrypted,
        )
        .map_err(|_| anyhow!("Unable to encrypt data."))?;
        let mut output = nonce_bytes.to_vec();
        output.extend(encrypted);
        Ok(output)
    }

    /// Decrypt data asynchronously using a provided key.
    pub async fn decrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        if data.len() <= 12 {
            return Err(anyhow!("Encrypted payload is truncated."));
        }
        let unbound = UnboundKey::new(&aead::AES_256_GCM, key)
            .map_err(|_| anyhow!("AES-256-GCM requires a 32-byte encryption key."))?;
        let key = LessSafeKey::new(unbound);
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes.copy_from_slice(&data[..12]);
        let mut encrypted = data[12..].to_vec();
        let plaintext = key
            .open_in_place(
                Nonce::assume_unique_for_key(nonce_bytes),
                Aad::empty(),
                &mut encrypted,
            )
            .map_err(|_| anyhow!("Unable to authenticate encrypted data."))?;
        Ok(plaintext.to_vec())
    }

    /// Generate a secure random encryption key asynchronously.
    pub async fn generate_key(length: usize) -> Result<Vec<u8>> {
        if length == 0 {
            return Err(anyhow!("Encryption key length must be greater than zero."));
        }
        let mut key = vec![0u8; length];
        SystemRandom::new()
            .fill(&mut key)
            .map_err(|_| anyhow!("Unable to generate encryption key."))?;
        Ok(key)
    }
}
