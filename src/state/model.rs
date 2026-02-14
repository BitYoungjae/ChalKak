#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AppState {
    #[default]
    Idle,
    Preview,
    Editor,
}
