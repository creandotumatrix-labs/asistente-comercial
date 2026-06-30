//! Shared application state, cloned cheaply into every request and spawned task
//! (everything behind `Arc` or already cheap to clone).

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

use crate::agent::LlmClient;
use crate::calendar::Calendar;
use crate::config::{AppConfig, Offer};
use crate::crm::Crm;
use crate::notify::Notifier;
use crate::store::Store;
use crate::whatsapp::WhatsappClient;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub offer: Arc<Offer>,
    pub store: Store,
    pub llm: LlmClient,
    pub whatsapp: WhatsappClient,
    pub calendar: Arc<dyn Calendar>,
    pub crm: Arc<dyn Crm>,
    pub notifier: Notifier,
    pub locks: ConvLocks,
}

/// Per-conversation async locks keyed by WhatsApp id. Serializes processing for
/// a single contact so rapid-fire inbound messages don't race the stored history.
#[derive(Clone, Default)]
pub struct ConvLocks {
    inner: Arc<StdMutex<HashMap<String, Arc<AsyncMutex<()>>>>>,
}

impl ConvLocks {
    pub async fn acquire(&self, key: &str) -> OwnedMutexGuard<()> {
        let lock = {
            let mut map = self.inner.lock().expect("conv lock map poisoned");
            map.entry(key.to_string())
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };
        lock.lock_owned().await
    }
}
