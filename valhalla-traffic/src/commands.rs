use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, BufRead, Write},
    path::PathBuf,
};

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use valhalla_graphtile::GraphId;

use crate::{
    build_live_traffic_data, build_tiles,
    encoding::{K_BUCKETS_PER_WEEK, compress_speed_buckets, encode_compressed_speeds},
    update_tiles,
    utils::GraphIdExt,
};

pub fn dct_ii(pargs: &mut pico_args::Arguments) -> anyhow::Result<()> {
    let speeds_file: Option<PathBuf> = pargs.opt_value_from_str("--file")?;
    let speed: Option<String> = pargs.opt_value_from_str("--speed")?;
    let use_stdin = pargs.contains("--stdin");
    let speeds_entries = if use_stdin {
        std::io::stdin()
            .lock()
            .lines()
            .map(|line| line.unwrap())
            .collect::<Vec<_>>()
    } else if let Some(file) = speeds_file {
        let file = File::open(file)?;
        io::BufReader::new(file)
            .lines()
            .map(|line| line.unwrap())
            .collect::<Vec<_>>()
    } else if let Some(speed) = speed {
        vec![speed]
    } else {
        panic!("Either --file or --speed must be provided, or use --stdin to read from stdin");
    };
    let encoded_strs = speeds_entries
        .into_par_iter()
        .map(|e| {
            let parts = e.split(',');
            let speeds = parts.map(|s| s.parse().unwrap()).collect::<Vec<f32>>();
            if speeds.len() != K_BUCKETS_PER_WEEK {
                panic!(
                    "Expected {} speed buckets, got {}",
                    K_BUCKETS_PER_WEEK,
                    speeds.len()
                );
            }
            encode_compressed_speeds(&compress_speed_buckets(speeds.as_slice()))
        })
        .collect::<Vec<String>>();

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    for encoded in encoded_strs {
        writeln!(handle, "{}", encoded)?;
    }
    std::process::exit(0);
}

pub fn make_traffic_dir(pargs: &mut pico_args::Arguments) -> anyhow::Result<()> {
    let path: PathBuf = pargs.value_from_str("--csv")?;
    let dir_path: PathBuf = pargs.value_from_str("--dir")?;
    let file = File::open(path)?;
    let mut rdr = csv::Reader::from_reader(file);

    let mut dir_map: HashMap<PathBuf, Vec<csv::StringRecord>> = HashMap::new();

    for record in rdr.records() {
        let record = record?;
        let id = GraphId::from_hierarchical_string(&record[0])?;
        let filepath = id.file_path("csv")?;

        dir_map
            .entry(filepath)
            .and_modify(|e| e.push(record.clone()))
            .or_insert(vec![record]);
    }

    for (path, records) in dir_map {
        let path = dir_path.join(path);
        if !path.exists() {
            fs::create_dir_all(&path.parent().unwrap())?;
        }
        println!("Writing to {:?}", path);
        let file = File::create(path)?;
        let mut wtr = csv::Writer::from_writer(file);
        for record in records {
            wtr.write_record(&record)?;
        }
        wtr.flush()?;
    }

    std::process::exit(0);
}

pub fn make_dummy_live_traffic_tar(pargs: &mut pico_args::Arguments) -> anyhow::Result<()> {
    let tile_dir: PathBuf = pargs.value_from_str("--tile_dir")?;
    let traffic_extract: PathBuf = pargs.value_from_str("--traffic_extract")?;
    let tile_id_str: String = pargs.value_from_str("--tile_id")?;
    let tile_id = GraphId::from_hierarchical_string(&tile_id_str)?;
    let traffic_update_timestamp: u64 = pargs
        .value_from_str("--traffic_update_timestamp")
        .unwrap_or_else(|_| 0);
    build_live_traffic_data(
        &tile_dir,
        &traffic_extract,
        tile_id,
        0,
        traffic_update_timestamp,
    )?;

    Ok(())
}

pub fn make_live_traffic_tar(pargs: &mut pico_args::Arguments) -> anyhow::Result<()> {
    let csv_path: PathBuf = pargs.value_from_str("--csv")?;
    let tile_dir: PathBuf = pargs.value_from_str("--tile_dir")?;
    let traffic_extract: PathBuf = pargs.value_from_str("--traffic_extract")?;

    let mut rdr = csv::Reader::from_path(csv_path)?;
    let mut tiles_traffic_map: HashMap<GraphId, u32> = HashMap::new();
    for record in rdr.records() {
        let record = record?;
        //let id = GraphId::from_hierarchical_string(&record[0])?;
        let id = GraphId::try_from_id(record[0].parse()?)?;
        let speed: u32 = record[1].parse()?;
        tiles_traffic_map.insert(id, speed);
    }

    build_tiles(
        HashMap::new(),
        &tile_dir,
        &traffic_extract,
        pargs
            .value_from_str("--traffic_update_timestamp")
            .unwrap_or_else(|_| 0),
    )?;

    Ok(())
}

pub fn update_live_traffic_tar(pargs: &mut pico_args::Arguments) -> anyhow::Result<()> {
    let csv_path: PathBuf = pargs.value_from_str("--csv")?;
    let traffic_extract: PathBuf = pargs.value_from_str("--traffic_extract")?;
    let traffic_update_timestamp: u64 = pargs
        .value_from_str("--traffic_update_timestamp")
        .unwrap_or_else(|_| 0);

    let mut rdr = csv::Reader::from_path(csv_path)?;
    let mut tiles_traffic_map: HashMap<GraphId, u32> = HashMap::new();
    for record in rdr.records() {
        let record = record?;
        //let id = GraphId::from_hierarchical_string(&record[0])?;
        let id = GraphId::try_from_id(record[0].parse()?)?;
        let speed: u32 = record[1].parse()?;
        tiles_traffic_map.insert(id, speed);
    }

    update_tiles(
        tiles_traffic_map,
        &traffic_extract,
        traffic_update_timestamp,
    )?;

    Ok(())
}
