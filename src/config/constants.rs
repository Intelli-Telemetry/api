use std::{
    net::{IpAddr, Ipv4Addr},
    num::NonZeroU32,
    ops::Range,
    time::Duration,
};

use ring::pbkdf2;

// Auth
pub const BEARER_PREFIX: &str = "Bearer ";
pub const LOGIN_RATE_LIMIT: u8 = 5;
pub const LOGIN_RATE_LIMIT_DUR: Duration = Duration::from_secs(120);

// Discord auth
pub const DISCORD_API_URL: &str = "https://discord.com/api/v10";
pub const DISCORD_REDIRECT: &str = "https://intellitelemetry.live/auth/discord/callback";

// Email
pub const MAX_CONCURRENT_EMAILS: usize = 10;

// F1 Service
pub const BUFFER_SIZE: usize = 1460;
pub const SOCKET_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
pub const SOCKET_TIMEOUT: Duration = Duration::from_secs(15 * 60);
pub const GENERAL_INTERVAL: Duration = Duration::from_millis(700);
pub const TELEMETRY_INTERVAL: Duration = Duration::from_millis(100);

// Session
pub const HISTORY_INTERVAL: Duration = Duration::from_secs(1);
pub const SESSION_INTERVAL: Duration = Duration::from_secs(10);
pub const MOTION_INTERVAL: Duration = Duration::from_millis(700);

// Utils
// Ports Handler
pub const PORTS_RANGE: Range<i32> = 27700..27800;
// IdsGenerator
pub const IDS_POOL_SIZE: usize = 1024;
// Password Hasher
pub const PASS_SALT_LEN: usize = 16;
pub const PASS_CREDENTIAL_LEN: usize = 32;
pub const PASS_ITERATIONS: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(100_000) };
pub static PASS_ALG: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA256;
