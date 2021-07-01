use std::collections::HashMap;
use std::convert::TryFrom;
use std::ops::Add;
use std::path::Path;

use eyre::Result;
use osmpbfreader::{OsmPbfReader, Tags};

use route3_core::error::Error;
use route3_core::geo_types::{Coordinate, LineString};
use route3_core::graph::{GraphBuilder, H3Graph};
use route3_core::h3ron;
use route3_core::h3ron::H3Cell;

pub struct EdgeProperties<T> {
    pub is_bidirectional: bool,
    pub weight: T,
}

pub struct OsmPbfGraphBuilder<
    T: PartialOrd + PartialEq + Add + Copy,
    F: Fn(&Tags) -> Option<EdgeProperties<T>>,
> {
    h3_resolution: u8,
    edge_properties_fn: F,
    graph: H3Graph<T>,
}

impl<T, F> OsmPbfGraphBuilder<T, F>
where
    T: PartialOrd + PartialEq + Add + Copy,
    F: Fn(&Tags) -> Option<EdgeProperties<T>>,
{
    pub fn new(h3_resolution: u8, edge_properties_fn: F) -> Self {
        Self {
            h3_resolution,
            edge_properties_fn,
            graph: H3Graph::new(h3_resolution),
        }
    }

    pub fn read_pbf(&mut self, pbf_path: &Path) -> Result<()> {
        let pbf_file = std::fs::File::open(pbf_path)?;
        let mut pbf = OsmPbfReader::new(pbf_file);
        let mut nodeid_coordinates: HashMap<_, _> = Default::default();
        for obj_result in pbf.par_iter() {
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
                                let cell1 = H3Cell::try_from(window[0])?;
                                let cell2 = H3Cell::try_from(window[1])?;

                                if edge_props.is_bidirectional {
                                    self.graph.add_edge_using_cells_bidirectional(
                                        cell1,
                                        cell2,
                                        edge_props.weight,
                                    )?;
                                } else {
                                    self.graph.add_edge_using_cells(
                                        cell1,
                                        cell2,
                                        edge_props.weight,
                                    )?;
                                }
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

impl<T, F> GraphBuilder<T> for OsmPbfGraphBuilder<T, F>
where
    T: PartialOrd + PartialEq + Add + Copy,
    F: Fn(&Tags) -> Option<EdgeProperties<T>>,
{
    fn build_graph(self) -> std::result::Result<H3Graph<T>, Error> {
        Ok(self.graph)
    }
}