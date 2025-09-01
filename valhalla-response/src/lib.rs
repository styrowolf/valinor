//! Data structures and helpers for generating Valhalla-compatible responses.

use serde::{Deserialize, Serialize};

/// A Valhalla status response including server version, capabilities, etc.
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
pub struct StatusResponse {
    /// The Valhalla server version (semver string).
    pub version: String,
    /// The UNIX timestamp (integer seconds) that the tileset was last modified.
    pub tileset_last_modified: u32,
    /// A list of actions that this server supports.
    ///
    /// These follow Valhalla action names.
    pub available_actions: Vec<String>,
    // Optional fields which may not always be present
    /// Whether a valid tileset is currently loaded.
    ///
    /// Only included in verbose responses.
    pub has_tiles: Option<bool>,
    /// Whether the current tileset was built with administrative boundaries.
    ///
    /// Only included in verbose responses.
    pub has_admins: Option<bool>,
    /// Whether the current tileset was built with timezones.
    ///
    /// Only included in verbose responses.
    pub has_timezones: Option<bool>,
    /// Whether live traffic tiles are currently available.
    ///
    /// Only included in verbose responses.
    pub has_live_traffic: Option<bool>,
    /// Whether transit tiles are currently available.
    ///
    /// Only included in verbose responses.
    pub has_transit_tiles: Option<bool>,
    /// The OSM changeset ID of the loaded tileset.
    ///
    /// Only included in verbose responses.
    pub osm_changeset: Option<u64>,
}
