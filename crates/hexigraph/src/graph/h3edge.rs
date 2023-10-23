use std::ops::Add;

use ahash::RandomState;
use geo::MultiPolygon;
use h3o::{CellIndex, DirectedEdgeIndex, Resolution};
use hashbrown::hash_map::Entry;
use tracing::debug;

use crate::algorithm::graph::covered_area::cells_covered_area;
use crate::algorithm::graph::CoveredArea;
use crate::container::{CellMap, DirectedEdgeMap};
use crate::error::Error;
use crate::graph::node::NodeType;
use crate::graph::{EdgeWeight, GetEdge, GetStats};
use crate::HasH3Resolution;

use super::GraphStats;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct H3EdgeGraph<W> {
    pub edges: DirectedEdgeMap<W>,
    pub h3_resolution: Resolution,
}

impl<W> H3EdgeGraph<W>
where
    W: PartialOrd + PartialEq + Add + Copy,
{
    pub fn new(h3_resolution: Resolution) -> Self {
        Self {
            h3_resolution,
            edges: Default::default(),
        }
    }

    ///
    /// This has to generate the node list first, so its rather
    /// expensive to call.
    pub fn num_nodes(&self) -> usize {
        self.nodes().len()
    }

    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }

    pub fn edge_weight(&self, edge: DirectedEdgeIndex) -> Option<&W> {
        self.edges.get(&edge)
    }

    /// get all edges in the graph leading from this edge to neighbors
    pub fn edges_from_cell(
        &self,
        cell: CellIndex,
    ) -> impl Iterator<Item = (&DirectedEdgeIndex, &W)> {
        cell.edges()
            .filter_map(|edge| self.edges.get_key_value(&edge))
    }

    pub fn add_edge(&mut self, edge: DirectedEdgeIndex, weight: W) {
        match self.edges.entry(edge) {
            Entry::Occupied(mut occ) => {
                if &weight < occ.get() {
                    // lower weight takes precedence
                    occ.insert(weight);
                }
            }
            Entry::Vacant(vac) => {
                vac.insert(weight);
            }
        }
    }

    pub fn try_add(&mut self, other: Self) -> Result<(), Error> {
        if self.h3_resolution != other.h3_resolution {
            return Err(Error::MixedH3Resolutions(
                self.h3_resolution,
                other.h3_resolution,
            ));
        }
        for (edge, weight) in other.edges.into_iter() {
            self.add_edge(edge, weight);
        }
        Ok(())
    }

    /// cells which are valid targets to route to
    ///
    /// This is a rather expensive operation as nodes are not stored anywhere
    /// and need to be extracted from the edges.
    pub fn nodes(&self) -> CellMap<NodeType> {
        debug!(
            "extracting nodes from the graph edges @ r={}",
            self.h3_resolution
        );
        extract_nodes(&self.edges)
    }

    pub fn iter_edges(&self) -> impl Iterator<Item = (DirectedEdgeIndex, &W)> {
        self.edges.iter().map(|(edge, weight)| (*edge, weight))
    }
}

fn extract_nodes<W>(edge_map: &DirectedEdgeMap<W>) -> CellMap<NodeType> {
    let mut cells = CellMap::with_capacity_and_hasher(edge_map.len(), RandomState::default());
    for edge in edge_map.keys() {
        let cell_from = edge.origin();
        cells
            .entry(cell_from)
            .and_modify(|node_type| *node_type += NodeType::Origin)
            .or_insert(NodeType::Origin);

        let cell_to = edge.destination();
        cells
            .entry(cell_to)
            .and_modify(|node_type| *node_type += NodeType::Destination)
            .or_insert(NodeType::Destination);
    }
    cells
}

impl<W> HasH3Resolution for H3EdgeGraph<W> {
    fn h3_resolution(&self) -> Resolution {
        self.h3_resolution
    }
}

impl<W> GetStats for H3EdgeGraph<W>
where
    W: PartialEq + PartialOrd + Add + Copy,
{
    fn get_stats(&self) -> Result<GraphStats, Error> {
        Ok(GraphStats {
            h3_resolution: self.h3_resolution,
            num_nodes: self.num_nodes(),
            num_edges: self.num_edges(),
        })
    }
}

impl<W> GetEdge for H3EdgeGraph<W>
where
    W: Copy,
{
    type EdgeWeightType = W;

    fn get_edge(&self, edge: DirectedEdgeIndex) -> Option<EdgeWeight<Self::EdgeWeightType>> {
        self.edges.get(&edge).map(|w| EdgeWeight::from(*w))
    }
}

impl<W> CoveredArea for H3EdgeGraph<W>
where
    W: PartialOrd + PartialEq + Add + Copy,
{
    type Error = Error;

    fn covered_area(&self, reduce_resolution_by: u8) -> Result<MultiPolygon<f64>, Self::Error> {
        cells_covered_area(
            self.nodes().iter().map(|(cell, _)| *cell),
            self.h3_resolution(),
            reduce_resolution_by,
        )
    }
}

