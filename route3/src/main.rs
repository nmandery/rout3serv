use std::convert::TryFrom;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use clap::{App, Arg, ArgMatches, SubCommand};
use eyre::Result;
use geo::algorithm::simplify::Simplify;
use geo_types::{MultiPolygon, Polygon};
use h3ron::{Index, ToLinkedPolygons};
use osmpbfreader::Tags;

use crate::graph::{EdgeProperties, GraphBuilder};
#[cfg(feature = "gdal")]
use crate::io::OgrWrite;
use crate::io::{load_graph, print_graph_stats, save_graph_to_file};

mod graph;
mod io;
mod osm;
mod server;

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

    let mut app = App::new(env!("CARGO_PKG_NAME"))
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
            SubCommand::with_name("graph-covered-area")
                .about("Extract the area covered by the graph as geojson")
                .arg(Arg::with_name("GRAPH").help("graph").required(true))
                .arg(
                    Arg::with_name("OUT-GEOJSON")
                        .help("output file to write the geojson geometry to")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("server")
                .about("start the GRPC server")
                .arg(
                    Arg::with_name("CONFIG-FILE")
                        .help("server configuration file")
                        .required(true),
                ),
        );

    if cfg!(feature = "gdal") {
        app = app.subcommand(
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
    }

    let matches = app.get_matches();

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

            let mut out_file = File::create(graph_output)?;
            save_graph_to_file(&graph, &mut out_file)?;
        }
        ("graph-stats", Some(sc_matches)) => {
            let graph_filename = sc_matches.value_of("GRAPH").unwrap().to_string();
            let _ = load_graph(File::open(graph_filename)?)?;
        }
        ("graph-to-ogr", Some(sc_matches)) => subcommand_graph_to_ogr(sc_matches)?,
        ("graph-covered-area", Some(sc_matches)) => subcommand_graph_covered_area(sc_matches)?,
        ("server", Some(sc_matches)) => subcommand_server(sc_matches)?,
        _ => unreachable!(),
    }
    Ok(())
}

#[cfg(feature = "gdal")]
fn subcommand_graph_to_ogr(sc_matches: &ArgMatches) -> Result<()> {
    let graph_filename = sc_matches.value_of("GRAPH").unwrap().to_string();
    let graph = load_graph(File::open(graph_filename)?)?;
    graph.ogr_write(
        sc_matches.value_of("driver").unwrap(),
        sc_matches.value_of("OUTPUT").unwrap(),
        sc_matches.value_of("layer_name").unwrap(),
    )?;
    Ok(())
}

#[cfg(not(feature = "gdal"))]
fn subcommand_graph_to_ogr(_sc_matches: &ArgMatches) -> Result<()> {
    unimplemented!("binary is build without gdal support")
}

fn subcommand_graph_covered_area(sc_matches: &ArgMatches) -> Result<()> {
    let graph_filename = sc_matches.value_of("GRAPH").unwrap().to_string();
    let graph = load_graph(File::open(graph_filename)?)?;

    let mut outfile = File::create(sc_matches.value_of("OUT-GEOJSON").unwrap())?;
    let multi_poly = {
        // using a lower resolution, to have less cells to merge
        let t_res = graph.h3_resolution.saturating_sub(3);
        let mut cells: Vec<_> = graph
            .cell_nodes
            .iter()
            .map(|cell| cell.get_parent(t_res))
            .collect::<Result<_, h3ron::Error>>()?;
        cells.sort_unstable();
        cells.dedup();

        MultiPolygon::from(
            cells
                // remove the number of vertices by smoothing
                .to_linked_polygons(true)
                .drain(..)
                // reduce the number of vertices again and discard all holes
                .map(|p| Polygon::new(p.exterior().simplify(&0.000001), vec![]))
                .collect::<Vec<_>>(),
        )
    };
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
