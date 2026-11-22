mod buffer;
mod change;
mod document;
mod error;
mod history;
mod position;
mod search;
mod selection;
mod suggestion;
#[cfg(test)]
mod testing;
mod transaction;

pub use buffer::Buffer;
pub use change::{Assoc, ChangeSet, Edit};
pub use document::Document;
pub use error::TextError;
pub use history::{GROUP_DELAY, History};
pub use position::{LineColumn, Position, Unit};
pub use search::{PatternError, Query, Search};
pub use selection::{Selection, SelectionRange};
pub use suggestion::{Suggestion, SuggestionKind, group_at};
pub use transaction::Transaction;
