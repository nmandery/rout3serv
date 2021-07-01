use std::fs::File;
use std::path::Path;

use clap::{App, Arg, SubCommand};
use eyre::Result;
use osmpbfreader::Tags;

use route3_core::graph::GraphBuilder;
use route3_core::io::save_graph_to_file;

use crate::osm::{EdgeProperties, OsmPbfGraphBuilder};

mod osm;

fn way_properties(tags: &Tags) -> Option<EdgeProperties<u64>> {
    // https://wiki.openstreetmap.org/wiki/Key:highway
    if let Some(highway_value) = tags.get("highway") {
        match highway_value.to_lowercase().as_str() {
            "motorway" | "motorway_link" | "trunk" | "trunk_link" | "primary" | "primary_link" => {
                Some(3)
            }
            "secondary" | "secondary_link" => Some(4),
            "tertiary" | "tertiary_link" => Some(5),
            "unclassified" | "residential" | "living_street" => Some(8),
            "road" => Some(12),
            //"service" | "track" => Some(20),
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

fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

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
        .get_matches();

    match matches.subcommand() {
        ("build-from-osm-pbf", Some(sc_matches)) => {
            let h3_resolution: u8 = sc_matches.value_of("h3_resolution").unwrap().parse()?;
            let graph_output = sc_matches.value_of("OUTPUT-GRAPH").unwrap().to_string();

            let mut builder = OsmPbfGraphBuilder::new(h3_resolution, way_properties);
            for pbf_input in sc_matches.values_of("OSM-PBF").unwrap() {
                builder.read_pbf(Path::new(&pbf_input))?;
            }
            let graph = builder.build_graph()?;

            log::info!(
                "Created graph ({} nodes, {} edges)",
                graph.num_nodes()?,
                graph.num_edges()
            );

            let mut out_file = File::create(graph_output)?;
            save_graph_to_file(&graph, &mut out_file)?;
        }
        _ => {
            println!("unknown command");
        }
    }
    Ok(())
}
