mod buffer;
mod change;
mod error;
mod position;
#[cfg(test)]
mod testing;

pub use buffer::Buffer;
pub use change::{Assoc, ChangeSet, Edit};
pub use error::TextError;
pub use position::{LineColumn, Position, Unit};
