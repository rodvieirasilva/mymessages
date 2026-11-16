pub mod db;
pub mod fts;
pub mod hasher;
pub mod reindex;
pub mod writer;

pub use db::Database;
pub use fts::{list_chats, get_chat_messages, search_messages, AttachmentInfo, ChatSummary, SearchQuery, SearchResult};
pub use hasher::{is_hash_indexed, record_hash};
pub use reindex::{reindex_from_folder, reindex_from_maildir, ReindexReport, ReindexSource, ReindexTarget};
pub use writer::index_message;
