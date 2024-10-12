use crate::graph_tile::GraphTileError;
use crate::{GraphId, RoadUse};
use bitfield_struct::bitfield;
use std::fmt::{Debug, Formatter};
use zerocopy::try_transmute;
use zerocopy_derive::{FromBytes, Immutable, TryFromBytes};

#[derive(FromBytes)]
#[bitfield(u64)]
struct FirstBitfield {
    #[bits(46)]
    end_node: u64,
    #[bits(8)]
    restrictions: u8,
    #[bits(7)]
    opposing_edge_index: u32,
    // Booleans represented this way for infailability.
    // See comment in node_info.rs for details.
    #[bits(1)]
    forward: u8,
    #[bits(1)]
    leaves_tile: u8,
    #[bits(1)]
    country_crossing: u8,
}

#[derive(FromBytes)]
#[bitfield(u64)]
struct SecondBitfield {
    #[bits(25)]
    edgeinfo_offset: u32,
    #[bits(12)]
    access_restrictions: u16,
    #[bits(12)]
    start_restriction: u16,
    #[bits(12)]
    end_restriction: u16,
    // Booleans represented this way for infailability.
    // See comment in node_info.rs for details.
    #[bits(1)]
    complex_restriction: u8,
    #[bits(1)]
    dest_only: u8,
    #[bits(1)]
    not_thru: u8,
}

#[derive(FromBytes)]
#[bitfield(u64)]
struct ThirdBitfield {
    #[bits(8)]
    speed: u8,
    #[bits(8)]
    free_flow_speed: u8,
    #[bits(8)]
    constrained_flow_speedspeed: u8,
    #[bits(8)]
    truck_speed: u8,
    #[bits(8)]
    name_consistency: u8,
    #[bits(6)]
    edge_use: u8,
    #[bits(4)]
    lane_count: u8,
    #[bits(4)]
    density: u8,
    #[bits(3)]
    classification: u8,
    #[bits(3)]
    surface: u8,
    // Booleans represented this way for infailability.
    // See comment in node_info.rs for details.
    #[bits(1)]
    toll: u8,
    #[bits(1)]
    roundabout: u8,
    #[bits(1)]
    truck_route: u8,
    #[bits(1)]
    has_predicted_speed: u8,
}

#[derive(FromBytes)]
#[bitfield(u64)]
struct FourthBitfield {
    #[bits(12)]
    forward_access: u16,
    #[bits(12)]
    reverse_access: u16,
    #[bits(5)]
    max_up_slope: u8,
    #[bits(5)]
    max_down_slope: u8,
    #[bits(3)]
    sac_scale: u8,
    #[bits(2)]
    cycle_lane: u8,
    // Booleans represented this way for infailability.
    // See comment in node_info.rs for details.
    #[bits(1)]
    is_bike_network: u8,
    #[bits(1)]
    use_sidepath: u8,
    #[bits(1)]
    bicycle_dismount: u8,
    #[bits(1)]
    has_sidewalk_left: u8,
    #[bits(1)]
    has_sidewalk_right: u8,
    #[bits(1)]
    has_shoulder: u8,
    #[bits(1)]
    has_lane_connectivity: u8,
    #[bits(1)]
    has_turn_lanes: u8,
    #[bits(1)]
    has_exit_signs: u8,
    #[bits(1)]
    is_intersection_internal: u8,
    #[bits(1)]
    is_tunnel: u8,
    #[bits(1)]
    is_bridge: u8,
    #[bits(1)]
    has_traffic_signal: u8,
    #[bits(1)]
    is_seasonal: u8,
    #[bits(1)]
    is_dead_end: u8,
    #[bits(1)]
    has_bss_connection: u8,
    #[bits(1)]
    has_stop_sign: u8,
    #[bits(1)]
    has_yield_sign: u8,
    #[bits(1)]
    hov_type: u8,
    #[bits(1)]
    is_indoor: u8,
    #[bits(1)]
    is_lit: u8,
    #[bits(1)]
    is_dest_only_hgv: u8,
    #[bits(3)]
    _spare: u8,
}

#[derive(FromBytes)]
#[bitfield(u64)]
struct FifthBitfield {
    #[bits(24)]
    turn_type: u32,
    #[bits(8)]
    edge_to_left: u8,
    #[bits(24)]
    length: u32,
    #[bits(4)]
    weighted_grade: u8,
    #[bits(4)]
    curvature: u8,
}

#[derive(FromBytes, Immutable)]
#[bitfield(u32)]
struct StopImpact {
    #[bits(24)]
    impact_between_edges: u32,
    #[bits(8)]
    edge_to_right: u8,
}

