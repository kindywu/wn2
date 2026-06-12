use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub bearer_token: String,
}

impl AppState {
    pub fn new(db: PgPool, bearer_token: String) -> Self {
        Self { db, bearer_token }
    }
}
