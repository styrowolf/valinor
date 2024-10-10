use crate::GraphId;
use thiserror::Error;

// TODO: mmapped tarball version
mod directory_tile_provider;

use crate::graph_id::InvalidGraphIdError;
use crate::graph_tile::{GraphTile, GraphTileError};
pub use directory_tile_provider::DirectoryTileProvider;

#[derive(Debug, Error)]
pub enum GraphTileProviderError {
    #[error("This tile does not exist (ex: in your extract)")]
    TileDoesNotExist,
    #[error("Error fetching tile: {0}")]
    TileFetchError(String),
    #[error("Invalid graph ID: {0}")]
    InvalidGraphId(#[from] InvalidGraphIdError),
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Graph tile error: {0}")]
    GraphTileError(#[from] GraphTileError),
}

pub trait GraphTileProvider {
    // TODO: Should this return an owned structure or a pointer?
    // TODO: Should this be async? Seems like it to allow network retrieval.
    /// Gets a tile with the given graph ID.
    ///
    /// # Errors
    ///
    /// This operation may fail for several reasons,
    /// including the tile not existing, I/O errors, and more.
    /// Refer to [`GraphTileProviderError`] for details.
    fn get_tile(&self, graph_id: &GraphId) -> Result<GraphTile, GraphTileProviderError>;
    // TODO: Coordinate, bbox, etc. (can have implementations that are generic!!)
}
