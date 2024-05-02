use base64::{engine::general_purpose::STANDARD, Engine};
use ring::{
    pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
use std::{num::NonZeroU32, sync::Arc};
use tokio::sync::Semaphore;

use crate::error::{AppResult, CommonError};

const SALT_LEN: usize = 16;
const CREDENTIAL_LEN: usize = 32;
static ALG: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA256;
const ITERATIONS: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(100_000) };

pub struct PasswordHasher {
    semaphore: Arc<Semaphore>,
}

impl PasswordHasher {
    pub fn new(max_concurrent: usize) -> PasswordHasher {
        PasswordHasher {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub async fn hash_password(&self, password: String) -> AppResult<String> {
        let _permit = self.semaphore.acquire().await.unwrap();

        tokio::task::spawn_blocking(move || {
            let mut salt = [0u8; SALT_LEN];
            let rng = SystemRandom::new();

            rng.fill(&mut salt)
                .map_err(|_| CommonError::HashingFailed)?;

            let mut hash = [0u8; CREDENTIAL_LEN];
            pbkdf2::derive(ALG, ITERATIONS, &salt, password.as_bytes(), &mut hash);

            let mut salt_hash = Vec::with_capacity(salt.len() + hash.len());
            salt_hash.extend_from_slice(&salt);
            salt_hash.extend_from_slice(&hash);

            Ok(STANDARD.encode(&salt_hash))
        })
        .await
        .unwrap_or_else(|_| Err(CommonError::InternalServerError)?)
    }

    pub async fn verify_password(&self, encoded: String, password: String) -> AppResult<bool> {
        let _permit = self.semaphore.acquire().await.unwrap();

        tokio::task::spawn_blocking(move || {
            let salt_and_hash = STANDARD
                .decode(encoded)
                .map_err(|_| CommonError::HashingFailed)?;

            let (salt, hash) = salt_and_hash.split_at(SALT_LEN);

            Ok(pbkdf2::verify(ALG, ITERATIONS, salt, password.as_bytes(), hash).is_ok())
        })
        .await
        .unwrap_or_else(|_| Err(CommonError::InternalServerError)?)
    }
}
