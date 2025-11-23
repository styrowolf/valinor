use std::collections::BinaryHeap;

use geo::{coord, point, BoundingRect, Distance, Euclidean, HasDimensions, Haversine, Intersects, LineString};
use valhalla_graphtile::{tile_hierarchy::STANDARD_LEVELS, tile_provider::{DirectoryTileProvider, GraphTileProvider}, GraphId};

fn main() {
    bins();
    return;
    let provider = DirectoryTileProvider::new("/home/oguzkurt/data/valhalla_tiles".into(), 32.try_into().unwrap());
    let id = GraphId::try_from_id(392653986720).unwrap();
    let tile = provider.get_tile_containing(&id).unwrap();

    let corner = tile.header.sw_corner();
    let side_length = match tile.graph_id().level() {
        0 => 4.0,
        1 => 1.0,
        2 => 0.25,
        _ => panic!("wtf is this level"),
    };
    let tile_rect = geo::Rect::new(
        corner,
            coord! { x: corner.x + side_length, y: corner.y + side_length }
    );

    let a = tile.header.sw_corner();

    let edge = tile.get_directed_edge(&id).unwrap();
    let end_node = tile.get_node(&edge.end_node_id()).unwrap();
    let start_index = end_node.edge_index();
    println!("Out edges from end node of {}", id);
    for i in 0..end_node.edge_count() {
        let new_id = GraphId::try_from_components(edge.end_node_id().level(), edge.end_node_id().tile_id(), start_index as u64 + i as u64).unwrap();
        let edge = tile.get_edge_info(&tile.get_directed_edge(&new_id).unwrap()).unwrap();
        
        println!("{}", new_id)
    }
    
}

fn get_tile_index_from_latlon(level: u8, lon: f32, lat: f32) -> GraphId {
    assert!(level <= 2);

    let levels = STANDARD_LEVELS.as_slice();
    let ts = &levels[level as usize].tiling_system;
    
    let num_columns = ((lon + 180.0) / ts.tile_size).round() as u64;
    let num_rows = ((lat + 90.0) / ts.tile_size).round() as u64;

    return GraphId::try_from_components(level, num_rows * ts.n_cols as u64 + num_columns, 0_u64).unwrap();
}

fn tiles_for_bbox(rect: geo::Rect<f32>) -> Vec<GraphId> {
    let levels = STANDARD_LEVELS.as_slice();
    let (left, bottom, right, top) = (rect.min().x, rect.min().y, rect.max().x, rect.max().y);
    
    assert!(-90.0 <= bottom && bottom <= 90.0);
    assert!(-90.0 <= top && top <= 90.0);
    assert!(bottom <= top);
    assert!(-180.0 <= left && left <= 180.0);
    assert!(-180.0 <= right && right <= 180.0);

    // TODO: determine if anti-meridian crossing is possible with geo::Rect
    // i don't think so

    let mut tiles = Vec::new();
    for level in 0..=2 {
        let tile_size = levels[level].tiling_system.tile_size;
        let col_start = ((left + 180.0) / tile_size).round() as i32;
        let col_end = ((right + 180.0) / tile_size).round() as i32;
        let row_start = ((bottom + 90.0) / tile_size).round() as i32;
        let row_end = ((top + 90.0) / tile_size).round() as i32;

        for col in col_start..=col_end {
            for row in row_start..=row_end {
                let tile_index = (row as f32 * (360.0 / tile_size) + col as f32).round() as u64;
                tiles.push(unsafe { GraphId::from_components_unchecked(level as u8, tile_index, 0) });
            }
        }
    }

    return tiles
}

#[test]
fn test_tiles_for_bbox() {
    let ny_rect = geo::Rect::new(
        coord! { x: -74.25196, y: 40.51276 },
        coord! { x: -73.75540, y: 40.90312 },
    );

    let mut tiles = tiles_for_bbox(ny_rect);
    tiles.sort_by(|a, b| {
        a.level().cmp(&b.level()).then(a.tile_id().cmp(&b.tile_id()))
    });
    tiles.iter().for_each(|id| println!("{}", id));
}

