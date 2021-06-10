use std::fs::File;
use std::path::Path;

use clap::{App, Arg, SubCommand};
use osmpbfreader::Tags;

use crate::graph::{EdgeProperties, Graph, GraphBuilder};
use crate::io::{GraphStats, OgrWrite};

mod graph;
mod io;
mod osm;

fn way_properties(tags: &Tags) -> Option<EdgeProperties> {
    // https://wiki.openstreetmap.org/wiki/Key:highway
    if let Some(highway_value) = tags.get("highway") {
        match highway_value.to_lowercase().as_str() {
            "motorway" | "motorway_link" | "trunk" | "trunk_link" | "primary" | "primary_link" => {
                Some(10)
            }
            "secondary" | "secondary_link" => Some(8),
            "tertiary" | "tertiary_link" => Some(5),
            "unclassified" | "residential" | "living_street" => Some(3),
            "road" => Some(1),
            //"service" | "track" => Some(1),
            _ => None,
        }
        .map(|weight| {
            // oneway streets (https://wiki.openstreetmap.org/wiki/Key:oneway)
            // NOTE: reversed direction "oneway=-1" is not supported
            let is_bidirectional = tags
                .get("oneway")
                .map(|v| {
                    if v.to_lowercase() == "yes" {
                        false
                    } else {
                        true
                    }
                })
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

fn print_graph_stats(graph: &Graph) -> eyre::Result<()> {
    let stats = GraphStats::new(graph);
    println!("{}", toml::to_string(&stats)?);
    Ok(())
}

fn load_graph<R: std::io::Read>(reader: R) -> eyre::Result<Graph> {
    let graph = bincode::deserialize_from(reader)?;
    print_graph_stats(&graph)?;
    Ok(graph)
}

fn main() -> eyre::Result<()> {
    env_logger::init();
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommand(
            SubCommand::with_name("build-from-osm-pbf")
                .about("Build a routing graph from an OSM PBF file")
                .arg(
                    Arg::with_name("h3_resolution")
                        .short("r")
                        .takes_value(true)
                        .default_value("10"),
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
                ),
        )
        .subcommand(
            SubCommand::with_name("graph-stats")
                .about("Load a graph and print some basic stats")
                .arg(Arg::with_name("GRAPH").help("graph").required(true)),
        )
        .subcommand(
            SubCommand::with_name("graph-to-ogr")
                .about("Export the input graph to an OGR vector dataset")
                .arg(Arg::with_name("GRAPH").help("graph").required(true))
                .arg(
                    Arg::with_name("OUTPUT")
                        .help("output file to write the vector data to")
                        .required(true),
                )
                .arg(
                    Arg::with_name("driver")
                        .help("OGR driver to use")
                        .short("d")
                        .default_value("FlatGeobuf"),
                )
                .arg(
                    Arg::with_name("layer_name")
                        .help("layer name")
                        .short("l")
                        .default_value("graph"),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("build-from-osm-pbf", Some(sc_matches)) => {
            let h3_resolution: u8 = sc_matches.value_of("h3_resolution").unwrap().parse()?;
            let graph_output = sc_matches.value_of("OUTPUT-GRAPH").unwrap().to_string();

            let mut builder = crate::osm::OsmPbfGraphBuilder::new(h3_resolution, way_properties);
            for pbf_input in sc_matches.values_of("OSM-PBF").unwrap() {
                builder.read_pbf(Path::new(&pbf_input))?;
            }
            let graph = builder.build_graph()?;

            println!("Created graph");
            print_graph_stats(&graph)?;

            bincode::serialize_into(File::create(graph_output)?, &graph)?;
        }
        ("graph-stats", Some(sc_matches)) => {
            let graph_filename = sc_matches.value_of("GRAPH").unwrap().to_string();
            let _ = load_graph(File::open(graph_filename)?)?;
        }
        ("graph-to-ogr", Some(sc_matches)) => {
            let graph_filename = sc_matches.value_of("GRAPH").unwrap().to_string();
            let graph = load_graph(File::open(graph_filename)?)?;
            graph.ogr_write(
                sc_matches.value_of("driver").unwrap(),
                sc_matches.value_of("OUTPUT").unwrap(),
                sc_matches.value_of("layer_name").unwrap(),
            )?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
