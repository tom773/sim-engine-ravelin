pub trait Decision: Send + Sync + 'static {
    type Action: crate::Action;          // what kind of action this decision yields
    fn into_actions(self) -> Vec<Self::Action>;
}