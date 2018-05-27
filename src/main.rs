use std::fs::read_to_string;
use std::io::Error as IoErr;
use std::mem::replace;
use std::sync::atomic::{AtomicIsize, Ordering};

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate geo_types;
use geo_types::{LineString, MultiPoint, MultiPolygon, Point, Polygon};

extern crate geo;
use geo::winding_order::Winding;

extern crate geojson;
use geojson::conversion::TryInto;
use geojson::{Error as GjErr, Feature, FeatureCollection, GeoJson, Geometry, Value};

extern crate serde_json;
use serde_json::{to_string_pretty, Map};

extern crate rayon;
use rayon::prelude::*;

extern crate failure;

extern crate console;
use console::{style, user_attended};

extern crate indicatif;
use indicatif::ProgressBar;

#[macro_use]
extern crate failure_derive;

#[derive(Fail, Debug)]
enum PolylabelError {
    #[fail(display = "IO error: {}", _0)]
    IoError(#[cause] IoErr),
    #[fail(display = "GeoJSON deserialisation error: {}. Is your GeoJSON valid?", _0)]
    GeojsonError(#[cause] GjErr),
}

impl From<IoErr> for PolylabelError {
    fn from(err: IoErr) -> PolylabelError {
        PolylabelError::IoError(err)
    }
}

impl From<GjErr> for PolylabelError {
    fn from(err: GjErr) -> PolylabelError {
        PolylabelError::GeojsonError(err)
    }
}

/// Attempt to open a file, read it, and parse it into `GeoJSON`
fn open_and_parse(p: &str) -> Result<GeoJson, PolylabelError> {
    let contents = read_to_string(p)?;
    Ok(contents.parse::<GeoJson>()?)
}

/// Process top-level `GeoJSON` items
fn process_geojson(gj: &mut GeoJson, ctr: &AtomicIsize) {
    match *gj {
        GeoJson::FeatureCollection(ref mut collection) => collection.features
            .par_iter_mut()
            // Only pass on non-empty geometries, doing so by reference
            .filter_map(|feature| feature.geometry.as_mut())
            .for_each(|geometry| label_geometry(geometry, ctr)),
        GeoJson::Feature(ref mut feature) => {
            if let Some(ref mut geometry) = feature.geometry {
                label_geometry(geometry, ctr)
            }
        }
        GeoJson::Geometry(ref mut geometry) => label_geometry(geometry, ctr),
    }
}

/// Process `GeoJSON` geometries
fn label_geometry(geom: &mut Geometry, ctr: &AtomicIsize) {
    match geom.value {
        Value::Polygon(_) | Value::MultiPolygon(_) => reverse_rings(Some(geom), ctr),
        Value::GeometryCollection(ref mut collection) => {
            // GeometryCollections contain other Geometry types, and can nest
            // we deal with this by recursively processing each geometry
            collection
                .par_iter_mut()
                .for_each(|geometry| label_geometry(geometry, ctr))
        }
        // Point, LineString, and their Multi– counterparts
        // no-op
        _ => {}
    }
}

/// Generate a label position for a (Multi)Polygon Value
fn reverse_rings(geom: Option<&mut Geometry>, ctr: &AtomicIsize) {
    if let Some(gmt) = geom {
        // construct a fake empty Polygon – this doesn't allocate
        let v1: Vec<Point<f64>> = Vec::new();
        let ls2 = Vec::new();
        let fake_polygon: Polygon<f64> = Polygon::new(LineString::from(v1), ls2);
        // convert it into a Value, and swap it for our actual (Multi)Polygon
        gmt.value = match gmt.value {
            Value::Polygon(_) => {
                let intermediate = replace(&mut gmt.value, Value::from(&fake_polygon));
                let mut geo_type: Polygon<f64> = intermediate
                    .try_into()
                    .expect("Failed to convert a Polygon");
                geo_type.exterior.make_cw_winding();
                for line in &mut geo_type.interiors {
                    line.make_ccw_winding();
                }
                ctr.fetch_add(1, Ordering::SeqCst);
                Value::from(&geo_type)
            }
            Value::MultiPolygon(_) => {
                let intermediate = replace(&mut gmt.value, Value::from(&fake_polygon));
                let mut geo_type: MultiPolygon<f64> = intermediate
                    .try_into()
                    .expect("Failed to convert a MultiPolygon");
                geo_type.0.par_iter_mut().for_each(|polygon| {
                    // bump the Polygon counter
                    ctr.fetch_add(1, Ordering::SeqCst);
                    polygon.exterior.make_cw_winding();
                    for line in &mut polygon.interiors {
                        line.make_ccw_winding();
                    }
                });
                Value::from(&geo_type)
            }
            _ => replace(&mut gmt.value, Value::from(&fake_polygon)),
        }
    }
}

fn main() {
    let command_params = App::new("geojson_d3")
        .version(&crate_version!()[..])
        .author("Stephan Hügel <urschrei@gmail.com>")
        .about("Make GeoJSON (Multi)Polygons D3-compatible")
        .arg(
            Arg::with_name("pretty")
                .help("Pretty-print GeoJSON output")
                .short("p")
                .long("pretty"),
        )
        .arg(
            Arg::with_name("statsonly")
                .help("Process polygons, but only print stats")
                .short("s")
                .long("stats-only"),
        )
        .arg(
            Arg::with_name("GEOJSON")
                .help("GeoJSON containing (Multi)Polygons you wish to process using D3")
                .index(1)
                .required(true),
        )
        .get_matches();

    let poly = value_t!(command_params.value_of("GEOJSON"), String).unwrap();
    let pprint = command_params.is_present("pretty");
    let statsonly = command_params.is_present("statsonly");
    let sp = ProgressBar::new_spinner();
    sp.set_message("Parsing GeoJSON");
    sp.enable_steady_tick(1);
    let res = open_and_parse(&poly);
    sp.finish_and_clear();
    let sp2 = ProgressBar::new_spinner();
    sp2.set_message("Processing…");
    sp2.enable_steady_tick(1);
    match res {
        Err(e) => println!("{}", e),
        Ok(mut gj) => {
            let ctr = AtomicIsize::new(0);
            process_geojson(&mut gj, &ctr);
            sp2.finish_and_clear();
            let to_print = if !pprint {
                gj.to_string()
            } else {
                to_string_pretty(&gj).unwrap()
            };
            if user_attended() {
                let labelled = ctr.load(Ordering::Relaxed);
                println!(
                    "Processing complete. Processed {} {}\n",
                    style(&labelled.to_string()).red(),
                    if labelled == 1 { "Polygon" } else { "Polygons" }
                );
            }
            if !statsonly {
                println!("{}", to_print);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geojson::GeoJson;
    #[test]
    fn test_ccw() {
        let raw_gj = r#"
        {
          "features": [
            {
              "geometry": {
                "coordinates": [
                  [
                    [
                      0.0,
                      0.0
                    ],
                    [
                      3.0,
                      6.0
                    ],
                    [
                      6.0,
                      1.0
                    ],
                    [
                      0.0,
                      0.0
                    ]
                  ],
                  [
                    [
                      2.0,
                      2.0
                    ],
                    [
                      4.0,
                      2.0
                    ],
                    [
                      3.0,
                      3.0
                    ],
                    [
                      2.0,
                      2.0
                    ]
                  ]
                ],
                "type": "Polygon"
              },
              "properties": {
                "foo": "bar"
              },
              "type": "Feature"
            }
          ],
          "type": "FeatureCollection"
        }
        "#;
        let correct = raw_gj.parse::<GeoJson>().unwrap();
        let mut gj = open_and_parse(&"with_hole.geojson").unwrap();
        let ctr = AtomicIsize::new(0);
        process_geojson(&mut gj, &ctr);
        assert_eq!(gj, correct);
    }
}
