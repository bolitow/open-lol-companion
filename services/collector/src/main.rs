//! Ligne de commande du collecteur. Voir `services/collector/README.md`.

use std::process::ExitCode;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};
use olc_collector::aggregation::{self, AggregationError, AggregationOptions};
use olc_collector::campaign;
use olc_collector::catalog::{self, CommunityPolicy};
use olc_collector::collector::{now_ms, Collector, RunOutcome, StopReason};
use olc_collector::config::{database_url, ApiKey, Division, RunParams, RuntimeOptions, Tier};
use olc_collector::report;
use olc_collector::riot_client::HttpsTransport;
use olc_collector::shared_quota::CoordinatedTransport;
use olc_collector::static_data;
use olc_collector::storage::{RunStatus, Storage};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "olc-collector",
    version,
    about = "Collecte Riot multirégion et agrégation par patch, file, rôle et rang"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Normalise les catalogues en cache et archive les sources exactes (#61).
    Catalog {
        /// Version Data Dragon ; sinon les versions du manifeste en cache.
        #[arg(long, conflicts_with = "rebuild")]
        version: Option<String>,
        #[arg(long, default_value_t=2, value_parser=clap::value_parser!(u8).range(1..=10))]
        patch_count: u8,
        #[arg(long, value_enum, default_value = "required")]
        community: CommunityPolicy,
        /// Revérifie le complément public même s'il est archivé pour ce patch.
        #[arg(long)]
        refresh: bool,
        /// Reconstruit une publication depuis ses sources archivées, sans réseau.
        #[arg(long)]
        rebuild: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Applique les migrations PostgreSQL.
    Migrate,
    /// Synchronise les données publiques FR/EN des derniers patches Data Dragon.
    SyncStatic {
        #[arg(long, default_value_t=2, value_parser=clap::value_parser!(u8).range(1..=10))]
        patch_count: u8,
        /// Retélécharge aussi les versions déjà en cache.
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        watch: bool,
        #[arg(long)]
        json: bool,
    },
    /// Recalcule les statistiques des patches sélectionnés (#18).
    Aggregate {
        /// Patches techniques explicites ; sinon les deux patches du cache Data Dragon.
        #[arg(long, value_delimiter = ',', conflicts_with = "all_stored")]
        patches: Vec<String>,
        /// Agrège tous les patches stockés, sans résolution Data Dragon.
        #[arg(long)]
        all_stored: bool,
        #[arg(long, value_delimiter = ',')]
        platforms: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        queues: Vec<i32>,
        /// Borne UTC inclusive, en millisecondes Unix.
        #[arg(long)]
        from_ms: Option<i64>,
        /// Borne UTC exclusive, en millisecondes Unix.
        #[arg(long)]
        to_ms: Option<i64>,
        /// Synchronise les statiques avant chaque calcul (immédiat puis horaire avec --watch).
        #[arg(long)]
        sync_static: bool,
        /// Parties minimales par champion/rôle/patch pour publier taux et position.
        #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..))]
        min_games: u32,
        /// Écart maximal, en heures, entre le début d'une partie et l'observation de rang retenue.
        #[arg(
            long,
            default_value_t = aggregation::DEFAULT_RANK_MAX_AGE_HOURS,
            value_parser = clap::value_parser!(u32).range(1..=i64::from(aggregation::MAX_RANK_MAX_AGE_HOURS))
        )]
        rank_max_age_hours: u32,
        /// Durée minimale, en secondes, d'une partie classée (420/440) ; 0 désactive le contrôle (#111).
        #[arg(
            long,
            default_value_t = aggregation::DEFAULT_MIN_GAME_DURATION_S,
            value_parser = clap::value_parser!(u32).range(0..=i64::from(aggregation::MAX_MIN_GAME_DURATION_S))
        )]
        min_game_duration_s: u32,
        /// Part minimale (%) de la durée jouée par chaque participant d'une partie classée ; 0 désactive (#111).
        #[arg(
            long,
            default_value_t = aggregation::DEFAULT_MIN_PLAYED_PERCENT,
            value_parser = clap::value_parser!(u32).range(0..=100)
        )]
        min_played_percent: u32,
        /// Conserve les parties classées avec un participant `wasAfk` (exclues par défaut) (#111).
        #[arg(long)]
        keep_afk: bool,
        /// Recalcule immédiatement puis chaque heure (Ctrl+C pour arrêter).
        #[arg(long)]
        watch: bool,
        /// Rapport JSON complet ; une ligne par publication en mode continu.
        #[arg(long)]
        json: bool,
    },
    /// Collecte toutes les files sur les plateformes choisies, par tranches reprenables de 15 min.
    Campaign {
        #[arg(
            long,
            value_delimiter = ',',
            default_value = "EUW1,NA1,KR,OC1,EUN1,BR1,JP1,TW2,TR1,LA1,VN2,ME1,LA2,RU,SG2"
        )]
        platforms: Vec<String>,
        #[arg(long,default_value_t=24,value_parser=clap::value_parser!(u8).range(1..=24))]
        hours: u8,
        #[arg(long, default_value_t = 10000)]
        target_per_platform: u32,
        #[arg(long, value_delimiter = ',')]
        patches: Vec<String>,
        #[arg(long, default_value_t = 5)]
        seeds_per_division: u32,
        #[arg(long, default_value_t = 100)]
        max_matches_per_seed: u32,
        #[arg(long, default_value_t = 100000)]
        call_budget_per_platform: u64,
        #[arg(long, default_value_t = 4)]
        concurrency: usize,
    },
    /// Reprend la campagne avec ses fenêtres, patches et échéance d'origine.
    CampaignResume {
        campaign_id: i64,
        #[command(flatten)]
        runtime: RuntimeArgs,
    },
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
    #[arg(long, default_value = "EUW1")]
    platform: String,
    /// Identifiant de file ; 0 découvre toutes les files de l'historique.
    #[arg(long, default_value_t = 420)]
    queue: i32,
    /// Patches techniques explicites ; sinon les deux patches du cache Data Dragon.
    #[arg(long, value_delimiter = ',', conflicts_with = "all_patches")]
    patches: Vec<String>,
    #[arg(long)]
    all_patches: bool,
    /// Observe le classement Solo/Flex de chaque participant (cache de 24 h).
    #[arg(long)]
    collect_ranks: bool,
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
            max_duration: self
                .max_duration_mins
                .map(|m| Duration::from_secs(m.saturating_mul(60))),
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
        Command::Catalog {
            version,
            patch_count,
            community,
            refresh,
            rebuild,
            json,
        } => {
            let storage = connect(&db_url, 4).await?;
            let work = async {
                let mut manifests = Vec::new();
                if let Some(publication) = rebuild {
                    manifests.push(catalog::rebuild(&storage, &publication).await?);
                } else {
                    let versions = if let Some(version) = version {
                        vec![version]
                    } else {
                        let available: Option<serde_json::Value> = sqlx::query_scalar(
                            "SELECT versions FROM static_data_manifest WHERE id=1",
                        )
                        .fetch_optional(storage.pool())
                        .await?;
                        let available = available.ok_or(catalog::CatalogError::NotFound)?;
                        available
                            .as_array()
                            .ok_or(catalog::CatalogError::InvalidSource)?
                            .iter()
                            .take(usize::from(patch_count))
                            .map(|v| {
                                v.as_str()
                                    .map(str::to_owned)
                                    .ok_or(catalog::CatalogError::InvalidSource)
                            })
                            .collect::<Result<Vec<_>, _>>()?
                    };
                    if versions.is_empty() {
                        return Err(catalog::CatalogError::NotFound);
                    }
                    for version in versions {
                        manifests
                            .push(catalog::build(&storage, &version, community, refresh).await?);
                    }
                }
                Ok::<_, catalog::CatalogError>(manifests)
            };
            let manifests = tokio::select! { _=shutdown()=>return Ok(ExitCode::from(3)),result=work=>result.map_err(|e|e.to_string())? };
            if json {
                println!(
                    "{}",
                    serde_json::to_string(&manifests).map_err(|e| e.to_string())?
                );
            } else {
                for manifest in manifests {
                    println!(
                        "Catalogue {} : {} fiches, {} champs non normalisés, publication {}{}.",
                        manifest.version,
                        manifest.coverage.records,
                        manifest.coverage.unmapped_fields,
                        manifest.publication_id,
                        if manifest.degraded {
                            " (couverture dégradée)"
                        } else {
                            ""
                        }
                    );
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Aggregate {
            min_games,
            rank_max_age_hours,
            min_game_duration_s,
            min_played_percent,
            keep_afk,
            watch,
            json,
            patches,
            all_stored,
            platforms,
            queues,
            from_ms,
            to_ms,
            sync_static,
        } => {
            aggregate(
                &db_url,
                min_games,
                rank_max_age_hours,
                aggregation::QualityThresholds {
                    min_game_duration_s,
                    min_played_percent,
                    exclude_afk: !keep_afk,
                },
                watch,
                json,
                AggregationOptions {
                    patches,
                    platforms: platforms
                        .into_iter()
                        .map(|p| p.to_ascii_uppercase())
                        .collect(),
                    queues,
                    start_ms: from_ms,
                    end_ms: to_ms,
                },
                all_stored,
                sync_static,
            )
            .await
        }
        Command::SyncStatic {
            patch_count,
            refresh,
            watch,
            json,
        } => {
            let storage = connect(&db_url, 2).await?;
            let sync = || async {
                let result =
                    static_data::sync_recent_refresh(&storage, usize::from(patch_count), refresh)
                        .await?;
                if json {
                    println!("{}", serde_json::to_string(&result)?);
                } else {
                    println!("Données statiques publiées : {} (FR/EN, champions et compétences, items, runes, sorts, cartes et catalogues).",result.releases.iter().map(|r|r.version.as_str()).collect::<Vec<_>>().join(", "));
                }
                Ok::<(), AggregationError>(())
            };
            if watch {
                aggregation::run_periodic(sync, shutdown())
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                tokio::select! { _=shutdown()=>return Ok(ExitCode::from(3)),r=sync()=>r.map_err(|e|e.to_string())? }
            }
            Ok(ExitCode::SUCCESS)
        }
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
        Command::Campaign {
            platforms,
            hours,
            target_per_platform,
            patches,
            seeds_per_division,
            max_matches_per_seed,
            call_budget_per_platform,
            concurrency,
        } => {
            let storage = connect(&db_url, concurrency.clamp(1, 16) as u32 + 3).await?;
            let patches = if patches.is_empty() {
                static_data::cached_patches(&storage, 2)
                    .await
                    .map_err(|e| e.to_string())?
            } else {
                patches
            };
            let template = RunParams {
                queue_id: 0,
                patches,
                collect_ranks: true,
                target_matches: target_per_platform,
                tiers: vec![
                    Tier::Iron,
                    Tier::Bronze,
                    Tier::Silver,
                    Tier::Gold,
                    Tier::Platinum,
                    Tier::Emerald,
                    Tier::Diamond,
                    Tier::Master,
                    Tier::Grandmaster,
                    Tier::Challenger,
                ],
                window_days: 28,
                seeds_per_division,
                max_matches_per_seed,
                call_budget: call_budget_per_platform,
                ..RunParams::default()
            };
            template.validate().map_err(|e| e.to_string())?;
            let api_key = ApiKey::from_env_value(std::env::var("RIOT_API_KEY").ok())
                .map_err(|e| e.to_string())?;
            let id = campaign::start(
                &storage,
                &platforms,
                &template,
                now_ms(),
                Duration::from_secs(u64::from(hours) * 3600),
            )
            .await
            .map_err(|e| e.to_string())?;
            drive_campaign(
                &storage,
                &api_key,
                id,
                RuntimeOptions {
                    concurrency: concurrency.clamp(1, 16),
                    ..RuntimeOptions::default()
                },
            )
            .await
        }
        Command::CampaignResume {
            campaign_id,
            runtime,
        } => {
            let api_key = ApiKey::from_env_value(std::env::var("RIOT_API_KEY").ok())
                .map_err(|e| e.to_string())?;
            let storage = connect(&db_url, runtime.options().concurrency as u32 + 3).await?;
            drive_campaign(&storage, &api_key, campaign_id, runtime.options()).await
        }
        Command::Run { params, runtime } => {
            let patches = if params.patches.is_empty() && !params.all_patches {
                let storage = connect(&db_url, 2).await?;
                static_data::cached_patches(&storage, 2)
                    .await
                    .map_err(|e| e.to_string())?
            } else {
                params.patches
            };
            let params = RunParams {
                platform_id: params.platform.to_ascii_uppercase(),
                queue_id: params.queue,
                patches,
                collect_ranks: params.collect_ranks,
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

// Un paramètre par option CLI de `aggregate` ; les regrouper n'apporterait aucune règle.
#[allow(clippy::too_many_arguments)]
async fn aggregate(
    db_url: &str,
    min_games: u32,
    rank_max_age_hours: u32,
    quality: aggregation::QualityThresholds,
    watch: bool,
    json: bool,
    filters: AggregationOptions,
    all_stored: bool,
    sync_static: bool,
) -> Result<ExitCode, String> {
    let storage = connect(db_url, 2)
        .await
        .map_err(|_| "connexion ou migrations PostgreSQL impossibles".to_owned())?;
    let calculate = || async {
        if sync_static {
            static_data::sync_recent(&storage, 2).await?;
        }
        let mut selected = filters.clone();
        if selected.patches.is_empty() && !all_stored {
            selected.patches = static_data::cached_patches(&storage, 2).await?;
        }
        let report = aggregation::recalculate_with_quality(
            &storage,
            min_games,
            rank_max_age_hours,
            &selected,
            &quality,
        )
        .await?;
        if json {
            println!("{}", serde_json::to_string(&report)?);
        } else {
            let eligible = report
                .groups
                .iter()
                .filter(|g| g.position.is_some())
                .count();
            println!("Agrégats publiés : {} / {} parties retenues, {} groupes dont {} classés (seuil {}). Rangs observés séparés.",
                report.included_matches, report.source_matches, report.groups.len(), eligible, report.min_games);
            for (reason, count) in &report.exclusions {
                println!("  Exclusions {reason} : {count}");
            }
        }
        Ok::<(), AggregationError>(())
    };
    let shutdown = async {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };
    if watch {
        aggregation::run_periodic(calculate, shutdown)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        tokio::select! {
            biased;
            _ = shutdown => return Ok(ExitCode::from(3)),
            result = calculate() => result.map_err(|e| e.to_string())?,
        }
    }
    Ok(ExitCode::SUCCESS)
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
        .map_err(|_| "connexion PostgreSQL impossible".to_owned())?;
    storage
        .migrate()
        .await
        .map_err(|_| "migrations PostgreSQL impossibles".to_owned())?;
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
    let transport = CoordinatedTransport::new(transport, storage.clone());
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

async fn shutdown() {
    if tokio::signal::ctrl_c().await.is_err() {
        std::future::pending::<()>().await;
    }
}

async fn drive_campaign(
    storage: &Storage,
    key: &ApiKey,
    id: i64,
    options: RuntimeOptions,
) -> Result<ExitCode, String> {
    let transport = HttpsTransport::new(key, Duration::from_secs(15)).map_err(|e| e.to_string())?;
    let transport = CoordinatedTransport::new(transport, storage.clone());
    eprintln!("Campagne #{id} en cours ; reprise : olc-collector campaign-resume {id}.");
    let outcome = campaign::execute(storage, transport, options, id, shutdown())
        .await
        .map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string(&outcome)
            .map_err(|_| "bilan de campagne non sérialisable".to_owned())?
    );
    Ok(if outcome.status == campaign::CampaignStatus::Finished {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(3)
    })
}
