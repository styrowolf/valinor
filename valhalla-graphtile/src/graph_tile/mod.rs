use bytes::Bytes;
use thiserror::Error;

mod header;

pub use header::GraphTileHeader;

#[derive(Debug, Error)]
pub enum GraphTileError {
    #[error("The graph tile header bytes are not of the expected length.")]
    InvalidHeaderSize,
}

pub struct GraphTile {
    /// The raw underlying graph tile bytes.
    data: Bytes,
    pub header: GraphTileHeader,
    // TODO: List of nodes
    // TODO: List of transitions
    // TODO: List of directed edges
    // TODO: A WHOLE lot more LOL
}

impl GraphTile {
    pub(crate) fn new(data: Bytes) -> Result<Self, GraphTileError> {
        let header = GraphTileHeader::from_bytes(&data[0..size_of::<GraphTileHeader>()])?;
        Ok(Self { data, header })
    }
}
