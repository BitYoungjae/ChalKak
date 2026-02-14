//! Core editor operation model and undo/redo state.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionState {
    None,
    Single(u64),
}

impl SelectionState {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn single_id(&self) -> Option<u64> {
        match self {
            Self::None => None,
            Self::Single(id) => Some(*id),
        }
    }

    pub fn is_selected(&self, id: u64) -> bool {
        match self {
            Self::None => false,
            Self::Single(current) => *current == id,
        }
    }

    pub fn select(&mut self, id: u64) {
        *self = Self::Single(id);
    }

    pub fn clear(&mut self) {
        *self = Self::None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorDocument {
    pub width: u32,
    pub height: u32,
    pub elements: Vec<String>,
}

impl EditorDocument {
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            elements: Vec::new(),
        }
    }

    pub fn add_element(&mut self, name: impl Into<String>) {
        self.elements.push(name.into());
    }

    pub fn remove_last_element(&mut self) -> Option<String> {
        self.elements.pop()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorOperation {
    id: u64,
    summary: String,
    before: EditorDocument,
    after: EditorDocument,
}

impl EditorOperation {
    pub const fn id(&self) -> u64 {
        self.id
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn before_snapshot(&self) -> &EditorDocument {
        &self.before
    }

    pub fn after_snapshot(&self) -> &EditorDocument {
        &self.after
    }
}

#[derive(Debug, Default)]
pub struct EditorOperationModel {
    undo_stack: Vec<EditorOperation>,
    redo_stack: Vec<EditorOperation>,
    current: EditorDocument,
    next_operation_id: u64,
}

impl EditorOperationModel {
    pub fn new(document: EditorDocument) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current: document,
            next_operation_id: 1,
        }
    }

    pub fn current_document(&self) -> &EditorDocument {
        self.current()
    }

    pub fn current(&self) -> &EditorDocument {
        &self.current
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.next_operation_id = 1;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_len(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn operation_count(&self) -> usize {
        self.undo_stack.len() + self.redo_stack.len()
    }

    pub fn commit(
        &mut self,
        summary: impl Into<String>,
        mutator: impl FnOnce(&mut EditorDocument),
    ) {
        let before = self.current.clone();
        let mut after = before.clone();
        mutator(&mut after);

        if before == after {
            return;
        }

        let operation = EditorOperation {
            id: self.next_operation_id,
            summary: summary.into(),
            before,
            after: after.clone(),
        };
        self.next_operation_id += 1;

        self.current = after;
        self.undo_stack.push(operation);
        self.redo_stack.clear();
    }

    pub fn record(
        &mut self,
        summary: impl Into<String>,
        mutator: impl FnOnce(&mut EditorDocument),
    ) {
        self.commit(summary, mutator);
    }

    pub fn undo(&mut self) -> Option<EditorOperation> {
        let operation = self.undo_stack.pop()?;
        self.current = operation.before.clone();
        self.redo_stack.push(operation.clone());
        Some(operation)
    }

    pub fn redo(&mut self) -> Option<EditorOperation> {
        let operation = self.redo_stack.pop()?;
        self.current = operation.after.clone();
        self.undo_stack.push(operation.clone());
        Some(operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> EditorDocument {
        EditorDocument::new(800, 450)
    }

    #[test]
    fn editor_undo_redo_records_operations_and_restores_snapshots() {
        let mut model = EditorOperationModel::new(doc());

        model.commit("add title", |document| {
            document.add_element("title");
        });
        model.commit("add body", |document| {
            document.add_element("body");
        });

        assert_eq!(model.current().elements, vec!["title", "body"]);
        assert_eq!(model.undo_len(), 2);
        assert_eq!(model.redo_len(), 0);
        assert!(model.can_undo());
        assert!(!model.can_redo());

        let undone = model.undo().expect("first undo should exist");
        assert_eq!(undone.summary(), "add body");
        assert_eq!(model.current().elements, vec!["title"]);
        assert_eq!(model.redo_len(), 1);

        let undone = model.undo().expect("second undo should exist");
        assert_eq!(undone.summary(), "add title");
        assert!(model.current().elements.is_empty());
        assert!(!model.can_undo());
        assert!(model.can_redo());

        let redone = model.redo().expect("redo should restore last op");
        assert_eq!(redone.summary(), "add title");
        assert_eq!(model.current().elements, vec!["title"]);
        assert!(model.can_undo());

        let redone = model.redo().expect("second redo should restore next op");
        assert_eq!(redone.summary(), "add body");
        assert_eq!(model.current().elements, vec!["title", "body"]);
        assert!(!model.can_redo());
    }

    #[test]
    fn editor_undo_redo_clears_redo_after_new_record() {
        let mut model = EditorOperationModel::new(doc());

        model.commit("add marker", |document| {
            document.add_element("marker-a");
        });
        model.undo().expect("undo should exist");
        assert!(model.can_redo());

        model.commit("replace with marker", |document| {
            document.elements.clear();
            document.add_element("marker-b");
        });

        assert_eq!(model.current().elements, vec!["marker-b"]);
        assert_eq!(model.undo_len(), 1);
        assert_eq!(model.redo_len(), 0);
        assert!(model.can_undo());
    }

    #[test]
    fn editor_selection_state_starts_empty() {
        let selection = SelectionState::None;
        assert!(selection.is_empty());
    }

    #[test]
    fn editor_selection_single_replaces_previous_selection() {
        let mut selection = SelectionState::None;

        selection.select(11);
        selection.select(22);

        assert!(selection.is_selected(22));
        assert!(!selection.is_selected(11));
        assert_eq!(selection.single_id(), Some(22));
    }

    #[test]
    fn editor_selection_clear_removes_selection() {
        let mut selection = SelectionState::Single(88);

        selection.clear();

        assert!(selection.is_empty());
        assert_eq!(selection.single_id(), None);
        assert!(!selection.is_selected(88));
    }
}
