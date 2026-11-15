use std::ops::Range;
use std::time::{Duration, Instant};

use crate::buffer::Buffer;
use crate::change::{Assoc, ChangeSet};
use crate::error::TextError;
use crate::position::Position;
use crate::selection::Selection;
use crate::transaction::Transaction;

pub const GROUP_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone)]
struct Entry {
    changes: ChangeSet,
    selection: Selection,
}

#[derive(Debug, Clone)]
struct Last {
    time: Instant,
    touched: Vec<Range<usize>>,
}

#[derive(Debug, Clone)]
pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    last: Option<Last>,
    group_delay: Duration,
}

impl Default for History {
    fn default() -> Self {
        Self::with_group_delay(GROUP_DELAY)
    }
}

impl History {
    #[must_use]
    pub fn with_group_delay(group_delay: Duration) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            last: None,
            group_delay,
        }
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn record(
        &mut self,
        inverse: ChangeSet,
        changes: &ChangeSet,
        selection: Selection,
        at: Instant,
    ) -> Result<(), TextError> {
        let joins = self.last.as_ref().is_some_and(|last| {
            at.saturating_duration_since(last.time) < self.group_delay
                && changes.changed_ranges().any(|(old, _)| {
                    last.touched
                        .iter()
                        .any(|range| old.start <= range.end && range.start <= old.end)
                })
        });
        match self.undo.last_mut() {
            Some(entry) if joins => entry.changes = inverse.compose(&entry.changes)?,
            _ => self.undo.push(Entry {
                changes: inverse,
                selection,
            }),
        }
        self.redo.clear();
        self.last = Some(Last {
            time: at,
            touched: changes.changed_ranges().map(|(_, new)| new).collect(),
        });
        Ok(())
    }

    pub fn break_group(&mut self) {
        self.last = None;
    }

    pub fn undo(
        &mut self,
        buffer: &Buffer,
        selection: &Selection,
    ) -> Result<Option<Transaction>, TextError> {
        self.last = None;
        step(&mut self.undo, &mut self.redo, buffer, selection)
    }

    pub fn redo(
        &mut self,
        buffer: &Buffer,
        selection: &Selection,
    ) -> Result<Option<Transaction>, TextError> {
        self.last = None;
        step(&mut self.redo, &mut self.undo, buffer, selection)
    }

    pub fn rebase(&mut self, remote: &ChangeSet) -> Result<(), TextError> {
        let undo = rebase(&self.undo, remote)?;
        let redo = rebase(&self.redo, remote)?;
        self.undo = undo;
        self.redo = redo;
        if let Some(last) = &mut self.last {
            for range in &mut last.touched {
                let start = remote.map_position(Position::new(range.start), Assoc::Before);
                let end = remote.map_position(Position::new(range.end), Assoc::After);
                *range = start.offset()..end.offset();
            }
        }
        Ok(())
    }
}

fn step(
    from: &mut Vec<Entry>,
    to: &mut Vec<Entry>,
    buffer: &Buffer,
    selection: &Selection,
) -> Result<Option<Transaction>, TextError> {
    let Some(entry) = from.pop() else {
        return Ok(None);
    };
    let reverse = match entry.changes.invert(buffer) {
        Ok(reverse) => reverse,
        Err(error) => {
            from.push(entry);
            return Err(error);
        }
    };
    to.push(Entry {
        changes: reverse,
        selection: selection.clone(),
    });
    Ok(Some(
        Transaction::new(entry.changes).with_selection(entry.selection),
    ))
}

fn rebase(stack: &[Entry], remote: &ChangeSet) -> Result<Vec<Entry>, TextError> {
    let mut mapping = remote.clone();
    let mut rebased = Vec::with_capacity(stack.len());
    for entry in stack.iter().rev() {
        let changes = entry.changes.map(&mapping, false)?;
        mapping = mapping.map(&entry.changes, true)?;
        rebased.push(Entry {
            changes,
            selection: entry.selection.map(&mapping),
        });
    }
    rebased.reverse();
    rebased.retain(|entry| !entry.changes.is_empty());
    Ok(rebased)
}
