use crate::error::Error;
pub use h3edge::{H3EdgeGraph, H3EdgeGraphBuilder};
use h3o::{CellIndex, DirectedEdgeIndex, Resolution};
use node::NodeType;
pub use prepared::PreparedH3EdgeGraph;

use crate::graph::fastforward::FastForward;

pub mod fastforward;
pub mod h3edge;
pub mod modifiers;
pub mod node;
pub mod prepared;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GraphStats {
    pub h3_resolution: Resolution,
    pub num_nodes: usize,
    pub num_edges: usize,
}

pub trait GetStats {
    fn get_stats(&self) -> Result<GraphStats, Error>;
}

pub trait GetCellNode {
    fn get_cell_node(&self, cell: CellIndex) -> Option<NodeType>;
}

pub trait IterateCellNodes<'a> {
    type CellNodeIterator;
    fn iter_cell_nodes(&'a self) -> Self::CellNodeIterator;
}

pub trait GetCellEdges {
    type EdgeWeightType;

    /// get all edges and their values originating from cell `cell`
    #[allow(clippy::complexity)]
    fn get_edges_originating_from(
        &self,
        cell: CellIndex,
    ) -> Vec<(DirectedEdgeIndex, EdgeWeight<Self::EdgeWeightType>)>;
}

pub trait GetEdge {
    type EdgeWeightType;

    fn get_edge(&self, edge: DirectedEdgeIndex) -> Option<EdgeWeight<Self::EdgeWeightType>>;
}

impl<G> GetEdge for G
where
    G: GetCellEdges,
{
    type EdgeWeightType = G::EdgeWeightType;

    fn get_edge(&self, edge: DirectedEdgeIndex) -> Option<EdgeWeight<Self::EdgeWeightType>> {
        let cell = edge.origin();
        for (found_edge, value) in self.get_edges_originating_from(cell) {
            if edge == found_edge {
                return Some(value);
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct EdgeWeight<'a, W> {
    pub weight: W,

    pub fastforward: Option<(&'a FastForward, W)>,
}

impl<'a, W> From<W> for EdgeWeight<'a, W> {
    fn from(weight: W) -> Self {
        EdgeWeight {
            weight,
            fastforward: None,
        }
    }
}
