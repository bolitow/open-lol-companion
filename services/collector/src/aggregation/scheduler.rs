use std::future::Future;
use std::time::Duration;

use tokio::time::{interval, MissedTickBehavior};

use super::AggregationError;

/// Exécute immédiatement puis chaque heure, sans chevauchement ; arrêt annulable.
pub async fn run_periodic<F, J, S>(mut job: F, shutdown: S) -> Result<(), AggregationError>
where
    F: FnMut() -> J,
    J: Future<Output = Result<(), AggregationError>>,
    S: Future<Output = ()>,
{
    let mut ticks = interval(Duration::from_secs(3600));
    ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            biased;
            _ = &mut shutdown => return Ok(()),
            _ = ticks.tick() => {}
        }
        tokio::select! {
            biased;
            _ = &mut shutdown => return Ok(()),
            result = job() => result?,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;
    use tokio::sync::{oneshot, Notify};

    use super::*;

    async fn checkpoint(
        notify: &Notify,
        task: &mut tokio::task::JoinHandle<Result<(), AggregationError>>,
    ) {
        tokio::select! {
            _ = notify.notified() => {}
            result = task => panic!("ordonnanceur terminé avant le calcul attendu : {result:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn calcule_immediatement_puis_a_l_heure_et_s_arrete_pendant_l_attente() {
        let calls = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(Notify::new());
        let counter = calls.clone();
        let notification = completed.clone();
        let (stop, stopped) = oneshot::channel();
        let mut task = tokio::spawn(run_periodic(
            move || {
                counter.fetch_add(1, Ordering::SeqCst);
                notification.notify_one();
                async { Ok(()) }
            },
            async {
                let _ = stopped.await;
            },
        ));
        checkpoint(&completed, &mut task).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        tokio::time::advance(Duration::from_secs(3599)).await;
        tokio::task::yield_now().await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        tokio::time::advance(Duration::from_secs(1)).await;
        checkpoint(&completed, &mut task).await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        stop.send(()).unwrap();
        task.await.unwrap().unwrap();
        tokio::time::advance(Duration::from_secs(7200)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn un_calcul_lent_ne_declenche_ni_chevauchement_ni_rafale() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let started = Arc::new(Notify::new());
        let notification = started.clone();
        let (stop, stopped) = oneshot::channel();
        let mut task = tokio::spawn(run_periodic(
            move || {
                let index = counter.fetch_add(1, Ordering::SeqCst);
                notification.notify_one();
                async move {
                    if index == 0 {
                        tokio::time::sleep(Duration::from_secs(9000)).await;
                    }
                    Ok(())
                }
            },
            async {
                let _ = stopped.await;
            },
        ));
        checkpoint(&started, &mut task).await;
        tokio::time::advance(Duration::from_secs(7200)).await;
        tokio::task::yield_now().await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        tokio::time::advance(Duration::from_secs(1800)).await;
        checkpoint(&started, &mut task).await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        tokio::task::yield_now().await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        tokio::time::advance(Duration::from_secs(1799)).await;
        tokio::task::yield_now().await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        tokio::time::advance(Duration::from_secs(1)).await;
        checkpoint(&started, &mut task).await;
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        stop.send(()).unwrap();
        task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn arret_annule_le_calcul_en_cours_et_propage_les_erreurs() {
        let started = Arc::new(Notify::new());
        let notification = started.clone();
        let (stop, stopped) = oneshot::channel();
        let mut task = tokio::spawn(run_periodic(
            move || {
                notification.notify_one();
                std::future::pending::<Result<(), AggregationError>>()
            },
            async {
                let _ = stopped.await;
            },
        ));
        checkpoint(&started, &mut task).await;
        stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let result = run_periodic(
            || async { Err(AggregationError::InvalidThreshold) },
            std::future::pending(),
        )
        .await;
        assert!(matches!(result, Err(AggregationError::InvalidThreshold)));
    }
}