/// change the resolution of a graph to a lower resolution
///
/// the `weight_selector_fn` decides which weight is assigned to a downsampled edge
/// by selecting a weight from all full-resolution edges crossing the new cells boundary.
///
/// This has the potential to change the graphs topology as multiple edges get condensed into one.
/// So for example routing results may differ in parts, but the computation time will be reduced by
/// the reduced number of nodes and edges.
pub fn downsample_graph<W, F>(
    graph: &H3EdgeGraph<W>,
    target_h3_resolution: Resolution,
    weight_selector_fn: F,
) -> Result<H3EdgeGraph<W>, Error>
where
    W: Sync + Send + Copy,
    F: Fn(W, W) -> W + Sync + Send,
{
    if target_h3_resolution >= graph.h3_resolution {
        return Err(Error::TooHighH3Resolution(target_h3_resolution));
    }
    debug!(
        "downsampling graph from r={} to r={}",
        graph.h3_resolution, target_h3_resolution
    );

    let mut downsampled_edges = DirectedEdgeMap::with_capacity_and_hasher(
        graph.edges.len()
            / (u8::from(graph.h3_resolution).saturating_sub(u8::from(target_h3_resolution))
                as usize)
                .pow(7),
        RandomState::default(),
    );

    for (edge, weight) in graph.edges.iter() {
        let (cell_from, cell_to) = edge.cells();
        let cell_from = cell_from.parent(target_h3_resolution).unwrap();
        let cell_to = cell_to.parent(target_h3_resolution).unwrap();
        if cell_from != cell_to {
            let downsampled_edge = cell_from.edge(cell_to).unwrap();

            match downsampled_edges.entry(downsampled_edge) {
                Entry::Occupied(mut occ) => {
                    occ.insert(weight_selector_fn(*occ.get(), *weight));
                }
                Entry::Vacant(vac) => {
                    vac.insert(*weight);
                }
            }
        }
    }
    Ok(H3EdgeGraph {
        edges: downsampled_edges,
        h3_resolution: target_h3_resolution,
    })
}

pub trait H3EdgeGraphBuilder<W>
where
    W: PartialOrd + PartialEq + Add + Copy,
{
    fn build_graph(self) -> Result<H3EdgeGraph<W>, Error>;
}

#[cfg(test)]
mod tests {
    use std::cmp::min;

    use geo::{Coord, LineString};
    use h3o::geom::{PolyfillConfig, ToCells};
    use h3o::{LatLng, Resolution};

    use super::{downsample_graph, H3EdgeGraph, NodeType};

    #[test]
    fn test_downsample() {
        let full_h3_res = Resolution::Eight;
        let cells: Vec<_> = h3o::geom::LineString::from_degrees(LineString::from(vec![
            Coord::from((23.3, 12.3)),
            Coord::from((24.2, 12.2)),
        ]))
        .unwrap()
        .to_cells(PolyfillConfig::new(full_h3_res))
        .collect();
        assert!(cells.len() > 100);

        let mut graph = H3EdgeGraph::new(full_h3_res);
        for w in cells.windows(2) {
            let edge = w[0].edge(w[1]).unwrap();
            graph.add_edge(edge, 20);
        }
        assert!(graph.num_edges() > 50);
        let downsampled_graph = downsample_graph(&graph, Resolution::Five, min).unwrap();
        assert!(downsampled_graph.num_edges() > 0);
        assert!(downsampled_graph.num_edges() < 20);
    }

    #[test]
    fn test_graph_nodes() {
        let res = Resolution::Eight;
        let origin = LatLng::try_from(Coord::from((23.3, 12.3)))
            .unwrap()
            .to_cell(res);
        let edges: Vec<_> = origin
            .edges()
            .map(|edge| (edge, edge.destination()))
            .collect();

        let mut graph = H3EdgeGraph::new(res);
        graph.add_edge(edges[0].0, 1);
        graph.add_edge(edges[1].0, 1);

        let edges2: Vec<_> = edges[1]
            .1
            .edges()
            .map(|edge| (edge, edge.destination()))
            .collect();
        graph.add_edge(edges2[0].0, 1);

        let nodes = graph.nodes();
        assert_eq!(nodes.len(), 4);
        assert_eq!(nodes.get(&origin), Some(&NodeType::Origin));
        assert_eq!(nodes.get(&edges[0].1), Some(&NodeType::Destination));
        assert_eq!(
            nodes.get(&edges[1].1),
            Some(&NodeType::OriginAndDestination)
        );
        assert_eq!(nodes.get(&edges2[0].1), Some(&NodeType::Destination));
    }
}