fn dijkstra(provider: &DirectoryTileProvider, start_edge_id: GraphId, end_edge_id: GraphId) -> f64 {
    let start_tile = provider.get_tile_containing(&start_edge_id).unwrap();
    let end_tile = provider.get_tile_containing(&end_edge_id).unwrap();

    if start_tile.graph_id().level() != end_tile.graph_id().level() {
        panic!("Start and end edges must be in the same tile level");
    }

    let mut pq = BinaryHeap::new();

    struct State {
        distance: f32,
        edge: GraphId,
    }

    impl Ord for State {
        // to prioritize shorter distances
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.partial_cmp(&other).unwrap()
        }
    }

    impl PartialOrd for State {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            other.distance.partial_cmp(&self.distance)
        }
    }

    impl PartialEq for State {
        fn eq(&self, other: &Self) -> bool {
            self.distance == other.distance && self.edge == other.edge
        }
    }
    impl Eq for State {}

    pq.push(State {
        distance: 0.0,
        edge: start_edge_id,
    });

    while let Some(s) = pq.pop() {
        let current_tile = provider.get_tile_containing(&s.edge).unwrap();
        let current_edge = current_tile.get_directed_edge(&s.edge).expect("Current edge not found");
        let end_node_id = current_edge.end_node_id();
        let end_node = current_tile.get_node(&end_node_id).expect("End node not found");
        for i in 0..end_node.edge_count() {
            let new_edge_id = GraphId::try_from_components(
                end_node_id.level(),
                end_node_id.tile_id(),
                end_node.edge_index() as u64 + i as u64,
            ).expect("Failed to create new edge ID");

            let new_edge = current_tile.get_directed_edge(&new_edge_id).expect("New edge not found");
            let new_distance = s.distance + new_edge.length() as f32;

            if new_edge_id == end_edge_id {
                return new_distance as f64; // Return the distance when we reach the end edge
            }

            pq.push(State {
                distance: new_distance,
                edge: new_edge_id,
            });
        }
    }



    // Implement Dijkstra's algorithm here
    // This is a placeholder for the actual implementation
    0.0 // Return the distance found

}

#[test]
fn candidate() {
    let provider = DirectoryTileProvider::new("/home/oguzkurt/data/valhalla_tiles".into(), 32.try_into().unwrap());
    let p = point! { x: 29.117010, y: 41.011928 };
    let tile_id = get_tile_index_from_latlon(2, p.x() as f32, p.y() as f32);
    println!("{}", tile_id);
    let tile = provider.get_tile_containing(&tile_id).unwrap();
    for i in 0..tile.header.directed_edge_count() {
        let edge = tile.get_directed_edge(&GraphId::try_from_components(tile_id.level(), tile_id.tile_id(), i as u64).unwrap()).unwrap();
        let edge_info = tile.get_edge_info(&edge).unwrap();
        let ls = edge_info.shape().unwrap(); //ls.boundary_dimensions()
        let dist = Euclidean::distance(&p, ls);

        print!("{dist} ");
    }
}


fn bins() {
    let provider = DirectoryTileProvider::new("/home/oguzkurt/data/valhalla_tiles".into(), 32.try_into().unwrap());
    let p = point! { x: 29.117010, y: 41.011928 };
    let tile_id = get_tile_index_from_latlon(2, p.x() as f32, p.y() as f32);
    println!("{}", tile_id.value());
    let tile = provider.get_tile_containing(&tile_id).unwrap();
    let bin = tile.get_bin(0);

    //println!("Bin starts at byte offset {}", tile.get_bins_start());

    for i in 0..25 {
        println!("Bin {} with {} edges", i, tile.get_bin(i).len());
        //println!("Offset: {:?}", tile.offset_pair(i));
    }
}