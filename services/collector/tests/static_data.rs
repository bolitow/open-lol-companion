//! Cache statique sur PostgreSQL jetable et CDN synthétique, sans appel externe.
mod common;

use common::TestDb;
use olc_collector::static_data::test_support::FakeCdn;
use olc_collector::static_data::{cached_patches, sync_with_transport, StaticError};
use olc_collector::static_data::{StaticResponse, StaticTransport};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Notify;

macro_rules! db_or_skip {
    () => {
        match TestDb::create().await {
            Some(db) => db,
            None => return,
        }
    };
}

async fn manifest(db: &TestDb) -> Value {
    sqlx::query_scalar("SELECT to_jsonb(m) FROM static_data_manifest m WHERE id=1")
        .fetch_one(db.storage.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn statique_publie_deux_versions_completes_et_reutilise_les_bundles_sans_fausse_date() {
    let db = db_or_skip!();
    let fake = FakeCdn::new();
    let first = sync_with_transport(&db.storage, 2, fake.clone(), false)
        .await
        .unwrap();
    assert_eq!(first.live_version, "16.19.1");
    assert_eq!(
        first
            .releases
            .iter()
            .map(|r| r.version.as_str())
            .collect::<Vec<_>>(),
        ["16.19.1", "16.18.1"]
    );
    assert!(first
        .releases
        .iter()
        .all(|r| r.documents == 18 && r.champions == 1 && r.classic_champions == 1 && !r.reused));
    assert_eq!(
        cached_patches(&db.storage, 2).await.unwrap(),
        ["16.19", "16.18"]
    );
    assert_eq!(
        db.scalar("SELECT count(*) FROM static_data_releases").await,
        2
    );
    assert_eq!(
        manifest(&db).await["catalogs"]["queues"]["data"][0]["queueId"],
        420
    );
    fake.deny("/cdn/");
    let second = sync_with_transport(&db.storage, 2, fake, false)
        .await
        .unwrap();
    assert!(second.releases.iter().all(|r| r.reused));
    assert_eq!(
        first.releases[0].completed_at,
        second.releases[0].completed_at
    );
    assert_eq!(
        first.releases[1].completed_at,
        second.releases[1].completed_at
    );
    db.cleanup().await;
}

#[tokio::test]
async fn statique_conserve_le_manifeste_et_tous_les_bundles_si_une_langue_echoue() {
    let db = db_or_skip!();
    let fake = FakeCdn::new();
    sync_with_transport(&db.storage, 2, fake.clone(), false)
        .await
        .unwrap();
    let before = manifest(&db).await;
    fake.versions(&["16.20.1", "16.19.1", "16.18.1"], "16.20.1");
    fake.deny("16.20.1/data/fr_FR/champion/Aatrox.json");
    assert!(matches!(
        sync_with_transport(&db.storage, 2, fake, false).await,
        Err(StaticError::Network)
    ));
    assert_eq!(manifest(&db).await, before);
    assert_eq!(
        db.scalar("SELECT count(*) FROM static_data_releases").await,
        2
    );
    assert_eq!(
        cached_patches(&db.storage, 2).await.unwrap(),
        ["16.19", "16.18"]
    );
    db.cleanup().await;
}

#[tokio::test]
async fn statique_rollback_sur_erreur_de_publication_et_cache_invalide_refuse() {
    let db = db_or_skip!();
    let fake = FakeCdn::new();
    sync_with_transport(&db.storage, 2, fake.clone(), false)
        .await
        .unwrap();
    let before = manifest(&db).await;
    sqlx::raw_sql("CREATE FUNCTION reject_static() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'private detail'; END $$;
        CREATE CONSTRAINT TRIGGER reject_static AFTER UPDATE ON static_data_manifest DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reject_static();")
        .execute(db.storage.pool()).await.unwrap();
    fake.versions(&["16.20.1", "16.19.1", "16.18.1"], "16.20.1");
    let error = sync_with_transport(&db.storage, 2, fake, false)
        .await
        .unwrap_err();
    assert_eq!(error, StaticError::Database);
    assert!(!error.to_string().contains("private detail"));
    assert_eq!(manifest(&db).await, before);
    assert_eq!(
        db.scalar("SELECT count(*) FROM static_data_releases").await,
        2
    );
    sqlx::query("UPDATE static_data_releases SET bundle = '{}' WHERE version='16.19.1'")
        .execute(db.storage.pool())
        .await
        .unwrap();
    assert!(cached_patches(&db.storage, 2).await.is_err());
    db.cleanup().await;
}

#[tokio::test]
async fn statique_refuse_cache_absent_et_seuil_invalide() {
    let db = db_or_skip!();
    assert_eq!(
        cached_patches(&db.storage, 2).await,
        Err(StaticError::CacheMissing)
    );
    assert!(matches!(
        sync_with_transport(&db.storage, 0, FakeCdn::new(), false).await,
        Err(StaticError::InvalidCount)
    ));
    db.cleanup().await;
}

#[derive(Clone)]
struct BlockedCdn {
    started: Arc<Notify>,
}

impl StaticTransport for BlockedCdn {
    async fn get(&self, _url: &str) -> Result<StaticResponse, StaticError> {
        self.started.notify_one();
        std::future::pending().await
    }
}

#[tokio::test]
async fn statique_refuse_deux_synchronisations_et_libere_le_verrou_apres_annulation() {
    let db = db_or_skip!();
    sync_with_transport(&db.storage, 2, FakeCdn::new(), false)
        .await
        .unwrap();
    let before = manifest(&db).await;
    let other = db.separate_storage().await;
    let writer = other.clone();
    let started = Arc::new(Notify::new());
    let transport = BlockedCdn {
        started: started.clone(),
    };
    let task = tokio::spawn(async move { sync_with_transport(&writer, 2, transport, false).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), started.notified())
        .await
        .unwrap();
    assert!(matches!(
        sync_with_transport(&db.storage, 2, FakeCdn::new(), false).await,
        Err(StaticError::Busy)
    ));
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    tokio::time::timeout(std::time::Duration::from_secs(5), other.pool().close())
        .await
        .unwrap();
    assert_eq!(manifest(&db).await, before);
    assert!(sync_with_transport(&db.storage, 2, FakeCdn::new(), false)
        .await
        .is_ok());
    db.cleanup().await;
}

#[tokio::test]
async fn statique_refresh_explicite_retelecharge_et_echec_conserve_ancien_cache() {
    let db = db_or_skip!();
    sync_with_transport(&db.storage, 2, FakeCdn::new(), false)
        .await
        .unwrap();
    let before = manifest(&db).await;
    let broken = FakeCdn::new();
    broken.deny("/cdn/");
    assert!(sync_with_transport(&db.storage, 2, broken, true)
        .await
        .is_err());
    assert_eq!(manifest(&db).await, before);
    let refreshed = sync_with_transport(&db.storage, 2, FakeCdn::new(), true)
        .await
        .unwrap();
    assert!(refreshed.releases.iter().all(|r| !r.reused));
    assert_eq!(
        db.scalar("SELECT count(*) FROM static_data_releases").await,
        2
    );
    db.cleanup().await;
}
