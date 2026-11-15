mod action;
mod boundary;
mod editor;
mod highlight;

pub use action::{Action, Deletion, Motion};
pub use editor::Editor;
pub use highlight::{Highlighter, Role, Style};
