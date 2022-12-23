use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use flatgeobuf::{ColumnType, FgbCrs, FgbWriter, FgbWriterOptions, GeometryType};
use geo_types::Geometry;
use geozero::{ColumnValue, PropertyProcessor};
use h3ron::to_geo::ToLineString;
use h3ron::H3DirectedEdge;
use h3ron_graph::algorithm::covered_area::CoveredArea;
use h3ron_graph::graph::{GetStats, H3EdgeGraph, H3EdgeGraphBuilder, PreparedH3EdgeGraph};
use h3ron_graph::io::osm::OsmPbfH3EdgeGraphBuilder;
use mimalloc::MiMalloc;
use tracing::info;
use uom::si::f32::Length;
use uom::si::length::meter;
use uom::si::time::second;

use crate::config::ServerConfig;
use crate::io::serde_util::{deserialize_from_byte_slice, serialize_into};
use crate::osm::car::CarAnalyzer;
use crate::weight::{RoadWeight, Weight};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod build_info;
mod config;
mod customization;
mod grpc;
mod io;
mod osm;
mod weight;

fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    let app = Command::new(env!("CARGO_PKG_NAME"))
        .version(build_info::version())
        .long_version(build_info::long_version())
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
                    Command::new("to-fgb")
                        .about("Export the input graph to a flatgeobuf dataset")
                        .arg(Arg::new("GRAPH").help("graph").required(true))
                        .arg(
                            Arg::new("OUTPUT")
                                .help("output file to write the vector data to")
                                .required(true),
                        ),
                )
                .subcommand(
                    Command::new("from-osm-pbf")
                        .about("Build a routing graph from an OSM PBF file")
                        .arg(
                            Arg::new("h3_resolution")
                                .short('r')
                                .num_args(1)
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
                                .num_args(1..),
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
    let f = File::open(filename)?;
    let mapped = unsafe { memmap2::Mmap::map(&f)? };
    Ok(deserialize_from_byte_slice(&mapped)?)
}

fn dispatch_command(matches: ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("graph", graph_sc_matches)) => match graph_sc_matches.subcommand() {
            Some(("stats", sc_matches)) => {
                let graph_filename: &String = sc_matches.get_one("GRAPH").unwrap();
                let prepared_graph = read_graph_from_filename(graph_filename)?;
                println!("{}", serde_yaml::to_string(&prepared_graph.get_stats()?)?);
            }
            Some(("to-fgb", sc_matches)) => subcommand_graph_to_fgb(sc_matches)?,
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

fn subcommand_graph_to_fgb(sc_matches: &ArgMatches) -> Result<()> {
    let graph_filename: &String = sc_matches.get_one("GRAPH").unwrap();
    let graph: H3EdgeGraph<RoadWeight> = read_graph_from_filename(graph_filename)?.into();
    let mut writer = BufWriter::new(File::create(
        sc_matches.get_one::<String>("OUTPUT").unwrap(),
    )?);

    let mut fgb = FgbWriter::create_with_options(
        "edges",
        GeometryType::LineString,
        FgbWriterOptions {
            description: Some("graph edges"),
            crs: FgbCrs {
                code: 4326,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;

    fgb.add_column("travel_duration_secs", ColumnType::Float, |_fbb, col| {
        col.nullable = false;
    });
    fgb.add_column("edge_preference", ColumnType::Float, |_fbb, col| {
        col.nullable = false;
    });

    for (edge, weight) in graph.iter_edges() {
        fgb.add_feature_geom(Geometry::LineString(edge.to_linestring()?), |feat| {
            feat.property(
                0,
                "travel_duration_secs",
                &ColumnValue::Float(weight.travel_duration().get::<second>()),
            )
            .unwrap();
            feat.property(
                1,
                "edge_preference",
                &ColumnValue::Float(weight.edge_preference()),
            )
            .unwrap();
        })?;
    }
    fgb.write(&mut writer)?;
    Ok(())
}

fn subcommand_graph_covered_area(sc_matches: &ArgMatches) -> Result<()> {
    let graph_filename: &String = sc_matches.get_one("GRAPH").unwrap();
    let prepared_graph = read_graph_from_filename(graph_filename)?;

    let mut writer = BufWriter::new(File::create(
        sc_matches.get_one::<String>("OUT-GEOJSON").unwrap(),
    )?);
    let multi_poly = prepared_graph.covered_area(2)?;
    let gj_geom = geojson::Geometry::try_from(&multi_poly)?;
    writer.write_all(gj_geom.to_string().as_ref())?;

    writer.flush()?;
    Ok(())
}

fn subcommand_server(sc_matches: &ArgMatches) -> Result<()> {
    let config_contents =
        std::fs::read_to_string(sc_matches.get_one::<String>("CONFIG-FILE").unwrap())?;
    let config: ServerConfig = serde_yaml::from_str(&config_contents)?;
    config.validate()?;
    grpc::launch_server::<RoadWeight>(config)?;
    Ok(())
}

fn subcommand_from_osm_pbf(sc_matches: &ArgMatches) -> Result<()> {
    let h3_resolution: u8 = sc_matches
        .get_one::<String>("h3_resolution")
        .unwrap()
        .parse()?;
    let graph_output: &String = sc_matches.get_one("OUTPUT-GRAPH").unwrap();

    let edge_length = Length::new::<meter>(
        H3DirectedEdge::cell_centroid_distance_avg_m_at_resolution(h3_resolution)? as f32,
    );
    info!(
        "Building graph using resolution {} with edge length ~= {:?}",
        h3_resolution, edge_length
    );
    let mut builder = OsmPbfH3EdgeGraphBuilder::new(h3_resolution, CarAnalyzer {});
    for pbf_input in sc_matches.get_many::<String>("OSM-PBF").unwrap() {
        builder.read_pbf(Path::new(&pbf_input))?;
    }
    let graph = builder.build_graph()?;

    info!("Preparing graph");
    let prepared_graph = PreparedH3EdgeGraph::from_h3edge_graph(graph, 5)?;

    let stats = prepared_graph.get_stats()?;
    info!(
        "Created graph ({} nodes, {} edges)",
        stats.num_nodes, stats.num_edges
    );
    let mut writer = BufWriter::new(File::create(graph_output)?);
    serialize_into(&mut writer, &prepared_graph, true)?;
    Ok(())
}
