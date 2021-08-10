#[macro_use]
extern crate lazy_static;

use std::convert::TryFrom;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use clap::{App, Arg, ArgMatches, SubCommand};
use eyre::Result;
use mimalloc::MiMalloc;

use route3_core::formats::osm::OsmPbfH3EdgeGraphBuilder;
use route3_core::graph::{H3EdgeGraph, H3EdgeGraphBuilder};
use route3_core::io::gdal::OgrWrite;

use crate::io::{arrow_load_graph, arrow_save_graph};
use crate::osm::way_properties;
use crate::types::Weight;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod build_info;
mod io;
mod osm;
mod server;
mod types;

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

    let app = App::new(env!("CARGO_PKG_NAME"))
        .version(crate::build_info::version())
        .long_version(long_version.as_str())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommand(
            SubCommand::with_name("graph")
                .about("Commands related to graph creation and export")
                .subcommand(
                    SubCommand::with_name("stats")
                        .about("Load a graph and print some basic stats")
                        .arg(Arg::with_name("GRAPH").help("graph").required(true)),
                )
                .subcommand(
                    SubCommand::with_name("covered-area")
                        .about("Extract the area covered by the graph as geojson")
                        .arg(Arg::with_name("GRAPH").help("graph").required(true))
                        .arg(
                            Arg::with_name("OUT-GEOJSON")
                                .help("output file to write the geojson geometry to")
                                .required(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("to-ogr")
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
                .subcommand(
                    SubCommand::with_name("from-osm-pbf")
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
                ),
        )
        .subcommand(
            SubCommand::with_name("server")
                .about("Start the GRPC server")
                .arg(
                    Arg::with_name("CONFIG-FILE")
                        .help("server configuration file")
                        .required(true),
                ),
        );

    let matches = app.get_matches();

    match matches.subcommand() {
        ("graph", Some(graph_sc_matches)) => match graph_sc_matches.subcommand() {
            ("stats", Some(sc_matches)) => {
                let graph_filename = sc_matches.value_of("GRAPH").unwrap().to_string();
                let graph: H3EdgeGraph<Weight> = arrow_load_graph(File::open(graph_filename)?)?;
                println!("{}", toml::to_string(&graph.stats())?);
            }
            ("to-ogr", Some(sc_matches)) => subcommand_graph_to_ogr(sc_matches)?,
            ("covered-area", Some(sc_matches)) => subcommand_graph_covered_area(sc_matches)?,
            ("from-osm-pbf", Some(sc_matches)) => subcommand_from_osm_pbf(sc_matches)?,
            _ => {
                println!("unknown subcommand");
            }
        },
        ("server", Some(sc_matches)) => subcommand_server(sc_matches)?,
        _ => {
            println!("unknown subcommand");
        }
    }
    Ok(())
}

fn subcommand_graph_to_ogr(sc_matches: &ArgMatches) -> Result<()> {
    let graph_filename = sc_matches.value_of("GRAPH").unwrap().to_string();
    let graph: H3EdgeGraph<Weight> = arrow_load_graph(File::open(graph_filename)?)?;
    graph.ogr_write(
        sc_matches.value_of("driver").unwrap(),
        sc_matches.value_of("OUTPUT").unwrap(),
        sc_matches.value_of("layer_name").unwrap(),
    )?;
    Ok(())
}

fn subcommand_graph_covered_area(sc_matches: &ArgMatches) -> Result<()> {
    let graph_filename = sc_matches.value_of("GRAPH").unwrap().to_string();
    let graph: H3EdgeGraph<Weight> = arrow_load_graph(File::open(graph_filename)?)?;

    let mut outfile = File::create(sc_matches.value_of("OUT-GEOJSON").unwrap())?;
    let multi_poly = graph.covered_area()?;
    let gj_geom = geojson::Geometry::try_from(&multi_poly)?;
    outfile.write_all(gj_geom.to_string().as_ref())?;

    outfile.flush()?;
    Ok(())
}

fn subcommand_server(sc_matches: &ArgMatches) -> Result<()> {
    let config_contents = std::fs::read_to_string(sc_matches.value_of("CONFIG-FILE").unwrap())?;
    let config = toml::from_str(&config_contents)?;
    crate::server::launch_server(config)?;
    Ok(())
}

fn subcommand_from_osm_pbf(sc_matches: &ArgMatches) -> Result<()> {
    let h3_resolution: u8 = sc_matches.value_of("h3_resolution").unwrap().parse()?;
    let graph_output = sc_matches.value_of("OUTPUT-GRAPH").unwrap().to_string();

    let mut builder = OsmPbfH3EdgeGraphBuilder::new(h3_resolution, way_properties);
    for pbf_input in sc_matches.values_of("OSM-PBF").unwrap() {
        builder.read_pbf(Path::new(&pbf_input))?;
    }
    let graph = builder.build_graph()?;

    log::info!(
        "Created graph ({} nodes, {} edges)",
        graph.num_nodes(),
        graph.num_edges()
    );
    let mut out_file = File::create(graph_output)?;
    arrow_save_graph(&graph, &mut out_file)?;
    Ok(())
}
