use crate::{AsCowStr, BIN_COUNT, GraphId};
use bitfield_struct::bitfield;
use geo::{Coord, coord};
use std::borrow::Cow;
use zerocopy_derive::FromBytes;

/// Remaining variable offset slots for growth.
/// See valhalla/balder/graphtileheader.h for details.
const EMPTY_SLOTS: usize = 11;

#[derive(FromBytes)]
#[bitfield(u64)]
struct FirstBitfield {
    #[bits(46)]
    graph_id: u64,
    #[bits(4)]
    density: u8,
    #[bits(4)]
    name_quality: u8,
    #[bits(4)]
    speed_quality: u8,
    #[bits(4)]
    exit_quality: u8,
    // NOTE: We don't use bool, because that doesn't have a `FromBytes` implementation.
    // We are only using a single bit though, so we *can* guarantee that the bit pattern is valid.
    // This will probably change once we have the ability to assume bit validity.
    // This is currently in beta: https://doc.rust-lang.org/beta/std/mem/trait.TransmuteFrom.html.
    #[bits(1)]
    has_elevation: u8,
    #[bits(1)]
    has_ext_directed_edge: u8,
}

#[derive(FromBytes)]
#[bitfield(u64)]
struct SecondBitfield {
    #[bits(21)]
    node_count: u32,
    #[bits(21)]
    directed_edge_count: u32,
    #[bits(21)]
    predicted_speeds_count: u32,
    #[bits(1)]
    _spare: u8,
}

// NOTE: There are a lot of weird paddings which arose because of an incompatibility in padding
// between x86 and amd64 architectures.
// This was entirely accidental upstream and will eventually be cleaned up in v4.

#[derive(FromBytes)]
#[bitfield(u32)]
struct TransitionCountBitfield {
    #[bits(22)]
    transition_count: u32,
    // Deprecated; scheduled for deletion in v4
    #[bits(10)]
    _spare: u16,
}

#[derive(FromBytes)]
#[bitfield(u32)]
struct TurnLaneCountBitfield {
    #[bits(21)]
    turn_lane_count: u32,
    #[bits(11)]
    _spare: u16,
}

#[derive(FromBytes)]
#[bitfield(u64)]
struct TransitRecordBitfield {
    #[bits(16)]
    transfer_count: u32,
    #[bits(7)]
    _spare: u16,
    #[bits(24)]
    departure_count: u32,
    #[bits(16)]
    stop_count: u16,
    #[bits(1)]
    _spare: u8,
}

#[derive(FromBytes)]
#[bitfield(u64)]
struct MiscCountsBitFieldOne {
    #[bits(12)]
    route_count: u16,
    #[bits(12)]
    schedule_count: u16,
    #[bits(24)]
    sign_count: u32,
    #[bits(16)]
    _spare: u16,
}

#[derive(FromBytes)]
#[bitfield(u64)]
struct MiscCountsBitFieldTwo {
    #[bits(24)]
    access_restriction_count: u32,
    #[bits(16)]
    admin_count: u16,
    #[bits(24)]
    _spare: u32,
}

// TODO: Unsure if packed is needed as well.
// Seems like it's technically not, since Valhalla is explicit about padding
/// Summary information about the graph tile.
///
/// This contains metadata like version,
/// number of nodes and edges,
/// and pointer offsets to other data.
#[derive(Copy, Clone, FromBytes, Debug)]
#[repr(C)]
pub struct GraphTileHeader {
    bit_field_1: FirstBitfield,
    base_lon_lat: [f32; 2],
    version: [u8; 16],
    /// The dataset ID (canonically, the last OSM changeset ID).
    pub dataset_id: u64,
    bit_field_2: SecondBitfield,
    transition_count_bitfield: TransitionCountBitfield,
    turn_lane_count_bitfield: TurnLaneCountBitfield,
    transit_record_bitfield: TransitRecordBitfield,
    misc_counts_bit_field_one: MiscCountsBitFieldOne,
    misc_counts_bit_field_two: MiscCountsBitFieldTwo,
    /// These can be used for custom information.
    /// As long as the size of the header and order of the date of the structure doesn't change,
    /// this should be backward compatible.
    pub reserved: u128,
    // TODO: Valhalla has the unhelpful comment "Offset to the beginning of the variable sized data."
    // We should improve these.
    pub complex_restriction_forward_offset: u32,
    pub complex_restriction_reverse_offset: u32,
    pub edge_info_offset: u32,
    pub text_list_offset: u32,
    /// The date the tile was created (since pivot date).
    /// TODO: Figure out what pivot date means; probably hide the raw value
    pub create_date: u32,
    /// Offsets for each bin of the grid (for search/lookup)
    pub bin_offsets: [u32; BIN_COUNT as usize],
    /// Offset to the beginning of the variable sized data.
    pub lane_connectivity_offset: u32,
    /// Offset to the beginning of the variable sized data.
    pub predicted_speeds_offset: u32,
    /// The size of the tile (in bytes)
    pub tile_size: u32,
    /// See valhalla/balder/graphtileheader.h for details.
    _empty_slots: [u32; EMPTY_SLOTS],
}

