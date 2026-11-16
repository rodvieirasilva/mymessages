use anyhow::Result;
use crate::models::CanonicalMessage;

pub mod evolution;
pub mod folder;
pub mod waha;

pub use evolution::EvolutionApiAdapter;
pub use folder::FolderAdapter;
pub use waha::WahaAdapter;

pub trait SourceAdapter: Send + Sync {
    fn source_id(&self) -> &'static str;
    fn parse_webhook(&self, payload: &serde_json::Value) -> Result<Option<CanonicalMessage>>;
}
