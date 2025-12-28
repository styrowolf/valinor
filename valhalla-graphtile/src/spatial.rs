//! # Spatial utilities useful for routing

use geo::{Coord, CoordFloat, Destination, Haversine, Point};
use num_traits::FromPrimitive;

const METERS_PER_DEGREE_LAT: f64 = 111_132.954;

/// Returns a bounding box centered upon `center` containing a circle with radius `radius` meters.
///
/// The bbox is in (N, E, S, W) order.
pub fn bbox_with_center<F: CoordFloat + FromPrimitive>(
    center: Point<F>,
    radius: F,
) -> (F, F, F, F) {
    // Per https://github.com/georust/geo/pull/1091/,
    // the longitude values should be normalized to [-180, 180].
    // We assert this again in a unit test below.
    // These unwraps cannot fail
    let north = Haversine.destination(center, F::zero(), radius).y();
    let east = Haversine
        .destination(center, F::from_i64(90).unwrap(), radius)
        .x();
    let south = Haversine
        .destination(center, F::from_i64(180).unwrap(), radius)
        .y();
    let west = Haversine
        .destination(center, F::from_i64(270).unwrap(), radius)
        .x();

    (north, east, south, west)
}

/// Fast distance approximation.
///
/// This is intended for cases when you need a _fast_ estimate over _short_ distances
/// (up to a few kilometers).
/// It is capable of giving estimates like "is point A within X meters of point B"
/// when you want to save potentially expensive trigonometry, especially at scale.
///
/// # Limitations
///
/// * Accuracy decreases at polar latitudes.
/// * Does NOT account for the antimeridian.
/// * Expected range of overestimation is less than 1m for short distances (up to a few kilometers),
///   but will get worse over larger distances, and closer to the poles.
pub struct DistanceApproximator<F: CoordFloat + FromPrimitive> {
    center: Coord<F>,
    meters_per_lon_degree: F,
    meters_per_lat_degree: F,
}

impl<F: CoordFloat + FromPrimitive> DistanceApproximator<F> {
    /// Create a new approximator centered on the given point.
    #[inline]
    pub fn new(center: Coord<F>) -> Self {
        let lon_scale = center.y.to_radians().cos();
        let meters_per_lat_degree = F::from(METERS_PER_DEGREE_LAT).unwrap();
        Self {
            center,
            meters_per_lon_degree: lon_scale * meters_per_lat_degree,
            meters_per_lat_degree,
        }
    }

    /// Returns an approximation of the **squared** distance in meters to the given point.
    ///
    /// This is helpful for spatial predicates which are allowed to over-estimate,
    /// including some sorts of filters, A-star heuristics, etc.
    /// The returned distance will always be _larger_ than the actual distance.
    ///
    /// Compare against `max_distance * max_distance` (to avoid `sqrt` in your code),
    /// or else use the [`DistanceApproximator::is_probably_within_distance_of`] helper.
    #[inline]
    pub fn distance_squared(&self, other: Coord<F>) -> F {
        let dlat = (other.y - self.center.y) * self.meters_per_lat_degree;
        let dlon = (other.x - self.center.x) * self.meters_per_lon_degree;
        (dlat * dlat) + (dlon * dlon)
    }

    /// Returns whether the other coordinate is *probably* within `meters` of the reference coordinate.
    ///
    /// See the [`DistanceApproximator`] docs for more details on the limitations.
    #[inline]
    pub fn is_probably_within_distance_of(&self, other: Coord<F>, meters: F) -> bool {
        debug_assert!(
            meters < F::from(20_000).unwrap(),
            "A distance threshold greater than 20km is not a great idea."
        );

        let sq_dist = self.distance_squared(other);

        // Comparison that cleverly avoids sqrt (multiplication is cheap)
        sq_dist <= (meters * meters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Distance, coord};
    use num_traits::Zero;
    use proptest::{prop_assert, proptest};

    proptest! {
        #[test]
        fn haversine_oracle_f32(lat in -90.0f32..90.0, lon in -180.0f32..180.0,
            dlat in -0.1f32..0.1, dlon in -0.1f32..0.1) {
            // Construct a test with coordinates fairly close together.
            // 0.001 degrees is about 1.1km at the equator.
            // We expect the real use cases for this to be much smaller.
            let a = coord! {x: lon, y: lat};
            let b = coord! {x: lon + dlon, y: lat + dlat};
            let approximator = DistanceApproximator::new(a);

            let sq_dist = approximator.distance_squared(b);
            let haversine_dist = Haversine.distance(a.into(), b.into());

            prop_assert!(sq_dist >= haversine_dist, "Expected sq dist ({sq_dist}) to be >= haversine_dist ({haversine_dist})");

            let delta = sq_dist.sqrt() - haversine_dist;

            prop_assert!(delta < 1.0, "Expected a delta of less than 1m; was {delta}");
            prop_assert!(approximator.is_probably_within_distance_of(b, haversine_dist), "The approximator should return the same result as a Haversine test over short distances");
        }

        #[test]
        fn haversine_oracle_f64(lat in -90.0..90.0, lon in -180.0f64..180.0,
            dlat in -0.1..0.1, dlon in -0.1..0.1) {
            // Construct a test with coordinates fairly close together.
            // 0.001 degrees is about 1.1km at the equator.
            // We expect the real use cases for this to be much smaller.
            let a = coord! {x: lon, y: lat};
            let b = coord! {x: lon + dlon, y: lat + dlat};
            let approximator = DistanceApproximator::new(a);

            let sq_dist = approximator.distance_squared(b);
            let haversine_dist = Haversine.distance(a.into(), b.into());

            prop_assert!(sq_dist >= haversine_dist, "Expected sq dist to be > haversine_dist");

            let delta = sq_dist.sqrt() - haversine_dist;

            prop_assert!(delta < 1.0, "Expected a delta of less than 1m; was {delta}");
            prop_assert!(approximator.is_probably_within_distance_of(b, haversine_dist), "The approximator should return the same result as a Haversine test over short distances");
        }
    }
}
