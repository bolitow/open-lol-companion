//! Base PostgreSQL jetable pour les tests de l'API, sans appel réseau externe.
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};

use olc_collector::storage::Storage;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, Executor};

pub struct TestDb {
    pub storage: Storage,
    admin: PgConnectOptions,
    name: String,
}

impl TestDb {
    /// Une configuration absente ignore le test ; une connexion invalide le fait échouer.
    pub async fn create() -> Option<Self> {
        let Ok(url) = std::env::var("OLC_TEST_DATABASE_URL") else {
            eprintln!("OLC_TEST_DATABASE_URL absente : test PostgreSQL ignoré");
            return None;
        };
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let name = format!(
            "olc_api_test_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let admin = PgConnectOptions::from_str(&url).expect("OLC_TEST_DATABASE_URL invalide");
        let mut connection = admin.connect().await.expect("connexion PostgreSQL");
        connection
            .execute(format!("CREATE DATABASE {name}").as_str())
            .await
            .unwrap();
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect_with(admin.clone().database(&name))
            .await
            .unwrap();
        let storage = Storage::from_pool(pool);
        storage.migrate().await.unwrap();
        Some(Self {
            storage,
            admin,
            name,
        })
    }

    pub async fn cleanup(self) {
        self.storage.pool().close().await;
        let mut connection = self.admin.connect().await.expect("connexion de nettoyage");
        connection
            .execute(format!("DROP DATABASE {} WITH (FORCE)", self.name).as_str())
            .await
            .unwrap();
    }
}
