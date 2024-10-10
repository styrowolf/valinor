//! # Valhalla Graph Tile Manipulation
//!
//! This is an oxidized version of the graph tile access, manipulation and data structures
//! of Valhalla's `baldr` module.

// Private modules by default
mod graph_id;
mod graph_reader;
pub mod graph_tile;
pub mod tile_hierarchy;
pub mod tile_provider;

// Pub use for re-export without too many levels of hierarchy.
// The implementations are sufficiently complex that we want to have lots of files,
// But many of those only have one or two useful definitions to re-export,
// so this flattens things for better ergonomics.
pub use graph_id::GraphId;
pub use graph_reader::GraphReader;

#[repr(u8)]
pub enum RoadClass {
    Motorway,
    Trunk,
    Primary,
    Secondary,
    Tertiary,
    Unclassified,
    Residential,
    ServiceOther,
}

/// The number of subdivisions in each graph tile
const BIN_COUNT: u8 = 25;
