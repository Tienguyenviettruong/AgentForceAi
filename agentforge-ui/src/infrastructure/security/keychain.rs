use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ring::aead::{self, Aad, LessSafeKey, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};

pub const SECURE_SECRET_SERVICE: &str = "agentforge-mcp";
const INVOCATION_KEY_ACCOUNT: &str = "tool-invocation-seal-v1";
const INVOCATION_KEY_ENV: &str = "AGENTFORGE_INVOCATION_SEAL_KEY";

pub struct Keychain;

impl Keychain {
    pub async fn new() -> Result<Self> {
        Ok(Self)
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub async fn get_secret(&self, service: &str, account: &str) -> Result<Option<String>> {
        let service = service.to_string();
        let account = account.to_string();
        smol::unblock(move || {
            let entry = keyring::Entry::new(&service, &account)
                .map_err(|error| anyhow!("Unable to open OS credential entry: {}", error))?;
            match entry.get_password() {
                Ok(secret) => Ok(Some(secret)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(error) => Err(anyhow!("Unable to read OS credential entry: {}", error)),
            }
        })
        .await
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    pub async fn get_secret(&self, _service: &str, _account: &str) -> Result<Option<String>> {
        Err(anyhow!(
            "OS credential storage is unavailable on this platform build."
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub async fn set_secret(&self, service: &str, account: &str, secret: &str) -> Result<()> {
        let service = service.to_string();
        let account = account.to_string();
        let secret = secret.to_string();
        smol::unblock(move || {
            let entry = keyring::Entry::new(&service, &account)
                .map_err(|error| anyhow!("Unable to open OS credential entry: {}", error))?;
            entry
                .set_password(&secret)
                .map_err(|error| anyhow!("Unable to store OS credential: {}", error))
        })
        .await
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    pub async fn set_secret(&self, _service: &str, _account: &str, _secret: &str) -> Result<()> {
        Err(anyhow!(
            "OS credential storage is unavailable on this platform build."
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub async fn delete_secret(&self, service: &str, account: &str) -> Result<()> {
        let service = service.to_string();
        let account = account.to_string();
        smol::unblock(move || {
            let entry = keyring::Entry::new(&service, &account)
                .map_err(|error| anyhow!("Unable to open OS credential entry: {}", error))?;
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(error) => Err(anyhow!("Unable to delete OS credential: {}", error)),
            }
        })
        .await
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    pub async fn delete_secret(&self, _service: &str, _account: &str) -> Result<()> {
        Err(anyhow!(
            "OS credential storage is unavailable on this platform build."
        ))
    }
}

pub fn is_credential_reference(value: &str) -> bool {
    let value = value.trim();
    value
        .strip_prefix("secret://")
        .map(|account| !account.trim().is_empty())
        .unwrap_or(false)
        || value
            .strip_prefix("env:")
            .map(|name| !name.trim().is_empty())
            .unwrap_or(false)
}

pub async fn resolve_credential_reference(
    reference: Option<&str>,
    fallback_env_keys: &[&str],
) -> Result<Option<String>> {
    if let Some(reference) = reference.map(str::trim).filter(|value| !value.is_empty()) {
        if let Some(account) = reference.strip_prefix("secret://") {
            if account.trim().is_empty() {
                return Err(anyhow!("Secret reference account cannot be empty."));
            }
            let secret = Keychain::new()
                .await?
                .get_secret(SECURE_SECRET_SERVICE, account)
                .await?
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    anyhow!(
                        "Secret '{}' was not found in OS credential storage.",
                        account
                    )
                })?;
            return Ok(Some(secret));
        }

        if let Some(name) = reference.strip_prefix("env:") {
            if name.trim().is_empty() {
                return Err(anyhow!("Environment credential reference cannot be empty."));
            }
            return std::env::var(name)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .map(Some)
                .ok_or_else(|| anyhow!("Credential environment variable '{}' is missing.", name));
        }

        return Err(anyhow!(
            "Stored raw credentials are blocked. Configure a secret:// or env: reference."
        ));
    }

    Ok(fallback_env_keys.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    }))
}

fn decode_invocation_key(encoded: &str) -> Result<[u8; 32]> {
    let bytes = BASE64
        .decode(encoded.trim())
        .map_err(|_| anyhow!("Invocation seal key must be base64-encoded."))?;
    bytes
        .try_into()
        .map_err(|_| anyhow!("Invocation seal key must contain exactly 32 bytes."))
}

fn load_invocation_key() -> Result<[u8; 32]> {
    if let Ok(encoded) = std::env::var(INVOCATION_KEY_ENV) {
        return decode_invocation_key(&encoded);
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let entry = keyring::Entry::new(SECURE_SECRET_SERVICE, INVOCATION_KEY_ACCOUNT)
            .map_err(|error| anyhow!("Unable to open invocation seal key entry: {}", error))?;
        match entry.get_password() {
            Ok(encoded) => return decode_invocation_key(&encoded),
            Err(keyring::Error::NoEntry) => {
                let mut bytes = [0u8; 32];
                SystemRandom::new()
                    .fill(&mut bytes)
                    .map_err(|_| anyhow!("Unable to generate invocation seal key."))?;
                entry
                    .set_password(&BASE64.encode(bytes))
                    .map_err(|error| anyhow!("Unable to store invocation seal key: {}", error))?;
                return Ok(bytes);
            }
            Err(error) => {
                return Err(anyhow!("Unable to read invocation seal key: {}", error));
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err(anyhow!(
            "{} must provide a base64 32-byte key on this platform.",
            INVOCATION_KEY_ENV
        ))
    }
}

pub fn seal_sensitive_payload(plaintext: &str, associated_data: &str) -> Result<String> {
    let key = load_invocation_key()?;
    let unbound = UnboundKey::new(&aead::AES_256_GCM, &key)
        .map_err(|_| anyhow!("Unable to initialize invocation payload encryption."))?;
    let key = LessSafeKey::new(unbound);
    let mut nonce_bytes = [0u8; 12];
    SystemRandom::new()
        .fill(&mut nonce_bytes)
        .map_err(|_| anyhow!("Unable to generate invocation payload nonce."))?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut encrypted = plaintext.as_bytes().to_vec();
    key.seal_in_place_append_tag(nonce, Aad::from(associated_data.as_bytes()), &mut encrypted)
        .map_err(|_| anyhow!("Unable to seal invocation payload."))?;
    let mut output = nonce_bytes.to_vec();
    output.extend(encrypted);
    Ok(format!("v1:{}", BASE64.encode(output)))
}

pub fn open_sensitive_payload(ciphertext: &str, associated_data: &str) -> Result<String> {
    let encoded = ciphertext
        .strip_prefix("v1:")
        .ok_or_else(|| anyhow!("Stored invocation payload is not encrypted."))?;
    let bytes = BASE64
        .decode(encoded)
        .map_err(|_| anyhow!("Encrypted invocation payload is invalid."))?;
    if bytes.len() <= 12 {
        return Err(anyhow!("Encrypted invocation payload is truncated."));
    }
    let key = load_invocation_key()?;
    let unbound = UnboundKey::new(&aead::AES_256_GCM, &key)
        .map_err(|_| anyhow!("Unable to initialize invocation payload decryption."))?;
    let key = LessSafeKey::new(unbound);
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes.copy_from_slice(&bytes[..12]);
    let mut encrypted = bytes[12..].to_vec();
    let plaintext = key
        .open_in_place(
            Nonce::assume_unique_for_key(nonce_bytes),
            Aad::from(associated_data.as_bytes()),
            &mut encrypted,
        )
        .map_err(|_| anyhow!("Encrypted invocation payload could not be authenticated."))?;
    String::from_utf8(plaintext.to_vec())
        .map_err(|_| anyhow!("Decrypted invocation payload is not valid UTF-8."))
}

#[cfg(test)]
mod tests {
    use super::is_credential_reference;

    #[test]
    fn accepts_only_opaque_or_environment_references() {
        assert!(is_credential_reference("secret://provider-main"));
        assert!(is_credential_reference(" env:OPENROUTER_API_KEY "));
        assert!(!is_credential_reference("secret://"));
        assert!(!is_credential_reference("env:"));
        assert!(!is_credential_reference("sk-raw-value"));
    }
}
