pub mod eml;
pub mod eml_parser;
pub mod maildir;
pub mod pipeline;
pub mod sink;

pub use eml::build_eml;
pub use eml_parser::{extract_header, parse_eml_to_canonical};
pub use maildir::MaildirStorage;
pub use pipeline::{StoragePipeline, StorageReport};
pub use sink::{MaildirSink, SqliteSink, StorageSink};
