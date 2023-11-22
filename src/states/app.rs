use crate::{
    cache::RedisCache,
    config::Database,
    repositories::{
        ChampionshipRepository, F123Repository, GoogleRepository, UserRepository,
        UserRepositoryTrait,
    },
    services::{
        ChampionshipService, EmailService, F123Service, FirewallService, SavedSessionService,
        TokenService, TokenServiceTrait, UserService, UserServiceTrait,
    },
};
use std::sync::Arc;

pub type AppState = Arc<AppStateInner>;

pub struct AppStateInner {
    pub user_service: UserService,
    pub user_repository: UserRepository,
    pub token_service: TokenService,
    pub championship_service: ChampionshipService,
    pub championship_repository: ChampionshipRepository,
    pub email_service: EmailService,
    pub f123_service: F123Service,
    pub f123_repository: F123Repository,
    pub saved_session_service: SavedSessionService,
    pub google_repository: GoogleRepository,
}

impl AppStateInner {
    pub async fn new(
        db_conn: &Arc<Database>,
        firewall_service: Arc<FirewallService>,
        cache: &Arc<RedisCache>,
    ) -> Self {
        Self {
            user_service: UserService::new(db_conn, cache),
            f123_service: F123Service::new(db_conn, firewall_service),
            f123_repository: F123Repository::new(db_conn),
            user_repository: UserRepository::new(db_conn, cache),
            token_service: TokenService::new(cache),
            championship_service: ChampionshipService::new(db_conn, cache).await,
            championship_repository: ChampionshipRepository::new(db_conn, cache).await,
            email_service: EmailService::new(),
            saved_session_service: SavedSessionService::new(db_conn, cache),
            google_repository: GoogleRepository::new(),
        }
    }
}