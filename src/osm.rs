use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::Path;

use eyre::Result;
use geo_types::{Coordinate, LineString};
use h3ron::H3Cell;
use indexmap::set::IndexSet;
use osmpbfreader::{OsmPbfReader, Tags};

use crate::graph::{EdgeProperties, Graph, GraphBuilder};

pub struct OsmPbfGraphBuilder<F: Fn(&Tags) -> Option<EdgeProperties>> {
    h3_resolution: u8,
    edge_properties_fn: F,
    edges_weight: HashMap<(usize, usize), usize>,
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
            cell_nodes: Default::default(),
        }
    }

    fn set_edge_weight(&mut self, node_cell1: usize, node_cell2: usize, weight: usize) {
        self.edges_weight
            .entry((node_cell1, node_cell2))
            .and_modify(|v| {
                if *v < weight {
                    // TODO: min or max?
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
                                let (cell1, cell2) = ordered_h3index_pair(&window[0], &window[1])?;
                                let (node_cell1, _) = self.cell_nodes.insert_full(cell1);
                                let (node_cell2, _) = self.cell_nodes.insert_full(cell2);

                                if edge_props.is_bidirectional {
                                    self.set_edge_weight(node_cell2, node_cell1, edge_props.weight);
                                }
                                self.set_edge_weight(node_cell1, node_cell2, edge_props.weight);
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

fn ordered_h3index_pair(h3index_1: &u64, h3index_2: &u64) -> Result<(H3Cell, H3Cell)> {
    // have the edges in the same direction, independent of the input
    let (a, b) = if h3index_1 == h3index_2 {
        return Err(eyre::Report::from(h3ron::Error::LineNotComputable));
    } else if h3index_1 < h3index_2 {
        (h3index_1, h3index_2)
    } else {
        (h3index_2, h3index_1)
    };

    Ok((H3Cell::try_from(*a)?, H3Cell::try_from(*b)?))
}

impl<F> GraphBuilder for OsmPbfGraphBuilder<F>
where
    F: Fn(&Tags) -> Option<EdgeProperties>,
{
    fn build_graph(mut self) -> Result<Graph> {
        let mut input_graph = fast_paths::InputGraph::new();

        for ((node_id1, node_id2), weight) in self.edges_weight.drain() {
            input_graph.add_edge(node_id1, node_id2, weight);
        }
        input_graph.freeze();
        let graph = fast_paths::prepare(&input_graph);

        Ok(Graph {
            input_graph,
            graph,
            cell_nodes: self.cell_nodes,
        })
    }
}
