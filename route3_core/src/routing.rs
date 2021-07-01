use std::convert::TryFrom;
use std::ops::Add;

use crate::error::Error;
use crate::graph::{H3Graph, NodeType};
use crate::h3ron::{H3Cell, Index};
use crate::{H3CellMap, H3CellSet};

struct SearchSpace {
    /// h3 resolution used for the cells of the search space
    h3_resolution: u8,

    cells: H3CellSet,
}

impl SearchSpace {
    /// should only be called with cells with a resolution >= self.h3_resolution
    pub fn contains(&self, cell: &H3Cell) -> Result<bool, Error> {
        let parent_cell = cell.get_parent(self.h3_resolution)?;
        Ok(self.cells.contains(&parent_cell))
    }
}

pub struct RoutingGraph<T> {
    pub graph: H3Graph<T>,
    graph_nodes: H3CellMap<NodeType>,

    downsampled_graph: H3Graph<T>,
    downsampled_graph_nodes: H3CellMap<NodeType>,
}

impl<T> RoutingGraph<T> where T: PartialOrd + PartialEq + Add + Copy {}

impl<T> TryFrom<H3Graph<T>> for RoutingGraph<T>
where
    T: PartialOrd + PartialEq + Add + Copy,
{
    type Error = Error;

    fn try_from(graph: H3Graph<T>) -> std::result::Result<Self, Self::Error> {
        let downsampled_graph = graph.downsample(graph.h3_resolution.saturating_sub(3))?;

        let graph_nodes = graph.nodes()?;
        let downsampled_graph_nodes = downsampled_graph.nodes()?;
        Ok(Self {
            graph,
            downsampled_graph,
            graph_nodes,
            downsampled_graph_nodes,
        })
    }
}
