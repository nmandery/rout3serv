use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::Path;

use eyre::Result;
use osmpbfreader::{OsmPbfReader, Tags};

use route3_core::geo_types::{Coordinate, LineString};
use route3_core::graph::{EdgeProperties, GraphBuilder, H3Graph};
use route3_core::h3ron::H3Cell;
use route3_core::indexmap::set::IndexSet;
use route3_core::{fast_paths, h3ron};

pub struct OsmPbfGraphBuilder<F: Fn(&Tags) -> Option<EdgeProperties>> {
    h3_resolution: u8,
    edge_properties_fn: F,

    // hashmaps for deduplicating edges
    edges_weight: HashMap<(usize, usize), usize>,
    edges_weight_bidir: HashMap<(usize, usize), usize>,

    cell_nodes: IndexSet<H3Cell>,
}

impl<F> OsmPbfGraphBuilder<F>
where
    F: Fn(&Tags) -> Option<EdgeProperties>,
{
    pub fn new(h3_resolution: u8, edge_properties_fn: F) -> Self {
        Self {
            h3_resolution,
            edge_properties_fn,
            edges_weight: Default::default(),
            edges_weight_bidir: Default::default(),
            cell_nodes: Default::default(),
        }
    }

    fn set_edge_weight(
        &mut self,
        node_cell1: usize,
        node_cell2: usize,
        weight: usize,
        bidirectional: bool,
    ) {
        if bidirectional {
            &mut self.edges_weight_bidir
        } else {
            &mut self.edges_weight
        }
        .entry((node_cell1, node_cell2))
        .and_modify(|v| {
            // lower weights are preferred
            if *v > weight {
                *v = weight
            }
        })
        .or_insert(weight);
    }

    pub fn read_pbf(&mut self, pbf_path: &Path) -> Result<()> {
        let pbf_file = std::fs::File::open(pbf_path)?;
        let mut pbf = OsmPbfReader::new(pbf_file);
        let mut nodeid_coordinates: HashMap<_, _> = Default::default();
        for obj_result in pbf.iter() {
            let obj = obj_result?;
            match obj {
                osmpbfreader::OsmObj::Node(node) => {
                    let coordinate = Coordinate {
                        x: node.lon(),
                        y: node.lat(),
                    };
                    nodeid_coordinates.insert(node.id, coordinate);
                }
                osmpbfreader::OsmObj::Way(way) => {
                    if let Some(edge_props) = (self.edge_properties_fn)(&way.tags) {
                        let coordinates: Vec<_> = way
                            .nodes
                            .iter()
                            .filter_map(|node_id| nodeid_coordinates.get(node_id).cloned())
                            .collect();
                        if coordinates.len() >= 2 {
                            let mut h3indexes =
                                h3ron::line(&LineString::from(coordinates), self.h3_resolution)?;
                            h3indexes.dedup();

                            for window in h3indexes.windows(2) {
                                let (node_cell1, _) =
                                    self.cell_nodes.insert_full(H3Cell::try_from(window[0])?);
                                let (node_cell2, _) =
                                    self.cell_nodes.insert_full(H3Cell::try_from(window[1])?);

                                self.set_edge_weight(
                                    node_cell1,
                                    node_cell2,
                                    edge_props.weight,
                                    edge_props.is_bidirectional,
                                );
                            }
                        }
                    }
                }
                osmpbfreader::OsmObj::Relation(_) => {}
            }
        }
        Ok(())
    }
}

impl<F> GraphBuilder for OsmPbfGraphBuilder<F>
where
    F: Fn(&Tags) -> Option<EdgeProperties>,
{
    fn build_graph(mut self) -> Result<H3Graph> {
        let mut input_graph = fast_paths::InputGraph::new();

        for ((node_id1, node_id2), weight) in self.edges_weight_bidir.drain() {
            input_graph.add_edge_bidir(node_id1, node_id2, weight);
        }
        for ((node_id1, node_id2), weight) in self.edges_weight.drain() {
            input_graph.add_edge(node_id1, node_id2, weight);
        }
        input_graph.freeze();
        let graph = fast_paths::prepare(&input_graph);

        Ok(H3Graph {
            input_graph,
            graph,
            cell_nodes: self.cell_nodes,
            h3_resolution: self.h3_resolution,
        })
    }
}
