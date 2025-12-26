use flatgeobuf::{FgbWriter, GeometryType};
use geozero::error::GeozeroError;
use geozero::{ColumnValue, FeatureProcessor, GeomProcessor, PropertyProcessor};
use serde_json::json;
use std::io::Write;
use valhalla_graphtile::GraphId;
use valhalla_graphtile::graph_tile::{DirectedEdge, EdgeInfo};
use valhalla_graphtile::tile_hierarchy::STANDARD_LEVELS;

/// A streaming FlatGeobuf writer for directed edges.
pub struct StreamingEdgeWriter<'a> {
    fgb: FgbWriter<'a>,
    next_fid: u64,
}

impl<'a> StreamingEdgeWriter<'a> {
    /// Create a new writer with the given layer name.
    pub fn new(layer_name: &'a str) -> anyhow::Result<Self> {
        let fgb = FgbWriter::create(layer_name, GeometryType::LineString)?;
        Ok(Self { fgb, next_fid: 0 })
    }

    fn prop_bool(
        &mut self,
        colname: &str,
        value: bool,
        pidx: usize,
    ) -> Result<usize, GeozeroError> {
        self.fgb
            .property(pidx, colname, &ColumnValue::Bool(value))?;
        Ok(pidx + 1)
    }

    fn prop_u8(&mut self, colname: &str, value: u8, pidx: usize) -> Result<usize, GeozeroError> {
        self.fgb
            .property(pidx, colname, &ColumnValue::UByte(value))?;
        Ok(pidx + 1)
    }

    fn prop_u64(&mut self, colname: &str, value: u64, pidx: usize) -> Result<usize, GeozeroError> {
        self.fgb
            .property(pidx, colname, &ColumnValue::ULong(value))?;
        Ok(pidx + 1)
    }

    fn prop_string(
        &mut self,
        colname: &str,
        value: &str,
        pidx: usize,
    ) -> Result<usize, GeozeroError> {
        self.fgb
            .property(pidx, colname, &ColumnValue::String(value))?;
        Ok(pidx + 1)
    }

    fn prop_json(
        &mut self,
        colname: &str,
        value: &str,
        pidx: usize,
    ) -> Result<usize, GeozeroError> {
        self.fgb
            .property(pidx, colname, &ColumnValue::Json(value))?;
        Ok(pidx + 1)
    }

    /// Writes a directed edge.
    pub fn write_feature(
        &mut self,
        edge_id: GraphId,
        edge: &DirectedEdge,
        edge_info: &EdgeInfo,
        write_tippecanoe_properties: bool,
    ) -> anyhow::Result<()> {
        // Begin feature
        self.fgb.feature_begin(self.next_fid)?;

        // Properties
        self.fgb.properties_begin()?;

        // Identifiers
        let pidx = self.prop_u64("edge_id", edge_id.value(), 0)?;
        let pidx = self.prop_u64("osm_way_id", edge_info.way_id(), pidx)?;
        let pidx = self.prop_string("names", &edge_info.get_names().join("; "), pidx)?;

        // Tippecanoe meta-properties for better tile export

        let pidx = if write_tippecanoe_properties {
            let level = STANDARD_LEVELS.get(usize::from(edge_id.level()));
            let zoom = level.map_or(12, |level| level.tiling_system.min_zoom());
            let layer = level.map_or("other", |level| level.name);
            let json_str = serde_json::to_string(&json!({
                "minzoom": zoom,
                "layer": layer,
            }))?;

            self.prop_json("tippecanoe", &json_str, pidx)?
            // let pidx = self.prop_u8("minzoom", , pidx)?;
            // self.prop_string("layer", , pidx)?
        } else {
            pidx
        };

        // Classification
        let pidx = self.prop_string("road_class", &format!("{:?}", edge.classification()), pidx)?;
        let pidx = self.prop_string("use", &format!("{:?}", edge.road_use()), pidx)?;

        // Mode-based access bits (stringified for compact readability)
        let pidx = self.prop_string(
            "forward_access",
            &edge
                .forward_access()
                .iter()
                .map(|v| v.as_char())
                .collect::<String>(),
            pidx,
        )?;
        let pidx = self.prop_string(
            "reverse_access",
            &edge
                .reverse_access()
                .iter()
                .map(|v| v.as_char())
                .collect::<String>(),
            pidx,
        )?;

        // Access restrictions
        let pidx = self.prop_bool("dest_only", edge.dest_only(), pidx)?;
        let pidx = self.prop_bool("no_thru", edge.no_thru(), pidx)?;
        // TODO: Other restrictions (`access_restrictions`, `start_restriction`, `end_restriction`, and `complex_restrictions`) once we understand their usage + model them with nicer accessors

        // Speed
        let pidx = self.prop_u8("speed", edge.speed(), pidx)?;
        let pidx = self.prop_u8("free_flow_speed", edge.free_flow_speed(), pidx)?;
        let pidx = self.prop_u8(
            "constrained_flow_speed",
            edge.constrained_flow_speed(),
            pidx,
        )?;
        let pidx = self.prop_bool("has_predicted_speed", edge.has_predicted_speed(), pidx)?;
        let pidx = self.prop_u8("speed_limit", edge_info.speed_limit(), pidx)?;

        // Other attributes
        let pidx = self.prop_u8("lane_count", edge.lane_count(), pidx)?;
        let pidx = self.prop_u8("density", edge.density(), pidx)?;
        let pidx = self.prop_string("surface", &format!("{:?}", edge.surface()), pidx)?;
        let pidx = self.prop_bool("country_crossing", edge.country_crossing(), pidx)?;
        let pidx = self.prop_bool("toll", edge.toll(), pidx)?;
        let pidx = self.prop_bool("roundabout", edge.roundabout(), pidx)?;
        let pidx = self.prop_string(
            "bicycle_network",
            &edge_info
                .bicycle_network()
                .iter()
                .map(|v| v.as_char())
                .collect::<String>(),
            pidx,
        )?;
        let pidx = self.prop_bool("truck_route", edge.truck_route(), pidx)?;
        let pidx = self.prop_bool(
            "is_internal_intersection",
            edge.is_intersection_internal(),
            pidx,
        )?;
        let _pidx = self.prop_bool("is_shortcut", edge.is_shortcut(), pidx)?;

        self.fgb.properties_end()?;

        // Geometry (LineString)
        let mut coords = edge_info.decode_raw_shape()?;
        if !edge.edge_info_is_forward() {
            coords.reverse();
        }

        self.fgb.geometry_begin()?;
        self.fgb.linestring_begin(false, coords.len(), 0)?;
        for coord in coords {
            self.fgb.xy(coord.x, coord.y, 0)?;
        }
        self.fgb.linestring_end(false, 0)?;
        self.fgb.geometry_end()?;

        // End feature
        self.fgb.feature_end(self.next_fid)?;
        self.next_fid += 1;
        Ok(())
    }

    /// Finalize and write the FlatGeobuf stream to the provided writer.
    pub fn finalize<W: Write>(self, mut w: W) -> anyhow::Result<()> {
        self.fgb.write(&mut w)?;
        Ok(())
    }
}
