# Edge Exporter

This tool exports data from a Valhalla tile graph.
The intended use case is vector tile generation.
We currently accomplish that by dumping to an intermediate format and tiling with tippecanoe.

## Creating tiles from the export

### Dense tiles

```shell
tippecanoe -Z3 -z12 -P -o valhalla.pmtiles --drop-smallest-as-needed --extend-zooms-if-still-dropping tiles-json/**.geojson
```

### Sparse

I tried `-at`, which generated much smaller tiles, but it looks like the PMTiles web viewer doesn't handle this well.
This probably isn't an issue for the Stadia Maps tile server.

```shell
tippecanoe -at -P -o valhalla.pmtiles --extend-zooms-if-still-dropping tiles-json/**.geojson
```

## Open threads

* GeoJSON is a terrible serialization format; it's massively bloated. Can we use something better?
  - Tippecanoe supports FlatGeoBuf, but not GeoParquet
  - Tippecanoe can accept input from stdin, but does it have to read everything? If so, the RAM requirement will be a problem.
  - Could we just dump GeoParquet files and work with those?
    - Would be a heck of a lot better write throughput, and bypass the whole step
    - GeoParquet also supports a wider range of types
    - Could *not* use Tippecanoe effectively AFAIK
    - [parquet-wasm](https://github.com/kylebarron/parquet-wasm) is a thing... looks like a fair bit of work though
    - Something like [this](https://github.com/sfomuseum/go-geoparquet-show)?
    - Parquet backend to Stadia Maps tile server seems reasonable for our own prod deployment