impl GraphTileHeader {
    /// The full Graph ID of this tile.
    #[inline]
    pub const fn graph_id(&self) -> GraphId {
        // Safety: We know that the bit field cannot contain a value
        // larger than the max allowed value.
        unsafe { GraphId::from_id_unchecked(self.bit_field_1.graph_id()) }
    }

    /// The relative road density within this tile (0-15).
    #[inline]
    pub const fn density(&self) -> u8 {
        self.bit_field_1.density()
    }

    /// The relative quality of name assignment for this tile (0-15).
    #[inline]
    pub const fn name_quality(&self) -> u8 {
        self.bit_field_1.name_quality()
    }

    /// The relative quality of speed assignment for this tile (0-15).
    #[inline]
    pub const fn speed_quality(&self) -> u8 {
        self.bit_field_1.speed_quality()
    }

    /// The relative quality of exit signs for this tile (0-15).
    #[inline]
    pub const fn exit_quality(&self) -> u8 {
        self.bit_field_1.exit_quality()
    }

    /// Does this tile include elevation data?
    #[inline]
    pub const fn has_elevation(&self) -> bool {
        self.bit_field_1.has_elevation() != 0
    }

    /// Does this tile include extended directed edge attributes?
    #[inline]
    pub const fn has_ext_directed_edge(&self) -> bool {
        self.bit_field_1.has_ext_directed_edge() != 0
    }

    /// The coordinate of the southwest corner of this graph tile.
    #[inline]
    pub const fn sw_corner(&self) -> Coord<f32> {
        coord! {x: self.base_lon_lat[0], y: self.base_lon_lat[1]}
    }

    /// Gets the version of Baldr used to generate this graph tile.
    pub fn version(&self) -> Cow<'_, str> {
        self.version.as_cow_str()
    }

    #[inline]
    pub const fn node_count(&self) -> u32 {
        self.bit_field_2.node_count()
    }

    /// The number of directed edges in this graph tile.
    #[inline]
    pub const fn directed_edge_count(&self) -> u32 {
        self.bit_field_2.directed_edge_count()
    }

    /// The number of predicted speed records in this graph tile.
    #[inline]
    pub const fn predicted_speeds_count(&self) -> u32 {
        self.bit_field_2.predicted_speeds_count()
    }

    /// The number of node transitions in this graph tile.
    #[inline]
    pub const fn transition_count(&self) -> u32 {
        self.transition_count_bitfield.transition_count()
    }

    /// The number of turn lanes in this graph tile.
    #[inline]
    pub const fn turn_lane_count(&self) -> u32 {
        self.turn_lane_count_bitfield.turn_lane_count()
    }

    /// The number of transit transfers in this graph tile.
    #[inline]
    pub const fn transfer_count(&self) -> u32 {
        self.transit_record_bitfield.transfer_count()
    }

    /// The number of transit departures in this graph tile.
    #[inline]
    pub const fn departure_count(&self) -> u32 {
        self.transit_record_bitfield.departure_count()
    }

    /// The number of transit stops in this graph tile.
    #[inline]
    pub const fn stop_count(&self) -> u16 {
        self.transit_record_bitfield.stop_count()
    }

    /// The number of transit routes in this graph tile.
    #[inline]
    pub const fn route_count(&self) -> u16 {
        self.misc_counts_bit_field_one.route_count()
    }

    /// The number of transit schedules in this graph tile.
    #[inline]
    pub const fn schedule_count(&self) -> u16 {
        self.misc_counts_bit_field_one.schedule_count()
    }

    /// The number of signs in this graph tile.
    #[inline]
    pub const fn sign_count(&self) -> u32 {
        self.misc_counts_bit_field_one.sign_count()
    }

    /// The number of access restrictions in this graph tile.
    #[inline]
    pub const fn access_restriction_count(&self) -> u32 {
        self.misc_counts_bit_field_two.access_restriction_count()
    }

    /// The number of admin records in this graph tile.
    #[inline]
    pub const fn admin_count(&self) -> u16 {
        self.misc_counts_bit_field_two.admin_count()
    }
}

#[cfg(test)]
mod test {
    use crate::graph_tile::TEST_GRAPH_TILE;

    #[test]
    fn test_parse_header() {
        let tile = &*TEST_GRAPH_TILE;

        insta::assert_debug_snapshot!(tile.header);
    }
}
