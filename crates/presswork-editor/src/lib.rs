mod action;
mod boundary;
mod editor;
mod highlight;
mod layout;
mod view;

pub use action::{Action, Deletion, Motion};
pub use editor::Editor;
pub use highlight::{Highlighter, Role, Style};
pub use view::{EditorView, view};
