use std::fs;
use std::io::Error as IoErr;
use std::mem::replace;
use std::path::Path;
use std::sync::atomic::{AtomicIsize, Ordering};

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate geo_types;
use geo_types::{LineString, MultiPolygon, Point, Polygon};

extern crate geo;
use geo::winding_order::Winding;

extern crate geojson;
use geojson::conversion::TryInto;
use geojson::{Error as GjErr, GeoJson, Geometry, Value};

extern crate serde_json;
use serde_json::to_string_pretty;

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
fn open_and_parse<P>(filename: P) -> Result<GeoJson, PolylabelError>
where
    P: AsRef<Path>,
{
    let s = fs::read_to_string(filename)?;
    Ok(s.parse::<GeoJson>()?)
}

/// Process top-level `GeoJSON` items
fn process_geojson(gj: &mut GeoJson, ctr: &AtomicIsize, rev: &bool) {
    match *gj {
        GeoJson::FeatureCollection(ref mut collection) => collection.features
            .par_iter_mut()
            // Only pass on non-empty geometries, doing so by reference
            .filter_map(|feature| feature.geometry.as_mut())
            .for_each(|geometry| process_geometry(geometry, ctr, rev)),
        GeoJson::Feature(ref mut feature) => {
            if let Some(ref mut geometry) = feature.geometry {
                process_geometry(geometry, ctr, rev)
            }
        }
        GeoJson::Geometry(ref mut geometry) => process_geometry(geometry, ctr, rev),
    }
}

/// Process `GeoJSON` geometries
fn process_geometry(geom: &mut Geometry, ctr: &AtomicIsize, rev: &bool) {
    match geom.value {
        Value::Polygon(_) | Value::MultiPolygon(_) => reverse_rings(Some(geom), ctr, rev),
        Value::GeometryCollection(ref mut collection) => {
            // GeometryCollections contain other Geometry types, and can nest
            // we deal with this by recursively processing each geometry
            collection
                .par_iter_mut()
                .for_each(|geometry| process_geometry(geometry, ctr, rev))
        }
        // Point, LineString, and their Multi– counterparts
        // no-op
        _ => {}
    }
}

/// Generate a label position for a (Multi)Polygon Value
fn reverse_rings(geom: Option<&mut Geometry>, ctr: &AtomicIsize, rev: &bool) {
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
                wind(&mut geo_type, rev);
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
                    wind(polygon, rev);
                });
                Value::from(&geo_type)
            }
            _ => replace(&mut gmt.value, Value::from(&fake_polygon)),
        }
    }
}

/// Wind RFC 7946 Polygon rings to make them D3 compatible, or vice-versa
fn wind(poly: &mut Polygon<f64>, rev: &bool) {
    if !rev {
        poly.exterior.make_cw_winding();
        poly.interiors
            .iter_mut()
            .for_each(|ring| ring.make_ccw_winding());
    } else {
        poly.exterior.make_ccw_winding();
        poly.interiors
            .iter_mut()
            .for_each(|ring| ring.make_cw_winding());
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
            Arg::with_name("reverse")
                .help("Make D3-compatible Polygons RFC 7946-compatible")
                .short("r")
                .long("reverse"),
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
    let reverse = command_params.is_present("reverse");
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
            process_geojson(&mut gj, &ctr, &reverse);
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
        let rev = false;
        let correct = raw_gj.parse::<GeoJson>().unwrap();
        let mut gj = open_and_parse(&"with_hole.geojson").unwrap();
        let ctr = AtomicIsize::new(0);
        process_geojson(&mut gj, &ctr, &rev);
        assert_eq!(gj, correct);
    }
}
