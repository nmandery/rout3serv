// This example can also be used to build the benchmark data for `route_germany`.

use std::fs::File;
use std::path::Path;

use clap::{App, Arg};
use ordered_float::OrderedFloat;

use route3_core::formats::osm::osmpbfreader::Tags;
use route3_core::formats::osm::{EdgeProperties, OsmPbfH3EdgeGraphBuilder};
use route3_core::graph::H3EdgeGraphBuilder;
use route3_core::io::save_graph_to_file;

pub fn way_properties(tags: &Tags) -> Option<EdgeProperties<OrderedFloat<f64>>> {
    // https://wiki.openstreetmap.org/wiki/Key:highway or https://wiki.openstreetmap.org/wiki/DE:Key:highway
    if let Some(highway_value) = tags.get("highway") {
        match highway_value.to_lowercase().as_str() {
            "motorway" | "motorway_link" | "trunk" | "trunk_link" | "primary" | "primary_link" => {
                Some(3.0.into())
            }
            "secondary" | "secondary_link" => Some(4.0.into()),
            "tertiary" | "tertiary_link" => Some(5.0.into()),
            "unclassified" | "residential" | "living_street" | "service" => Some(8.0.into()),
            "road" => Some(9.0.into()),
            "pedestrian" => Some(50.0.into()), // fussgängerzone
            _ => None,
        }
        .map(|weight| {
            // oneway streets (https://wiki.openstreetmap.org/wiki/Key:oneway)
            // NOTE: reversed direction "oneway=-1" is not supported
            let is_bidirectional = tags
                .get("oneway")
                .map(|v| v.to_lowercase() != "yes")
                .unwrap_or(true);
            EdgeProperties {
                is_bidirectional,
                weight,
            }
        })
    } else {
        None
    }
}

fn main() {
    let app = App::new("graph_from_osm")
        .about("Build a routing graph from an OSM PBF file")
        .arg(
            Arg::with_name("h3_resolution")
                .short("r")
                .takes_value(true)
                .default_value("7"),
        )
        .arg(
            Arg::with_name("OUTPUT-GRAPH")
                .help("output file to write the graph to")
                .required(true),
        )
        .arg(
            Arg::with_name("OSM-PBF")
                .help("input OSM .pbf file")
                .required(true)
                .min_values(1),
        );

    let matches = app.get_matches();

    let h3_resolution: u8 = matches
        .value_of("h3_resolution")
        .unwrap()
        .parse()
        .expect("invalid h3 resolution");
    let graph_output = matches.value_of("OUTPUT-GRAPH").unwrap().to_string();

    let mut builder = OsmPbfH3EdgeGraphBuilder::new(h3_resolution, way_properties);
    for pbf_input in matches.values_of("OSM-PBF").unwrap() {
        builder
            .read_pbf(Path::new(&pbf_input))
            .expect("reading pbf failed");
    }
    let graph = builder.build_graph().expect("building graph failed");

    println!(
        "Created graph ({} nodes, {} edges)",
        graph.num_nodes(),
        graph.num_edges()
    );
    let mut out_file = File::create(graph_output).expect("creating output file failed");
    save_graph_to_file(&graph, &mut out_file).expect("writing graph failed");
}
