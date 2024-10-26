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
    min_zoom: u8,
}

impl From<(&TileLevel, RoadClass)> for Tippecanoe {
    fn from((level, road_class): (&TileLevel, RoadClass)) -> Self {
        Self {
            layer: level.name,
            min_zoom: std::cmp::min(12, level.tiling_system.min_zoom() + match &level.minimum_road_class.discriminant() - road_class.discriminant() {
                0 => 2,
                1 => 1,
                2.. => 0
            }),
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
struct EdgeProperties<'a> {
    // NOTE: We can't store an array in MVT
    names: String,
    edge: &'a DirectedEdge,
    // TODO: Use an EdgeInfo and impl Serialize on that
    #[serde(skip_serializing_if = "u8::is_zero")]
    speed_limit: u8,
}

impl EdgeProperties<'_> {
    fn new<'a>(names: String, edge: &'a DirectedEdge, edge_info: &'a EdgeInfo<'a>) -> EdgeProperties<'a> {
        EdgeProperties {
            names,
            edge,
            speed_limit: edge_info.speed_limit(),
        }
    }
}

#[derive(Serialize)]
pub struct EdgeRecord<'a> {
    // TODO: Figure out how to make this a default?
    #[serde(rename = "type")]
    record_type: &'static str,
    tippecanoe: Tippecanoe,
    geometry: Geometry,
    properties: EdgeProperties<'a>,
}

impl EdgeRecord<'_> {
    pub fn new<'a>(
        tile_level: &TileLevel,
        line_string: &LineString,
        names: String,
        edge: &'a DirectedEdge,
        edge_info: &'a EdgeInfo,
    ) -> EdgeRecord<'a> {
        EdgeRecord {
            record_type: "Feature",
            tippecanoe: Tippecanoe::from((tile_level, edge.classification())),
            geometry: Geometry::from(line_string),
            properties: EdgeProperties::new(names, edge, edge_info),
        }
    }
}