/// Stores either the stop impact or the transit line identifier.
/// Since transit lines are schedule-based, they have no need for edge transition logic,
/// so we can freely share this field.
#[derive(FromBytes, Immutable)]
#[repr(C)]
union StopOrLine {
    stop_impact: StopImpact,
    line_id: u32,
}

#[derive(FromBytes)]
#[bitfield(u32)]
struct SeventhBitField {
    #[bits(7)]
    local_level_edge_index: u8,
    #[bits(7)]
    local_level_opp_edge_index: u8,
    #[bits(7)]
    shortcut: u8,
    #[bits(7)]
    superseded: u8,
    // Booleans represented this way for infailability.
    // See comment in node_info.rs for details.
    #[bits(1)]
    is_shortcut: u8,
    #[bits(1)]
    speed_type: u8,
    #[bits(1)]
    is_named: u8,
    #[bits(1)]
    link_tag: u8,
}

impl Debug for StopOrLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Safety: It doesn't matter if we interpret this wrong as both fields are
        // packed into a u32.
        unsafe { f.write_fmt(format_args!("StopOrLine {{ raw_value: {} }}", self.line_id)) }
    }
}

/// A directed edge within the routing graph.
///
/// This struct contains only the essential edge information
/// which is needed for routing calculations.
/// Additional details can be found in the [`EdgeInfo`] struct,
/// which contains things like the encoded shape, OSM way ID,
/// and other info that is not necessary for making routing decisions.
#[derive(TryFromBytes, Debug)]
#[repr(C)]
pub struct DirectedEdge {
    first_bitfield: FirstBitfield,
    second_bitfield: SecondBitfield,
    third_bitfield: ThirdBitfield,
    fourth_bitfield: FourthBitfield,
    fifth_bitfield: FifthBitfield,
    stop_impact_or_line: StopOrLine,
    seventh_bitfield: SeventhBitField,
}

impl DirectedEdge {
    // TODO: Dozens of access helpers :)

    /// Is this a transit line (buss or rail)?
    #[inline]
    pub fn end_node_id(&self) -> GraphId {
        // Safety: We know the number of bits is limited
        unsafe { GraphId::from_id_unchecked(self.first_bitfield.end_node()) }
    }

    /// The way the edge is used.
    ///
    /// # Panics
    ///
    /// This function currently panics if the edge use contains an invalid bit pattern.
    /// We should check the validity of the field when we do the initial try_transmute
    /// on the type (and then have an error result that the tile is invalid).
    /// There are some upstream bugs though between zerocopy and bitfield-struct
    /// macros: https://github.com/google/zerocopy/issues/388.
    #[inline]
    pub fn edge_use(&self) -> RoadUse {
        try_transmute!(self.third_bitfield.edge_use())
            .map_err(|_| GraphTileError::ValidityError)
            .expect("Invalid bit pattern for edge use.")
    }

    /// Is this a transit line (buss or rail)?
    ///
    /// # Panics
    ///
    /// Can panic as a result of the issues noted in [`Self::edge_use`].
    #[inline]
    pub fn is_transit_line(&self) -> bool {
        let edge_use = self.edge_use();
        edge_use == RoadUse::Rail || edge_use == RoadUse::Bus
    }

    /// Gets the index of the opposing directed edge at the end node of this directed edge.
    ///
    /// Can be used to find the start node of this directed edge.
    #[inline]
    pub fn opposing_edge_index(&self) -> u32 {
        self.first_bitfield.opposing_edge_index()
    }

    /// Is this edge a shortcut?
    #[inline]
    pub fn is_shortcut(&self) -> bool {
        self.seventh_bitfield.is_shortcut() != 0
    }
}

/// Extended directed edge attributes.
///
/// This structure provides the ability to add extra
/// attributes to directed edges without breaking backward compatibility.
/// For now this structure is unused
#[derive(FromBytes, Debug)]
#[repr(C)]
pub struct DirectedEdgeExt(u64);

#[cfg(test)]
mod test {
    use crate::graph_tile::TEST_GRAPH_TILE;

    #[test]
    fn test_parse_directed_edges_count() {
        let tile = &*TEST_GRAPH_TILE;

        assert_eq!(
            tile.directed_edges.len(),
            tile.header.directed_edge_count() as usize
        );
    }

    #[test]
    fn test_parse_nodes() {
        let tile = &*TEST_GRAPH_TILE;

        insta::assert_debug_snapshot!(tile.directed_edges[0]);
        insta::assert_debug_snapshot!(tile.directed_edges.last().unwrap());

        // TODO: Other sanity checks after we add some more advanced methods
    }
}
