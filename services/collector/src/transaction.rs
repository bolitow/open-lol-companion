//! Connexion réservée pendant l'ouverture d'une transaction PostgreSQL.

use sqlx::pool::PoolConnection;
use sqlx::{Connection, PgPool, Postgres, Transaction};

pub(crate) struct TransactionConnection {
    connection: PoolConnection<Postgres>,
    begin_pending: bool,
}

impl TransactionConnection {
    pub(crate) async fn acquire(pool: &PgPool) -> Result<Self, sqlx::Error> {
        Ok(Self {
            connection: pool.acquire().await?,
            begin_pending: false,
        })
    }

    pub(crate) async fn begin(&mut self) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
        self.begin_with("BEGIN").await
    }

    async fn begin_with(
        &mut self,
        statement: &'static str,
    ) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
        if self.begin_pending {
            return Err(sqlx::Error::Protocol(
                "connexion invalidée par une ouverture de transaction interrompue".into(),
            ));
        }
        // SQLx 0.8.6 peut oublier le rollback si BEGIN est annulé pendant son
        // aller-retour : https://github.com/transact-rs/sqlx/pull/4394.
        // Le garde reste armé en cas d'erreur ou d'annulation. Il ne doit jamais
        // être réarmé sur cette même connexion après une ouverture incertaine.
        self.begin_pending = true;
        let transaction = self.connection.begin_with(statement).await?;
        self.begin_pending = false;
        Ok(transaction)
    }
}

impl Drop for TransactionConnection {
    fn drop(&mut self) {
        if self.begin_pending {
            self.connection.close_on_drop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::{PgConnection, PgPoolOptions};
    use std::time::Duration;

    #[tokio::test]
    async fn une_annulation_de_begin_ne_contamine_pas_le_pool() {
        let Ok(url) = std::env::var("OLC_TEST_DATABASE_URL") else {
            eprintln!("OLC_TEST_DATABASE_URL absente : test PostgreSQL ignoré");
            return;
        };
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let mut observer = PgConnection::connect(&url).await.unwrap();
        let mut connection = TransactionConnection::acquire(&pool).await.unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *connection.connection)
            .await
            .unwrap();
        {
            // Le sommeil élargit le même aller-retour que BEGIN ; on annule seulement
            // après avoir constaté côté serveur que ce message est en cours.
            let opening = connection.begin_with("BEGIN; SELECT pg_sleep(2);");
            tokio::pin!(opening);
            tokio::select! {
                _ = &mut opening => panic!("BEGIN est revenu avant l'annulation"),
                result = tokio::time::timeout(Duration::from_secs(5), async {
                    loop {
                        let sleeping: bool = sqlx::query_scalar(
                            "SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND wait_event='PgSleep')",
                        ).bind(pid).fetch_one(&mut observer).await.unwrap();
                        if sleeping { break; }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }) => result.unwrap(),
            }
        }
        assert!(matches!(
            connection.begin().await,
            Err(sqlx::Error::Protocol(_))
        ));
        drop(connection);
        // Une nouvelle requête doit s'exécuter hors de la transaction abandonnée.
        let mut next = tokio::time::timeout(Duration::from_secs(6), pool.acquire())
            .await
            .unwrap()
            .unwrap();
        let next_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *next)
            .await
            .unwrap();
        let state: String = sqlx::query_scalar("SELECT state FROM pg_stat_activity WHERE pid=$1")
            .bind(next_pid)
            .fetch_one(&mut observer)
            .await
            .unwrap();
        assert_eq!(state, "idle");
        drop(next);
        pool.close().await;
        observer.close().await.unwrap();
    }

    #[tokio::test]
    async fn une_transaction_terminee_conserve_la_connexion_du_pool() {
        let Ok(url) = std::env::var("OLC_TEST_DATABASE_URL") else {
            eprintln!("OLC_TEST_DATABASE_URL absente : test PostgreSQL ignoré");
            return;
        };
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let mut connection = TransactionConnection::acquire(&pool).await.unwrap();
        let mut transaction = connection.begin().await.unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *transaction)
            .await
            .unwrap();
        transaction.commit().await.unwrap();
        drop(connection);
        let next_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(pid, next_pid);
        pool.close().await;
    }
}
