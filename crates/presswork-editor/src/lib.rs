mod action;
mod boundary;
mod complete;
mod editor;
mod fold;
mod highlight;
mod layout;
mod suggest;
mod view;

pub use action::{Action, Deletion, Motion};
pub use complete::{CompletionEvent, CompletionItem, CompletionList};
pub use editor::Editor;
pub use highlight::{Highlighter, Role, Style};
pub use view::{EditorView, Mark, MarkKind, Tooltip, view};
