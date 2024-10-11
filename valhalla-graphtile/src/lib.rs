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

use enumset::EnumSetType;
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

/// Access permission by travel type.
///
/// This is stored internally as a bit field.
/// NB: While it is a 16-bit integer, the way it is stored in directed edges
/// only allows for TWELVE bits to be used!
#[derive(Debug, EnumSetType)]
#[enumset(repr = "u16")]
pub enum Access {
    Auto,
    Pedestrian,
    Bicycle,
    Truck,
    Emergency,
    Taxi,
    Bus,
    HOV,
    Wheelchair,
    Moped,
    Motorcycle,
    GolfCart,
    // NB: Only 12 bits are allowed to be used!
    // All = 4095,
}

/// The number of subdivisions in each graph tile
const BIN_COUNT: u8 = 25;

#[cfg(test)]
mod tests {
    use enumset::EnumSet;
    use crate::Access;

    #[test]
    fn test_access_representation() {
        let set: EnumSet<Access> = EnumSet::from_repr(2048);
        assert_eq!(set.len(), 1);
        assert!(set.contains(Access::GolfCart));
    }

    #[test]
    fn test_all_access_representation() {
        let set: EnumSet<Access> = EnumSet::all();
        assert_eq!(set.len(), 12);
        assert_eq!(set.as_repr(), 4095);
    }
}