use geo::LineString;
use num_traits::identities::Zero;
use serde::Serialize;
use std::rc::Rc;
use valhalla_graphtile::graph_tile::{DirectedEdge, EdgeInfo, GraphTile};
use valhalla_graphtile::tile_hierarchy::TileLevel;
use valhalla_graphtile::{GraphId, RoadClass, RoadUse};

// TODO: Do we need this?
pub struct EdgePointer<'a> {
    pub graph_id: GraphId,
    pub tile: Rc<GraphTile<'a>>,
}

impl EdgePointer<'_> {
    pub(crate) fn edge(&self) -> &DirectedEdge {
        self.tile
            .get_directed_edge(&self.graph_id)
            .expect("That wasn't supposed to happen...")
    }
}

#[derive(Serialize)]
struct Tippecanoe {
    layer: &'static str,
    #[serde(rename = "minzoom")]
    min_zoom: u32,
}

impl From<&TileLevel> for Tippecanoe {
    fn from(level: &TileLevel) -> Self {
        Self {
            layer: level.name,
            min_zoom: level.tiling_system.min_zoom(),
        }
    }
}

#[derive(Serialize)]
struct Geometry {
    #[serde(rename = "type")]
    geometry_type: &'static str,
    coordinates: Vec<[f32; 2]>,
}

impl From<&LineString> for Geometry {
    fn from(value: &LineString) -> Self {
        Self {
            geometry_type: "LineString",
            // TODO: Truncate to 6 digits
            coordinates: value
                .coords()
                .map(|coord| [coord.x as f32, coord.y as f32])
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct EdgeProperties {
    // NOTE: We can't store an array in MVT
    names: String,
    classification: RoadClass,
    // TODO: Directionality (forward/reverse)
    // I don't know what forward means
    forward: bool,
    forward_access: String,
    reverse_access: String,
    // TODO: Bike network
    // TODO: Estimated speed
    #[serde(skip_serializing_if = "u8::is_zero")]
    speed_limit: u8,
    #[serde(rename = "use")]
    road_use: RoadUse,
    // TODO: Cycle lane
    // TODO: Sidewalk
    // TODO: Use sidepath
    // TODO: More TODOs...
}

impl EdgeProperties {
    fn new(names: String, edge: &DirectedEdge, edge_info: &EdgeInfo) -> Self {
        EdgeProperties {
            names,
            classification: edge.classification(),
            forward: edge.forward(),
            forward_access: edge
                .forward_access()
                .iter()
                .map(|v| v.as_char())
                .collect::<String>(),
            reverse_access: edge
                .reverse_access()
                .iter()
                .map(|v| v.as_char())
                .collect::<String>(),
            speed_limit: edge_info.speed_limit(),
            road_use: edge.edge_use(),
        }
    }
}

#[derive(Serialize)]
pub struct EdgeRecord {
    // TODO: Figure out how to make this a default?
    #[serde(rename = "type")]
    record_type: &'static str,
    tippecanoe: Tippecanoe,
    geometry: Geometry,
    properties: EdgeProperties,
}

impl EdgeRecord {
    pub fn new(
        tile_level: &TileLevel,
        line_string: &LineString,
        names: String,
        edge: &DirectedEdge,
        edge_info: &EdgeInfo,
    ) -> Self {
        Self {
            record_type: "Feature",
            tippecanoe: Tippecanoe::from(tile_level),
            geometry: Geometry::from(line_string),
            properties: EdgeProperties::new(names, edge, edge_info),
        }
    }
}
