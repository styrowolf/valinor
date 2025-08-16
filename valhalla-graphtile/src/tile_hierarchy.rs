use super::RoadClass;
use geo::{Rect, coord};
use std::sync::LazyLock;

/// A tiling system description.
///
/// Valhalla is relatively generic here,
/// though everything is relatively hard-coded to WGS84.
pub struct TilingSystem {
    /// The bounding box of the tiling system.
    pub bounding_box: Rect<f32>,
    /// The size of each side of a square tile.
    pub tile_size: f32,
    /// The number of rows in the tiling system.
    pub n_rows: u32,
    /// The number of columns in the tiling system.
    pub n_cols: u32,
    /// The number of subdivisions within a single tile.
    pub n_subdivisions: u8,
    /// Does the tiling system wrap in the x direction (e.g. at longitude = 180)?
    pub wrap_x: bool,
}

impl TilingSystem {
    fn new(bounding_box: Rect<f32>, tile_size: f32) -> Self {
        Self {
            bounding_box,
            tile_size,
            #[allow(clippy::cast_possible_truncation)]
            #[allow(clippy::cast_sign_loss)]
            n_rows: (bounding_box.height() / tile_size).round() as u32,
            #[allow(clippy::cast_possible_truncation)]
            #[allow(clippy::cast_sign_loss)]
            n_cols: (bounding_box.width() / tile_size).round() as u32,
            n_subdivisions: 5,
            wrap_x: true, // I've not seen this overridden in Valhalla so far...
        }
    }

    pub const fn tile_count(&self) -> u32 {
        self.n_rows * self.n_cols
    }

    pub const fn min_zoom(&self) -> u8 {
        if self.tile_size > 1.0 {
            0
        } else if self.tile_size > 0.25 {
            6
        } else {
            12
        }
    }
}

/// A level in the Valhalla tile hierarchy.
pub struct TileLevel {
    /// The hierarchy level.
    pub level: u8,
    /// The minimum class of road contained in this hierarchy level.
    pub minimum_road_class: RoadClass,
    /// The human-readable name of the level.
    pub name: &'static str,
    /// The tiling system used for this level.
    pub tiling_system: TilingSystem,
}

pub static STANDARD_LEVELS: LazyLock<[TileLevel; 3]> = LazyLock::new(|| {
    [
        TileLevel {
            level: 0,
            minimum_road_class: RoadClass::Primary,
            name: "highway",
            tiling_system: TilingSystem::new(
                Rect::new(
                    coord! { x: -180f32, y: -90f32 },
                    coord! { x: 180f32, y: 90f32 },
                ),
                4.0,
            ),
        },
        TileLevel {
            level: 1,
            minimum_road_class: RoadClass::Tertiary,
            name: "arterial",
            tiling_system: TilingSystem::new(
                Rect::new(
                    coord! { x: -180f32, y: -90f32 },
                    coord! { x: 180f32, y: 90f32 },
                ),
                1.0,
            ),
        },
        TileLevel {
            level: 2,
            minimum_road_class: RoadClass::ServiceOther,
            name: "local",
            tiling_system: TilingSystem::new(
                Rect::new(
                    coord! { x: -180f32, y: -90f32 },
                    coord! { x: 180f32, y: 90f32 },
                ),
                0.25,
            ),
        },
    ]
});

pub static TRANSIT_LEVEL: LazyLock<TileLevel> = LazyLock::new(|| TileLevel {
    level: 3,
    minimum_road_class: RoadClass::ServiceOther,
    name: "transit",
    tiling_system: TilingSystem::new(
        Rect::new(
            coord! { x: -180f32, y: -90f32 },
            coord! { x: 180f32, y: 90f32 },
        ),
        0.25,
    ),
});
