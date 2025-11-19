//! # Graph tile providers
//!
//! This module contains implementations for several graph tile providers.
//! Note that there is currently no single generic trait for accessing tiles,
//! due to the fundamental difference in how memory maps work vs file systems.

use crate::GraphId;
use dashmap::DashMap;
use geo::{Destination, Haversine, Point};
use std::sync::Arc;
use std::sync::Mutex;
use thiserror::Error;

mod directory;
mod tarball;
mod traffic;

use crate::graph_id::InvalidGraphIdError;
use crate::graph_tile::{
    GraphTile, GraphTileDecodingError, GraphTileView, LookupError, NodeInfo, OwnedGraphTileHandle,
};
pub use directory::DirectoryGraphTileProvider;
pub use tarball::TarballTileProvider;
pub use traffic::TrafficTileProvider;

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
    #[error("Decoding error: {0}")]
    DecodingError(#[from] GraphTileDecodingError),
    #[error("Graph tile lookup error: {0}")]
    GraphTileLookupError(#[from] LookupError),
    #[error("Cache lock is poisoned: {0}")]
    PoisonedCacheLock(String),
    #[error("Invalid tarball: {0}")]
    InvalidTarball(String),
    #[error("Unsupported tile version; this may or may not be compatible.")]
    UnsupportedTileVersion,
}

pub trait GraphTileProvider {
    /// Gets the tile containing the given graph ID,
    /// and does some work in a closure which takes the reference as a parameter.
    ///
    /// # Performance
    ///
    /// This may or may not make use of caches,
    /// require filesystem access, etc.
    /// This is left up to the implementation.
    ///
    /// # Errors
    ///
    /// This operation may fail for several reasons,
    /// including the tile not existing, I/O errors, and more.
    /// Refer to [`GraphTileProviderError`] for details.
    fn with_tile<F, T>(&self, graph_id: GraphId, process: F) -> Result<T, GraphTileProviderError>
    where
        F: FnOnce(&GraphTileView) -> T;

    /// Enumerate base tile Graph IDs across all hierarchy levels that intersect a circle around
    /// `center` with radius `radius`.
    ///
    /// Assumes that `center` is a valid geographic coordinate (lat+lon).
    /// `radius` is specified in meters.
    ///
    /// # Implementation note
    ///
    /// All returned tiles MUST be accessible by the provider (callers are allowed to assume this).
    fn enumerate_tiles_within_radius(&self, center: Point, radius: f64) -> Vec<GraphId>;

    /// Gets a tile containing the given graph ID, or else panics.
    ///
    /// This is an unfortunately necessary convenience,
    /// since some use cases involve use in contexts where failure is not encodable
    /// in the type signature.
    ///
    /// Uses [`GraphTileProvider::with_tile`] under the hood.
    ///
    /// # Panics
    ///
    /// This will panic if the tile can't be loaded.
    fn with_tile_containing_or_panic<F, T>(&self, graph_id: GraphId, process: F) -> T
    where
        F: FnOnce(&GraphTileView) -> T,
    {
        self.with_tile(graph_id, process)
            .expect("Should be able to get a tile for this node (either the trait impl is incorrect or a tile has invalid data)")
    }

    /// Gets the opposing edge.
    ///
    /// # Errors
    ///
    /// This may fail under the following circumstances:
    ///
    /// - Failing to fetch the tile containing the graph ID
    /// - The index within the tile being invalid
    /// - Failing to fetch the tile containing the end node of the edge (or the end node therein)
    /// - Corrupt end node information in the tile
    ///
    /// # Performance
    ///
    /// This method always has to do a tile lookup (potentially cached, but a lookup nonetheless).
    /// This is MUCH slower than looking at the tile first, so you should always call
    /// [`GraphTile::get_opp_edge_index`] first
    fn get_opposing_edge(&self, graph_id: GraphId) -> Result<GraphId, GraphTileProviderError> {
        self.with_tile(graph_id, |tile| {
            let edge = tile.get_directed_edge(graph_id)?;

            // The edge might leave the tile, so we have to do a complicated lookup
            let end_node_id = edge.end_node_id();
            let opp_edge_index = edge.opposing_edge_index();

            // TODO: Probably a cleaner pattern here?
            let node_edge_index = match tile.get_node(end_node_id).map(NodeInfo::edge_index) {
                Ok(index) => index,
                Err(LookupError::MismatchedBase) => self.with_tile(end_node_id, |tile| {
                    Ok::<_, GraphTileProviderError>(tile.get_node(end_node_id)?.edge_index())
                })??,
                Err(LookupError::InvalidIndex) => return Err(LookupError::InvalidIndex)?,
            };

            // Construct an ID with the index set to the opposing edge
            let id = GraphId::try_from_components(
                end_node_id.level(),
                end_node_id.tile_id(),
                u64::from(node_edge_index + opp_edge_index),
            )?;

            // TODO: Should we try to return the edge too?
            // let edge = opp_tile.get_directed_edge(&id)?;

            Ok(id)
        })?
    }
}

pub trait OwnedGraphTileProvider: GraphTileProvider {
    /// Gets a tile containing the given graph ID.
    ///
    /// The result is an owned handle to a graph tile,
    /// which implements the [`GraphTile`](GraphTile) trait.
    ///
    /// # Errors
    ///
    /// This operation may fail for several reasons,
    /// including the tile not existing, I/O errors, and more.
    /// Refer to [`GraphTileProviderError`] for details.
    fn get_handle_for_tile_containing(
        &self,
        graph_id: GraphId,
    ) -> Result<Arc<OwnedGraphTileHandle>, GraphTileProviderError>;
}

/// A keyed lock.
///
/// This enables more granular locking than over an entire data structure.
pub(crate) struct LockTable<K>(DashMap<K, Arc<Mutex<()>>>);

impl<K: std::hash::Hash + Eq + Clone> LockTable<K> {
    pub fn new() -> Self {
        Self(DashMap::new())
    }

    pub fn lock_for(&self, k: K) -> Arc<Mutex<()>> {
        self.0
            .entry(k)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
}

/// Returns a bounding box centered upon `center` containing a circle with radius `radius` meters.
///
/// The bbox is in (N, E, S, W) order.
pub(crate) fn bbox_with_center(center: Point, radius: f64) -> (f64, f64, f64, f64) {
    // Per https://github.com/georust/geo/pull/1091/,
    // the longitude values should be normalized to [-180, 180].
    // We assert this again in a unit test below.
    let north = Haversine.destination(center, 0.0, radius).y();
    let east = Haversine.destination(center, 90.0, radius).x();
    let south = Haversine.destination(center, 180.0, radius).y();
    let west = Haversine.destination(center, 270.0, radius).x();

    (north, east, south, west)
}

#[cfg(test)]
mod tests {
    use geo::{Destination, Haversine, point};

    #[test]
    fn haversine_antimeridian_wraps() {
        // Wrapping from east -> west
        let projected = Haversine.destination(point!(x: 179.9, y: 0.0), 90.0, 50_000.0);
        assert!(projected.x() < -179.0);
        assert!(projected.x() > -180.0);

        // Wrapping the other way
        let projected = Haversine.destination(point!(x: -179.9, y: 0.0), 270.0, 50_000.0);
        assert!(projected.x() > 179.0);
        assert!(projected.x() < 180.0);
    }
}
