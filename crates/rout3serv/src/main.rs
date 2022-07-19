use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use h3ron::H3DirectedEdge;
use h3ron_graph::algorithm::covered_area::CoveredArea;
use h3ron_graph::graph::{GetStats, H3EdgeGraph, H3EdgeGraphBuilder, PreparedH3EdgeGraph};
use h3ron_graph::io::gdal::OgrWrite;
use h3ron_graph::io::osm::OsmPbfH3EdgeGraphBuilder;
use mimalloc::MiMalloc;
use s3io::ser_and_de::{deserialize_from, serialize_into};
use uom::si::f32::Length;
use uom::si::length::meter;

use crate::config::ServerConfig;
use crate::osm::car::CarAnalyzer;
use crate::weight::RoadWeight;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod build_info;
mod config;
mod customization;
mod io;
mod osm;
mod server;
mod weight;

fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    let long_version = format!(
        "{} (git: {}, build on {})",
        crate::build_info::version(),
        crate::build_info::git_comit_sha(),
        crate::build_info::build_timestamp()
    );

    let app = Command::new(env!("CARGO_PKG_NAME"))
        .version(crate::build_info::version())
        .long_version(long_version.as_str())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommand(
            Command::new("graph")
                .about("Commands related to graph creation and export")
                .subcommand(
                    Command::new("stats")
                        .about("Load a graph and print some basic stats")
                        .arg(Arg::new("GRAPH").help("graph").required(true)),
                )
                .subcommand(
                    Command::new("covered-area")
                        .about("Extract the area covered by the graph as geojson")
                        .arg(Arg::new("GRAPH").help("graph").required(true))
                        .arg(
                            Arg::new("OUT-GEOJSON")
                                .help("output file to write the geojson geometry to")
                                .required(true),
                        ),
                )
                .subcommand(
                    Command::new("to-ogr")
                        .about("Export the input graph to an OGR vector dataset")
                        .arg(Arg::new("GRAPH").help("graph").required(true))
                        .arg(
                            Arg::new("OUTPUT")
                                .help("output file to write the vector data to")
                                .required(true),
                        )
                        .arg(
                            Arg::new("driver")
                                .help("OGR driver to use")
                                .short('d')
                                .default_value("FlatGeobuf"),
                        )
                        .arg(
                            Arg::new("layer_name")
                                .help("layer name")
                                .short('l')
                                .default_value("graph"),
                        ),
                )
                .subcommand(
                    Command::new("from-osm-pbf")
                        .about("Build a routing graph from an OSM PBF file")
                        .arg(
                            Arg::new("h3_resolution")
                                .short('r')
                                .takes_value(true)
                                .default_value("10"),
                        )
                        .arg(
                            Arg::new("OUTPUT-GRAPH")
                                .help("output file to write the graph to")
                                .required(true),
                        )
                        .arg(
                            Arg::new("OSM-PBF")
                                .help("input OSM .pbf file")
                                .required(true)
                                .min_values(1),
                        ),
                ),
        )
        .subcommand(
            Command::new("server").about("Start the GRPC server").arg(
                Arg::new("CONFIG-FILE")
                    .help("server configuration file")
                    .required(true),
            ),
        );

    dispatch_command(app.get_matches())
}

fn read_graph_from_filename(filename: &str) -> Result<PreparedH3EdgeGraph<RoadWeight>> {
    Ok(deserialize_from(BufReader::new(File::open(filename)?))?)
}

fn dispatch_command(matches: ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("graph", graph_sc_matches)) => match graph_sc_matches.subcommand() {
            Some(("stats", sc_matches)) => {
                let graph_filename = sc_matches.value_of("GRAPH").unwrap().to_string();
                let prepared_graph = read_graph_from_filename(&graph_filename)?;
                println!("{}", serde_yaml::to_string(&prepared_graph.get_stats()?)?);
            }
            Some(("to-ogr", sc_matches)) => subcommand_graph_to_ogr(sc_matches)?,
            Some(("covered-area", sc_matches)) => subcommand_graph_covered_area(sc_matches)?,
            Some(("from-osm-pbf", sc_matches)) => subcommand_from_osm_pbf(sc_matches)?,
            _ => {
                println!("unknown subcommand");
            }
        },
        Some(("server", sc_matches)) => subcommand_server(sc_matches)?,
        _ => {
            println!("unknown subcommand");
        }
    }
    Ok(())
}

fn subcommand_graph_to_ogr(sc_matches: &ArgMatches) -> Result<()> {
    let graph_filename = sc_matches.value_of("GRAPH").unwrap().to_string();
    let graph: H3EdgeGraph<RoadWeight> = read_graph_from_filename(&graph_filename)?.into();
    graph.ogr_write(
        sc_matches.value_of("driver").unwrap(),
        sc_matches.value_of("OUTPUT").unwrap(),
        sc_matches.value_of("layer_name").unwrap(),
    )?;
    Ok(())
}

fn subcommand_graph_covered_area(sc_matches: &ArgMatches) -> Result<()> {
    let graph_filename = sc_matches.value_of("GRAPH").unwrap().to_string();
    let prepared_graph = read_graph_from_filename(&graph_filename)?;

    let mut writer = BufWriter::new(File::create(sc_matches.value_of("OUT-GEOJSON").unwrap())?);
    let multi_poly = prepared_graph.covered_area(2)?;
    let gj_geom = geojson::Geometry::try_from(&multi_poly)?;
    writer.write_all(gj_geom.to_string().as_ref())?;

    writer.flush()?;
    Ok(())
}

fn subcommand_server(sc_matches: &ArgMatches) -> Result<()> {
    let config_contents = std::fs::read_to_string(sc_matches.value_of("CONFIG-FILE").unwrap())?;
    let config: ServerConfig = serde_yaml::from_str(&config_contents)?;
    config.validate()?;
    crate::server::launch_server(config)?;
    Ok(())
}

fn subcommand_from_osm_pbf(sc_matches: &ArgMatches) -> Result<()> {
    let h3_resolution: u8 = sc_matches.value_of("h3_resolution").unwrap().parse()?;
    let graph_output = sc_matches.value_of("OUTPUT-GRAPH").unwrap().to_string();

    let edge_length = Length::new::<meter>(
        H3DirectedEdge::cell_centroid_distance_avg_m_at_resolution(h3_resolution)? as f32,
    );
    log::info!(
        "Building graph using resolution {} with edge length ~= {:?}",
        h3_resolution,
        edge_length
    );
    let mut builder = OsmPbfH3EdgeGraphBuilder::new(h3_resolution, CarAnalyzer {});
    for pbf_input in sc_matches.values_of("OSM-PBF").unwrap() {
        builder.read_pbf(Path::new(&pbf_input))?;
    }
    let graph = builder.build_graph()?;

    log::info!("Preparing graph");
    let prepared_graph = PreparedH3EdgeGraph::from_h3edge_graph(graph, 5)?;

    let stats = prepared_graph.get_stats()?;
    log::info!(
        "Created graph ({} nodes, {} edges)",
        stats.num_nodes,
        stats.num_edges
    );
    let mut writer = BufWriter::new(File::create(graph_output)?);
    serialize_into(&mut writer, &prepared_graph, true)?;
    Ok(())
}
