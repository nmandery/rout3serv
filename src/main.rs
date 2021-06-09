mod graph;
mod osm;

use crate::graph::BuildGraph;
use osmpbfreader::Tags;
use std::path::Path;

const H3_RES: u8 = 10;

fn way_weight(tags: &Tags) -> Option<usize> {
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
            "service" | "track" => Some(1),
            _ => None,
        }
    } else {
        None
    }
}

fn main() -> eyre::Result<()> {
    env_logger::init();
    let args: Vec<_> = std::env::args_os().collect();

    let mut builder = crate::osm::OsmPbfGraphBuilder::new(H3_RES, way_weight);
    builder.read_pbf(Path::new(&args[1]))?;
    let graph = builder.build_graph()?;
    graph.ogr_write("FlatGeobuf", "/tmp/graph.fgb", "graph")?;
    Ok(())
}
