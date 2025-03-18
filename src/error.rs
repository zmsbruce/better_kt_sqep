use thiserror::Error;

use crate::graph::EntityId;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("entity {0} not found")]
    EntityNotFound(EntityId),
    #[error("edge ({0}, {1}) not found")]
    EdgeNotFound(EntityId, EntityId),
    #[error("edge ({0}, {1}) already exists")]
    EdgeAlreadyExists(EntityId, EntityId),
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("nothing to redo")]
    NothingToRedo,
}
