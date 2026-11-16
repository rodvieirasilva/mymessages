pub mod attachment;
pub mod conversation;
pub mod message;

pub use attachment::{CanonicalAttachment, MediaType};
pub use conversation::{Contact, Conversation};
pub use message::{CanonicalMessage, CanonicalParticipant};
