//! Exécutable serveur ; toutes les valeurs sensibles restent dans l'environnement.
use clap::{Parser, Subcommand};
use olc_api::{
    auth::Auth,
    profiles::Profiles,
    realtime::start_observer,
    server::{router, AppState},
};
use olc_collector::{
    config::ApiKey,
    riot_client::HttpsTransport,
    shared_quota::{CoordinatedTransport, Priority},
    storage::Storage,
};
use std::{net::SocketAddr, process::ExitCode, sync::Arc, time::Duration};

#[derive(Parser)]
#[command(about = "API interne REST et WebSocket d'Open LoL Companion")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand)]
enum Command {
    /// Lance le serveur ; HTTPS doit être terminé par le proxy en production.
    Serve {
        #[arg(long, env = "OLC_API_BIND", default_value = "127.0.0.1:3030")]
        bind: SocketAddr,
    },
    /// Émet un jeton de développement, à conserver comme un secret d'accès.
    Token {
        #[arg(long)]
        subject: String,
        #[arg(long, default_value_t = 3600)]
        ttl: u64,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let _ = dotenvy::dotenv();
    match run(Args::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
async fn run(args: Args) -> Result<(), String> {
    let secret = std::env::var("OLC_API_JWT_SECRET")
        .map_err(|_| "OLC_API_JWT_SECRET manquant (au moins 32 octets aléatoires)".to_owned())?;
    let issuer =
        std::env::var("OLC_API_JWT_ISSUER").unwrap_or_else(|_| "open-lol-companion".into());
    let audience = std::env::var("OLC_API_JWT_AUDIENCE").unwrap_or_else(|_| "olc-api".into());
    let auth = Auth::new(secret.as_bytes(), &issuer, &audience).map_err(str::to_owned)?;
    let command = match args.command {
        Some(command) => command,
        None => Command::Serve {
            bind: std::env::var("OLC_API_BIND")
                .unwrap_or_else(|_| "127.0.0.1:3030".into())
                .parse()
                .map_err(|_| "adresse invalide")?,
        },
    };
    match command {
        Command::Token { subject, ttl } => {
            println!(
                "{}",
                auth.issue(&subject, jsonwebtoken::get_current_timestamp(), ttl)
                    .map_err(str::to_owned)?
            );
            Ok(())
        }
        Command::Serve { bind } => {
            let database =
                std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL manquant".to_owned())?;
            let storage = Storage::connect(&database, 16)
                .await
                .map_err(|_| "connexion PostgreSQL impossible".to_owned())?;
            storage
                .migrate()
                .await
                .map_err(|_| "migrations PostgreSQL impossibles".to_owned())?;
            let mut state = AppState::new(storage.pool().clone(), auth);
            state.desktop_catalog_dir = std::env::var_os("OLC_DESKTOP_CATALOG_DIR").map(Into::into);
            let origins = std::env::var("OLC_API_ALLOWED_ORIGINS").unwrap_or_default();
            for origin in origins.split(',').filter(|s| !s.trim().is_empty()) {
                let origin = origin.trim();
                let uri: axum::http::Uri = origin.parse().map_err(|_| "origine CORS invalide")?;
                if !matches!(uri.scheme_str(), Some("http" | "https"))
                    || uri.authority().is_none()
                    || uri.path() != "/"
                    || uri.query().is_some()
                {
                    return Err("origine CORS invalide".into());
                }
                state
                    .allowed_origins
                    .push(origin.parse().map_err(|_| "origine CORS invalide")?);
            }
            // Sujets de jeton habilités à l'export et à l'effacement RGPD ; vide par défaut.
            state.privacy_operators = std::env::var("OLC_API_PRIVACY_OPERATORS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            if let Ok(key) = std::env::var("RIOT_API_KEY") {
                let key =
                    ApiKey::from_env_value(Some(key)).map_err(|_| "configuration Riot invalide")?;
                let transport = HttpsTransport::new(&key, Duration::from_secs(15))
                    .map_err(|_| "transport Riot indisponible")?
                    .with_max_response_bytes(8 * 1024 * 1024);
                // L'API sert des requêtes d'utilisateurs : elle garde la part du quota Riot
                // que le collecteur n'a pas le droit de consommer.
                state.profiles = Some(Arc::new(Profiles::new(CoordinatedTransport::new(
                    transport,
                    storage.clone(),
                    Priority::Interactive,
                ))));
            }
            let listener = tokio::net::TcpListener::bind(bind)
                .await
                .map_err(|_| "écoute HTTP impossible")?;
            let observer = start_observer(&state);
            let shutdown = state.shutdown.clone();
            eprintln!("API en écoute sur {bind}");
            let result = axum::serve(listener, router(state))
                .with_graceful_shutdown(async move {
                    shutdown_signal().await;
                    shutdown.send_replace(true);
                })
                .await
                .map_err(|_| "serveur HTTP interrompu".to_owned());
            observer.abort();
            storage.pool().close().await;
            result
        }
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    if let Ok(mut terminate) =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    {
        tokio::select! { _ = terminate.recv() => {}, _ = tokio::signal::ctrl_c() => {} }
        return;
    }
    // Windows : Ctrl+C ; repli Unix si l'enregistrement de SIGTERM échoue.
    let _ = tokio::signal::ctrl_c().await;
}
