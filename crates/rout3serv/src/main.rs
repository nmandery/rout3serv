use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use flatgeobuf::{ColumnType, FgbCrs, FgbWriter, FgbWriterOptions, GeometryType};
use geo_types::{Geometry, LineString};
use geozero::{ColumnValue, PropertyProcessor};
use h3o::geom::ToGeo;
use h3o::Resolution;
use hexigraph::algorithm::edge::cell_centroid_distance_avg_m_at_resolution;
use hexigraph::algorithm::graph::CoveredArea;
use hexigraph::graph::{GetStats, H3EdgeGraphBuilder, PreparedH3EdgeGraph};
use hexigraph::io::osm::OsmPbfH3EdgeGraphBuilder;
use mimalloc::MiMalloc;
use tracing::info;
use uom::si::f32::Length;
use uom::si::length::meter;
use uom::si::time::second;

use crate::config::ServerConfig;
use crate::io::ipc::{ReadIPC, WriteIPC};
use crate::osm::car::CarAnalyzer;
use crate::weight::{StandardWeight, Weight};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod build_info;
mod config;
mod customization;
mod geo;
mod grpc;
mod io;
mod osm;
mod weight;

const SC_GRPC_SERVER: &str = "grpc";
const SC_GRAPH: &str = "graph";
const SC_GRAPH_STATS: &str = "stats";
const SC_GRAPH_COVERED_AREA: &str = "covered-area";
const SC_GRAPH_TO_FGB: &str = "to-fgb";
const SC_GRAPH_FROM_OSM_PBF: &str = "from-osm-pbf";

fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    let app = Command::new(env!("CARGO_PKG_NAME"))
        .version(build_info::version())
        .long_version(build_info::long_version())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommand(
            Command::new(SC_GRAPH)
                .about("Commands related to graph creation and export")
                .subcommand(
                    Command::new(SC_GRAPH_STATS)
                        .about("Load a graph and print some basic stats")
                        .arg(Arg::new("GRAPH").help("graph").required(true)),
                )
                .subcommand(
                    Command::new(SC_GRAPH_COVERED_AREA)
                        .about("Extract the area covered by the graph as geojson")
                        .arg(Arg::new("GRAPH").help("graph").required(true))
                        .arg(
                            Arg::new("OUT-GEOJSON")
                                .help("output file to write the geojson geometry to")
                                .required(true),
                        ),
                )
                .subcommand(
                    Command::new(SC_GRAPH_TO_FGB)
                        .about("Export the input graph to a flatgeobuf dataset")
                        .arg(Arg::new("GRAPH").help("graph").required(true))
                        .arg(
                            Arg::new("OUTPUT")
                                .help("output file to write the vector data to")
                                .required(true),
                        ),
                )
                .subcommand(
                    Command::new(SC_GRAPH_FROM_OSM_PBF)
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
            Command::new(SC_GRPC_SERVER)
                .about("Start the GRPC server")
                .arg(
                    Arg::new("CONFIG-FILE")
                        .help("server configuration file")
                        .required(true),
                ),
        );

    dispatch_command(app.get_matches())
}

fn read_graph_from_filename(filename: &str) -> Result<PreparedH3EdgeGraph<StandardWeight>> {
    let f = File::open(filename)?;
    Ok(PreparedH3EdgeGraph::read_ipc(BufReader::new(f))?)
}

fn dispatch_command(matches: ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some((SC_GRAPH, graph_sc_matches)) => match graph_sc_matches.subcommand() {
            Some((SC_GRAPH_STATS, sc_matches)) => {
                let graph_filename: &String = sc_matches.get_one("GRAPH").unwrap();
                let prepared_graph = read_graph_from_filename(graph_filename)?;
                println!("{}", serde_yaml::to_string(&prepared_graph.get_stats()?)?);
            }
            Some((SC_GRAPH_TO_FGB, sc_matches)) => subcommand_graph_to_fgb(sc_matches)?,
            Some((SC_GRAPH_COVERED_AREA, sc_matches)) => subcommand_graph_covered_area(sc_matches)?,
            Some((SC_GRAPH_FROM_OSM_PBF, sc_matches)) => subcommand_from_osm_pbf(sc_matches)?,
            _ => {
                println!("unknown subcommand");
            }
        },
        Some((SC_GRPC_SERVER, sc_matches)) => subcommand_grpc_server(sc_matches)?,
        _ => {
            println!("unknown subcommand");
        }
    }
    Ok(())
}

fn subcommand_graph_to_fgb(sc_matches: &ArgMatches) -> Result<()> {
    let graph_filename: &String = sc_matches.get_one("GRAPH").unwrap();
    let graph = read_graph_from_filename(graph_filename)?;
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
    fgb.add_column("is_long_edge", ColumnType::Bool, |_fbb, col| {
        col.nullable = false;
    });
    fgb.add_column("num_edges", ColumnType::UInt, |_fbb, col| {
        col.nullable = false;
    });

    for (edge, edgeweight) in graph.iter_edges() {
        let line = edge.to_geom(true).unwrap();
        fgb.add_feature_geom(Geometry::LineString(LineString::from(line)), |feat| {
            feat.property(
                0,
                "travel_duration_secs",
                &ColumnValue::Float(edgeweight.weight.travel_duration().get::<second>()),
            )
            .unwrap();
            feat.property(
                1,
                "edge_preference",
                &ColumnValue::Float(edgeweight.weight.edge_preference()),
            )
            .unwrap();
            feat.property(2, "is_long_edge", &ColumnValue::Bool(false))
                .unwrap();
            feat.property(3, "num_edges", &ColumnValue::UInt(1))
                .unwrap();
        })?;

        if let Some((fastforward, fastforward_weight)) = edgeweight.fastforward {
            fgb.add_feature_geom(Geometry::LineString(fastforward.to_linestring()?), |feat| {
                feat.property(
                    0,
                    "travel_duration_secs",
                    &ColumnValue::Float(fastforward_weight.travel_duration().get::<second>()),
                )
                .unwrap();
                feat.property(
                    1,
                    "edge_preference",
                    &ColumnValue::Float(fastforward_weight.edge_preference()),
                )
                .unwrap();
                feat.property(2, "is_long_edge", &ColumnValue::Bool(true))
                    .unwrap();
                feat.property(
                    3,
                    "num_edges",
                    &ColumnValue::UInt(fastforward.h3edges_len() as u32),
                )
                .unwrap();
            })?;
        }
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

fn subcommand_grpc_server(sc_matches: &ArgMatches) -> Result<()> {
    let config_contents =
        std::fs::read_to_string(sc_matches.get_one::<String>("CONFIG-FILE").unwrap())?;
    let config: ServerConfig = serde_yaml::from_str(&config_contents)?;
    config.validate()?;
    grpc::launch_server(config)?;
    Ok(())
}

fn subcommand_from_osm_pbf(sc_matches: &ArgMatches) -> Result<()> {
    let h3_resolution: u8 = sc_matches
        .get_one::<String>("h3_resolution")
        .unwrap()
        .parse()?;
    let h3_resolution: Resolution = h3_resolution.try_into()?;
    let graph_output: &String = sc_matches.get_one("OUTPUT-GRAPH").unwrap();

    let edge_length =
        Length::new::<meter>(cell_centroid_distance_avg_m_at_resolution(h3_resolution) as f32);
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
    let writer = BufWriter::new(File::create(graph_output)?);
    prepared_graph.write_ipc(writer)?;
    Ok(())
}
