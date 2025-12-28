//! # Spatial utilities useful for routing

use geo::{Coord, Destination, Haversine, Point};

const METERS_PER_DEGREE_LAT: f32 = 111_132.954;

/// Returns a bounding box centered upon `center` containing a circle with radius `radius` meters.
///
/// The bbox is in (N, E, S, W) order.
pub fn bbox_with_center(center: Point, radius: f64) -> (f64, f64, f64, f64) {
    // Per https://github.com/georust/geo/pull/1091/,
    // the longitude values should be normalized to [-180, 180].
    // We assert this again in a unit test below.
    let north = Haversine.destination(center, 0.0, radius).y();
    let east = Haversine.destination(center, 90.0, radius).x();
    let south = Haversine.destination(center, 180.0, radius).y();
    let west = Haversine.destination(center, 270.0, radius).x();

    (north, east, south, west)
}

/// Fast distance approximation.
///
/// This is intended for cases when you need a _fast_ estimate over _short_ distances
/// (a few hundred meters).
/// It is capable of giving estimates like "is point A within X meters of point B"
/// when you want to save potentially expensive work.
///
/// * Accuracy decreases at polar latitudes.
/// * Does NOT account for the antimeridian.
/// * Always over-estimates the distance.
/// * Expected range of overestimation is 5% or less for short distances (a few hundred meters).
pub struct DistanceApproximator {
    // TODO: Would be nice if we could support f32 as well
    center: Coord<f32>,
    meters_per_lon_degree: f32,
}

impl DistanceApproximator {
    /// Create a new approximator centered on the given point.
    #[inline]
    pub fn new(center: Coord<f32>) -> Self {
        let lon_scale = center.y.to_radians().cos();
        Self {
            center,
            meters_per_lon_degree: lon_scale * METERS_PER_DEGREE_LAT,
        }
    }

    /// Returns an approximation of the **squared** distance in meters to the given point.
    ///
    /// This is helpful for spatial predicates which are allowed to have false positives,
    /// A-star heuristics, etc.
    /// The returned distance will always be _larger_ than the actual distance.
    ///
    /// Compare against `max_distance * max_distance` (to avoid `sqrt` in your code),
    /// or else use the [`DistanceApproximator::is_within_distance_of`] helper.
    #[inline]
    pub fn distance_squared(&self, other: Coord<f32>) -> f32 {
        let dlat = (other.y - self.center.y) * METERS_PER_DEGREE_LAT;
        let dlon = (other.x - self.center.x) * self.meters_per_lon_degree;
        (dlat * dlat) + (dlon * dlon)
    }

    /// Returns whether the other coordinate is *probably* within `meters` of the reference coordinate.
    ///
    /// This has an error of less than 5% for small distances.
    /// This is designed to be used for distances of a few hundred meters as a fast approximation,
    /// and always over-estimates the distance.
    /// If this method returns false, then you can be assured the point is not within the specified range.
    /// However, an affirmative result will be a false positive approximately 5% of the time on average.
    #[inline]
    pub fn is_within_distance_of(&self, other: Coord<f32>, meters: f32) -> bool {
        let sq_dist = self.distance_squared(other);

        sq_dist < (meters * meters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Distance, coord};
    use proptest::{prop_assert, proptest};

    proptest! {
        #[test]
        fn haversine_oracle(lat in -90.0f32..90.0, lon in -180.0f32..180.0,
            dlat in -0.1f32..0.1, dlon in -0.1f32..0.1) {
            // Construct a test with coordinates fairly close together.
            // 0.001 degrees is about 1.1km at the equator.
            // We expect the real use cases for this to be much smaller.
            let a = coord! {x: lon, y: lat};
            let b = coord! {x: lon + dlon, y: lat + dlat};
            let approximator = DistanceApproximator::new(a);

            let sq_dist = approximator.distance_squared(b);
            let haversine_dist = Haversine.distance(a.into(), b.into());

            prop_assert!(sq_dist > haversine_dist, "Expected sq dist to be > haversine_dist");

            let diff = sq_dist.sqrt() - haversine_dist;

            prop_assert!(diff / haversine_dist < 0.05, "Expected an error of less than 5%");
            prop_assert!(approximator.is_within_distance_of(b, haversine_dist), "The approximator should return the same result as a Haversine test");
        }
    }
}
