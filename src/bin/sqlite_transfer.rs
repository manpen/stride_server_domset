use std::{collections::HashSet, time::Instant};

use dotenv::dotenv;
use futures::TryStreamExt;
use sqlx::{
    migrate::MigrateDatabase, mysql::MySqlPoolOptions, sqlite::SqlitePoolOptions, MySqlPool,
    Sqlite, SqlitePool,
};

async fn connect_to_database() -> MySqlPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Connection to DB");

    println!("Connection to MySQL database is successful!");
    pool
}

async fn connect_to_sqlite(path: &str) -> SqlitePool {
    let already_exists = Sqlite::database_exists(path).await.unwrap_or(false);

    if !already_exists {
        println!("Creating database {}", path);
        Sqlite::create_database(path).await.expect("Create DB");
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(path)
        .await
        .expect("Connection to SQLite");

    println!("Connection to SQLite database {path} is successful!");

    if !already_exists {
        sqlx::query("CREATE TABLE InstanceData ( did INT PRIMARY KEY, data LONGBLOB);")
            .execute(&pool)
            .await
            .expect("Failed to create SQLite tables");
    }

    pool
}

#[derive(Debug, sqlx::FromRow)]
struct InputRow {
    did: i32,
    data: Option<Vec<u8>>,
}

async fn query_existing_rows(pool: &SqlitePool) -> HashSet<i32> {
    sqlx::query_scalar::<_, i32>("SELECT did FROM InstanceData")
        .fetch_all(pool)
        .await
        .expect("To fetch total instances")
        .into_iter()
        .collect()
}

async fn insert_row(pool: &SqlitePool, did: i32, data: &[u8]) {
    sqlx::query("INSERT INTO InstanceData (did, data) VALUES (?, ?)")
        .bind(did)
        .bind(data)
        .execute(pool)
        .await
        .expect("To insert row");
}

struct ProgressReport {
    rows: u64,
    start: Instant,
    previous_print: Instant,
    total_instances: u64,
}

impl ProgressReport {
    fn new(total_instances: u64) -> Self {
        let now = Instant::now();
        Self {
            rows: 0,
            start: now,
            previous_print: now,
            total_instances,
        }
    }

    fn update(&mut self) {
        self.rows += 1;
        let now = Instant::now();

        if now.duration_since(self.previous_print).as_millis() > 500 {
            let elapsed = now.duration_since(self.start).as_secs_f64();
            let throughput = self.rows as f64 / elapsed;
            let estimate = ((self.total_instances - self.rows) as f64) / throughput;

            println!("Processed {:>6} of {:>6} rows in {elapsed:.1}s ({throughput:.1} rows/s) ETA: {estimate:.1}s",
                self.rows, self.total_instances);
            self.previous_print = now;
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    dotenv().ok();

    let sqllite_small = connect_to_sqlite("sqlite://small_data.db").await;
    let sqllite_all = connect_to_sqlite("sqlite://all_data.db").await;
    let mysql = connect_to_database().await;

    let total_instances = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM InstanceData")
        .fetch_one(&mysql)
        .await
        .expect("To fetch total instances");

    println!("Total instances: {}", total_instances);

    let mut stream =
        sqlx::query_as::<_, InputRow>("SELECT `did`, `data` FROM InstanceData").fetch(&mysql);

    let existing_small = query_existing_rows(&sqllite_small).await;
    let existing_all = query_existing_rows(&sqllite_all).await;

    let mut progress_report = ProgressReport::new(total_instances as u64);
    while let Some(row) = stream.try_next().await.expect("To fetch row") {
        if let Some(data) = &row.data {
            progress_report.update();

            if !existing_all.contains(&row.did) {
                insert_row(&sqllite_all, row.did, data).await;
            }

            if data.len() < 10000 && !existing_small.contains(&row.did) {
                insert_row(&sqllite_small, row.did, data).await;
            }
        }
    }
}
