//! Ligne de commande du collecteur. Voir `services/collector/README.md`.

use std::process::ExitCode;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};
use olc_collector::collector::{now_ms, Collector, RunOutcome, StopReason};
use olc_collector::config::{database_url, ApiKey, Division, RunParams, RuntimeOptions, Tier};
use olc_collector::report;
use olc_collector::riot_client::HttpsTransport;
use olc_collector::storage::{RunStatus, Storage};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "olc-collector",
    version,
    about = "Collecte de parties Ranked Solo/Duo EUW via l'API Riot"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Applique les migrations PostgreSQL.
    Migrate,
    /// Lance une nouvelle exécution.
    Run {
        #[command(flatten)]
        params: ParamsArgs,
        #[command(flatten)]
        runtime: RuntimeArgs,
    },
    /// Reprend une exécution arrêtée (fenêtre et paramètres d'origine).
    Resume {
        run_id: i64,
        /// Nouveau budget total d'appels (pour continuer après épuisement).
        #[arg(long)]
        call_budget: Option<u64>,
        /// Relance aussi les travaux en échec (après une panne réseau, par exemple).
        #[arg(long)]
        retry_failed: bool,
        #[command(flatten)]
        runtime: RuntimeArgs,
    },
    /// Affiche le bilan d'une exécution.
    Report {
        run_id: i64,
        /// Bilan au format JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Args)]
struct ParamsArgs {
    /// Nombre de parties distinctes à retenir.
    #[arg(long, default_value_t = 1000)]
    target: u32,
    /// Rangs de départ, séparés par des virgules.
    #[arg(long, value_delimiter = ',', default_value = "GOLD,PLATINUM,EMERALD")]
    tiers: Vec<Tier>,
    /// Divisions de départ, séparées par des virgules.
    #[arg(long, value_delimiter = ',', default_value = "I,II,III,IV")]
    divisions: Vec<Division>,
    /// Fenêtre de collecte en jours, figée au lancement.
    #[arg(long, default_value_t = 14)]
    window_days: u32,
    /// Joueurs de départ par rang et division.
    #[arg(long, default_value_t = 15)]
    seeds_per_division: u32,
    /// Parties découvertes au maximum par joueur de départ.
    #[arg(long, default_value_t = 10)]
    max_matches_per_seed: u32,
    /// Nombre maximal d'appels Riot pour l'exécution.
    #[arg(long, default_value_t = 3000)]
    call_budget: u64,
}

#[derive(Args)]
struct RuntimeArgs {
    /// Requêtes Riot simultanées.
    #[arg(long, default_value_t = 2)]
    concurrency: usize,
    /// Durée maximale de ce lancement, en minutes (reprise possible ensuite).
    #[arg(long)]
    max_duration_mins: Option<u64>,
}

impl RuntimeArgs {
    fn options(&self) -> RuntimeOptions {
        RuntimeOptions {
            concurrency: self.concurrency.clamp(1, 16),
            max_duration: self.max_duration_mins.map(|m| Duration::from_secs(m * 60)),
            ..RuntimeOptions::default()
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    // Un fichier .env est facultatif ; les variables d'environnement priment.
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    match run(Cli::parse()).await {
        Ok(code) => code,
        Err(message) => {
            eprintln!("Erreur : {message}");
            ExitCode::from(1)
        }
    }
}

async fn run(cli: Cli) -> Result<ExitCode, String> {
    let db_url = database_url(std::env::var("DATABASE_URL").ok()).map_err(|e| e.to_string())?;
    match cli.command {
        Command::Migrate => {
            let storage = connect(&db_url, 2).await?;
            println!("Migrations appliquées.");
            drop(storage);
            Ok(ExitCode::SUCCESS)
        }
        Command::Report { run_id, json } => {
            let storage = connect(&db_url, 2).await?;
            let report = report::generate(&storage, run_id)
                .await
                .map_err(|e| e.to_string())?;
            if json {
                let text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
                println!("{text}");
            } else {
                print!("{}", report.render());
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Run { params, runtime } => {
            let params = RunParams {
                target_matches: params.target,
                tiers: params.tiers,
                divisions: params.divisions,
                window_days: params.window_days,
                seeds_per_division: params.seeds_per_division,
                max_matches_per_seed: params.max_matches_per_seed,
                call_budget: params.call_budget,
            };
            params.validate().map_err(|e| e.to_string())?;
            collect(&db_url, runtime.options(), Start::New(params)).await
        }
        Command::Resume {
            run_id,
            call_budget,
            retry_failed,
            runtime,
        } => {
            collect(
                &db_url,
                runtime.options(),
                Start::Resume {
                    run_id,
                    call_budget,
                    retry_failed,
                },
            )
            .await
        }
    }
}

enum Start {
    New(RunParams),
    Resume {
        run_id: i64,
        call_budget: Option<u64>,
        retry_failed: bool,
    },
}

async fn connect(db_url: &str, max_connections: u32) -> Result<Storage, String> {
    let storage = Storage::connect(db_url, max_connections)
        .await
        .map_err(|e| e.to_string())?;
    storage.migrate().await.map_err(|e| e.to_string())?;
    Ok(storage)
}

async fn collect(db_url: &str, options: RuntimeOptions, start: Start) -> Result<ExitCode, String> {
    // La clé est vérifiée avant toute connexion : erreur claire, sans la valeur.
    let api_key =
        ApiKey::from_env_value(std::env::var("RIOT_API_KEY").ok()).map_err(|e| e.to_string())?;
    let storage = connect(db_url, options.concurrency as u32 + 3).await?;
    let _lock = storage.lock_collector().await.map_err(|e| e.to_string())?;
    let transport =
        HttpsTransport::new(&api_key, Duration::from_secs(15)).map_err(|e| e.to_string())?;
    let collector = Collector::new(storage.clone(), transport, options);

    let run_id = match start {
        Start::New(params) => collector
            .start_run(&params, now_ms())
            .await
            .map_err(|e| e.to_string())?,
        Start::Resume {
            run_id,
            call_budget,
            retry_failed,
        } => {
            if let Some(budget) = call_budget {
                storage
                    .set_call_budget(run_id, budget)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            if retry_failed {
                let n = storage
                    .requeue_failed(run_id)
                    .await
                    .map_err(|e| e.to_string())?;
                eprintln!("{n} travaux en échec remis en attente.");
            }
            run_id
        }
    };
    eprintln!("Exécution #{run_id} en cours (Ctrl+C pour arrêter proprement).");

    let shutdown = async {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };
    let outcome = collector
        .execute(run_id, shutdown)
        .await
        .map_err(|e| format!("{e} (reprise possible : olc-collector resume {run_id})"))?;

    let report = report::generate(&storage, run_id)
        .await
        .map_err(|e| e.to_string())?;
    if let Ok(json) = serde_json::to_value(&report) {
        storage
            .save_report(run_id, &json)
            .await
            .map_err(|e| e.to_string())?;
    }
    print!("{}", report.render());
    explain(&outcome, report.failed_jobs);
    Ok(match outcome.status {
        RunStatus::Completed => ExitCode::SUCCESS,
        RunStatus::Incomplete => ExitCode::from(2),
        RunStatus::Paused | RunStatus::Running => ExitCode::from(3),
    })
}

fn explain(outcome: &RunOutcome, failed_jobs: i64) {
    let id = outcome.run_id;
    if failed_jobs > 0 {
        eprintln!(
            "\n{failed_jobs} travaux en échec (voir « Erreurs principales »). Une fois la cause corrigée : \
             olc-collector resume {id} --retry-failed"
        );
    }
    match outcome.reason {
        StopReason::Finished if outcome.status == RunStatus::Incomplete && failed_jobs == 0 => eprintln!(
            "\nCollecte terminée sous la cible ({} / {}) : joueurs de départ ou fenêtre épuisés. \
             Relancez avec plus de joueurs (--seeds-per-division) ou une autre fenêtre.",
            outcome.retained, outcome.target
        ),
        StopReason::Finished if outcome.status == RunStatus::Incomplete => eprintln!(
            "\nCollecte terminée sous la cible ({} / {}).",
            outcome.retained, outcome.target
        ),
        StopReason::Finished => {}
        StopReason::AuthRejected(status) => eprintln!(
            "\nRiot a refusé la clé (HTTP {status}). La clé de développement expire après 24 h : \
             mettez à jour RIOT_API_KEY puis lancez « olc-collector resume {id} »."
        ),
        StopReason::CallBudget => eprintln!(
            "\nBudget d'appels épuisé. Pour continuer : olc-collector resume {id} --call-budget <nouveau total>"
        ),
        StopReason::MaxDuration | StopReason::Interrupted => {
            eprintln!("\nCollecte arrêtée. Pour reprendre : olc-collector resume {id}")
        }
    }
}
