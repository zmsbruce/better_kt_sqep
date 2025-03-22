use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum GraphError {
    #[error("entity {0} not found")]
    EntityNotFound(u64),
    #[error("edge ({0}, {1}) not found")]
    EdgeNotFound(u64, u64),
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("nothing to redo")]
    NothingToRedo,
}

#[derive(Debug, Error)]
pub enum SerdeError {
    #[error("failed to serialize to xml")]
    Serialize(#[from] quick_xml::SeError),
    #[error("failed to deserialize from xml")]
    Deserialize(#[from] quick_xml::DeError),
    #[error("failed to parse utf8 string")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("unexpected {0}: {1}")]
    Unexpected(&'static str, String),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("graph error: {0}")]
    Graph(#[from] GraphError),
    #[error("serde error: {0}")]
    Serde(#[from] SerdeError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("poison error: {0}")]
    Poison(String),
}
