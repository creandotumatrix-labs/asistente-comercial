mod agent;
mod calendar;
mod config;
mod crm;
mod error;
mod notify;
mod scoring;
mod server;
mod state;
mod store;
mod whatsapp;

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tracing_subscriber::EnvFilter;

use crate::agent::LlmClient;
use crate::calendar::{Calendar, GoogleCalendar};
use crate::config::{AppConfig, Offer};
use crate::notify::Notifier;
use crate::state::AppState;
use crate::store::Store;
use crate::whatsapp::WhatsappClient;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("asistente_comercial=info,tower_http=warn")
        }))
        .init();

    let config = AppConfig::from_env().context("loading configuration")?;
    let offer = Offer::load(&config.offer_config_path).context("loading offer config")?;
    tracing::info!(
        business = %offer.branding.business_name,
        agent = %offer.branding.agent_name,
        "offer loaded"
    );

    let store = Store::connect(&config.database_url)
        .await
        .context("connecting to postgres")?;
    if config.run_migrations {
        store.migrate().await.context("running migrations")?;
        tracing::info!("migrations applied");
    }

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("building http client")?;

    let llm = LlmClient::new(http.clone(), &config.anthropic);
    let whatsapp = WhatsappClient::new(http.clone(), &config.whatsapp);
    let notifier = Notifier::new(whatsapp.clone(), http.clone());

    let sa_json = config
        .google
        .service_account_json
        .clone()
        .context("Google service account not set (GOOGLE_SERVICE_ACCOUNT_JSON or _PATH)")?;
    let calendar: Arc<dyn Calendar> = Arc::new(
        GoogleCalendar::new(http.clone(), &sa_json).context("initializing Google Calendar")?,
    );

    let crm = crm::build(http.clone(), &config.crm).context("initializing CRM backend")?;

    let bind_addr = config.bind_addr.clone();
    let state = AppState {
        config: Arc::new(config),
        offer: Arc::new(offer),
        store,
        llm,
        whatsapp,
        calendar,
        crm,
        notifier,
        locks: Default::default(),
    };

    let app = server::router(state);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("binding {bind_addr}"))?;
    tracing::info!("Asistente Comercial escuchando en http://{bind_addr}");
    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}
