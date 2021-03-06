[![Linux / macOS Build Status](https://travis-ci.org/urschrei/polylabel_cmd.svg?branch=master)](https://travis-ci.org/urschrei/geojson_d3) [![Windows Build status](https://ci.appveyor.com/api/projects/status/pue3tmorkpdc560r/branch/master?svg=true)](https://ci.appveyor.com/project/urschrei/geojson-d3/branch/master)
 [![Crates Link](https://img.shields.io/crates/v/geojson_d3.svg)](https://crates.io/crates/geojson_d3)
# `GeoJSON_d3`

## Introduction
`d3-geo` uses spherical (as opposed to planar) polygons, and expects the rings of input polygons which are _smaller than a hemisphere_ to be oriented in [a different order](https://github.com/d3/d3-geo/pull/79) than the [GeoJSON RFC 7946](https://tools.ietf.org/html/rfc7946#section-3.1.6) specification, which can lead to unexpected errors. This binary checks the spherical area of RFC 7946-compliant Polygons, converting them to `d3-geo`-compliant ring orientation if necessary, or vice-versa.

## Installation
Install it using `cargo install geojson_d3`, or download a [binary](#binaries) and put it on your $PATH.  
This provides the `geojson_d3` command.

## Use
`geojson_d3` takes one mandatory argument: a file containing valid GeoJSON. Polygon and / or MultiPolygon geometries can be included as a `Feature,` or a `Geometry`, or a`FeatureCollection` or `GeometryCollection`. You may mix geometries in a `FeatureCollection` or `GeometryCollection`.

You may also pass:
- `-p` or `--pretty`, which will pretty-print the GeoJSON output
- `-s` or `--stats-only`, which will output the number of labelled polygons, but will *not* output GeoJSON
- `-r` or `--reverse`, which will _reverse_ the functionality, producing geometries with rings wound correctly according to RFC 7946.

- No assumptions are made concerning the existing winding order of Polygon rings:
    - By default, output will be in `d3-geo`-compliant winding order
    - If `-r` or `--reverse` are specified, output will be in RFC 7946 order
- Processing of nested `GeometryCollection`s is supported, [but you shouldn't be using those](https://tools.ietf.org/html/rfc7946#section-3.1.8)
- Empty geometries or collections will be left unaltered
- Non-(Multi)Polygon geometries are left unaltered
- All input properties are preserved

## Progress
If you aren't piping the output of the command to a file, `geojson_d3` will display progress of the parsing and processing steps in the terminal, as well as a final count of the processed (Multi)Polygons.

## Validity
While the structure of the input GeoJSON is validated, individual geometries are *not* validated in the DE-9IM sense. If they self-intersect, have open rings etc., results are not guaranteed to be correct.

## Speed
The included [`NYC Boroughs`](boroughs.geojson) file (~69k `Points`) is processed in ~140 ms on a dual-core 1.8 GHz Intel Core i7.

## Binaries
Pre-built binaries are available from [releases](https://github.com/urschrei/geojson_d3/releases/latest). Binaries are available for:
- macOS (x86_64)
- Linux (x86_64)
- Windows (x86_64 and i686)

## License
[MIT](license.txt)
