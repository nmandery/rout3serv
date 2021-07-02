use std::convert::TryFrom;
use std::ops::Add;

use crate::error::Error;
use crate::graph::{downsample_graph, H3Graph, NodeType};
use crate::h3ron::{H3Cell, Index};
use crate::{H3CellMap, H3CellSet};

struct SearchSpace {
    /// h3 resolution used for the cells of the search space
    h3_resolution: u8,

    cells: H3CellSet,
}

/// searchspace to constrain the area dijkstra is working on
impl SearchSpace {
    /// should only be called with cells with a resolution >= self.h3_resolution
    pub fn contains(&self, cell: &H3Cell) -> bool {
        self.cells
            .contains(&cell.get_parent_unchecked(self.h3_resolution))
    }
}

pub struct RoutingGraph<T> {
    pub graph: H3Graph<T>,
    graph_nodes: H3CellMap<NodeType>,

    downsampled_graph: H3Graph<T>,
    downsampled_graph_nodes: H3CellMap<NodeType>,
}

impl<T> RoutingGraph<T> where T: PartialOrd + PartialEq + Add + Copy + Send {}

impl<T> TryFrom<H3Graph<T>> for RoutingGraph<T>
where
    T: PartialOrd + PartialEq + Add + Copy + Send,
{
    type Error = Error;

    fn try_from(graph: H3Graph<T>) -> std::result::Result<Self, Self::Error> {
        let downsampled_graph =
            downsample_graph(graph.clone(), graph.h3_resolution.saturating_sub(3))?;

        let graph_nodes = graph.nodes()?;
        let downsampled_graph_nodes = downsampled_graph.nodes()?;
        Ok(Self {
            graph,
            graph_nodes,
            downsampled_graph,
            downsampled_graph_nodes,
        })
    }
}
