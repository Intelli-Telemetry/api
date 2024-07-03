use chrono::{Duration, Local, TimeDelta};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

//* Token Type Enum
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub enum TokenType {
    Bearer,
    Email,
    ResetPassword,
    RefreshBearer,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenClaim {
    pub exp: usize,
    pub sub: i32,
    pub token_type: TokenType,
}

// Token Type Implementation
impl TokenType {
    pub fn expiry(&self) -> Instant {
        Instant::now() + self.minutes().unwrap().to_std().unwrap()
    }

    pub fn set_expiration(&self) -> usize {
        let minutes = self.minutes().unwrap();

        Local::now()
            .checked_add_signed(minutes)
            .unwrap()
            .timestamp() as usize
    }

    const fn minutes(&self) -> Option<TimeDelta> {
        match self {
            TokenType::RefreshBearer => Duration::try_days(17),
            TokenType::Bearer => Duration::try_days(1),
            _ => Duration::try_minutes(15),
        }
    }
}
