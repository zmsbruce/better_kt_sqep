use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("entity {0} not found")]
    EntityNotFound(u64),
    #[error("edge ({0}, {1}) not found")]
    EdgeNotFound(u64, u64),
    #[error("edge ({0}, {1}) already exists")]
    EdgeAlreadyExists(u64, u64),
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("nothing to redo")]
    NothingToRedo,
}
