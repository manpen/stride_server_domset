use sqlx::MySqlPool;

pub type DbPool = MySqlPool;

pub struct AppState {
    db: DbPool,
}

impl AppState {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &DbPool {
        &self.db
    }
}
