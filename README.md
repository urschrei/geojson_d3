# `GeoJSON_D3`

# Introduction
D3 expects the rings of input Polygons in [a different order](https://github.com/d3/d3-geo/pull/79) than the [GeoJSON RFC 7946](https://tools.ietf.org/html/rfc7946#section-3.1.6) specification. This can lead to unexpected errors. This binary provides a conversion function for GeoJSON `RFC 7946` files containing Polygons and / or MultiPolygons. All other geometry types are left untouched.

## Installation
Install it using `cargo install geojson_d3`, or download a [binary](#binaries) and put it on your $PATH.  
This provides the `geojson_d3` command.

## Use
`geojson_d3` takes one mandatory argument: a file containing valid GeoJSON. Polygon and / or MultiPolygon geometries can be included as a `Feature,` or a `Geometry`, or a`FeatureCollection` or `GeometryCollection` – you may also mix the two geometries in a `FeatureCollection` or `GeometryCollection`.

- Processing of nested `GeometryCollection`s is supported, [but you shouldn't be using those](https://tools.ietf.org/html/rfc7946#section-3.1.8)
- Empty geometries or collections will be left unaltered
- Geometries which are already in "`D3`" format will be left unaltered

You may also pass:
- `-p` or `--pretty`, which will pretty-print the GeoJSON output
- `-s` or `--stats-only`, which will output the number of labelled polygons, but will *not* output GeoJSON.

## Progress
If you aren't piping the output of the command to a file, `geojson_d3` will display progress of the parsing and processing steps in the terminal, as well as a final count of the processed (Multi)Polygons.

## Validity
While the structure of the input GeoJSON is validated, individual geometries are *not* validated in the DE-9IM sense. If they self-intersect, have open rings etc., results are not guaranteed to be correct.

## Speed
I haven't benchmarked it, and don't intend to. `Serde-JSON` is fast, though, and geometries are processed in parallel. Expect an order-of-magnitude improvement over similar JS-based tools.

## Binaries
Pre-built binaries are available from [releases](https://github.com/urschrei/geojson_d3/releases/latest). Binaries are available for:
- macOS (x86_64)
- Linux (x86_64)
- Windows (x86_64 and i686)

## License
[MIT](license.txt)
