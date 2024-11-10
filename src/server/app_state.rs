use sqlx::MySqlPool;

pub type DbPool = MySqlPool;
pub type DbTransaction<'a> = sqlx::Transaction<'a, sqlx::MySql>;

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
