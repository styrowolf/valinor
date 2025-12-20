# valinor-sif

A proof of concept for Valinor's costing trait, based on a simplified version of Valhalla's `sif::DynamicCost`. The goal of the POC is demonstrating that unidirectional routing (Djikstra and A*), location snapping (to be built later), as well as edge and turn costs work.

A sample route, routed by both the POC and Valhalla, can be found in the `sample_results` directory. You can import the GeoJSON files into [Placemark Play](https://play.placemark.io/) to view it. Note that since location snapping is not implemented yet, the POC uses an `GraphId` edge id as its origin; therefore, the geometry is routes do not start and end at the exact same points (though the difference is negligible).

To run the POC and get the output, use the ```cargo test -- --no-capture > out-idx``` command. You can manually remove the lines added to the stdout with an editor and then put it through `jq` to get a GeoJSONL file, which can be opened in Placemark Play.

```sh
cat out-idx | jq -c '{
  type: "Feature",
  properties: {
    edge_id: .edge_id,
    idx: .idx
  },
  geometry: {
    type: "LineString",
    coordinates: .shape | map([.[0], .[1]])
  }
}' > path.geojsonl
```
