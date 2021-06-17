use std::collections::HashSet;

use eyre::{Report, Result};
use h3ron::H3Cell;
use indexmap::set::IndexSet;
use serde::{Deserialize, Serialize};

pub struct EdgeProperties {
    pub is_bidirectional: bool,
    pub weight: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Graph {
    pub input_graph: fast_paths::InputGraph,
    pub graph: fast_paths::FastGraph,

    /// lookup to correlate the continuous NodeIds of the FastGraph
    /// to H3Cells
    pub cell_nodes: IndexSet<H3Cell>,
    pub h3_resolution: u8,
}

impl Graph {
    pub fn build_graph_without_cells(&self, cells: &[H3Cell]) -> Result<fast_paths::FastGraph> {
        let mut modified_input_graph = fast_paths::InputGraph::new();

        let cell_nodes_to_exclude: HashSet<_> = cells
            .iter()
            .filter_map(|cell| self.cell_nodes.get_full(cell).map(|(node, _)| node))
            .collect();

        for edge in self.input_graph.get_edges() {
            modified_input_graph.add_edge(
                edge.from,
                edge.to,
                if cell_nodes_to_exclude.contains(&edge.from)
                    || cell_nodes_to_exclude.contains(&edge.to)
                {
                    fast_paths::WEIGHT_MAX
                } else {
                    edge.weight
                },
            );
        }
        modified_input_graph.freeze();
        let modified_graph =
            fast_paths::prepare_with_order(&modified_input_graph, &self.graph.get_node_ordering())
                .map_err(Report::msg)?;

        Ok(modified_graph)
    }
}

pub trait GraphBuilder {
    fn build_graph(self) -> Result<Graph>;
}

impl Graph {
    pub fn h3cell_by_nodeid(&self, node_id: usize) -> Result<&H3Cell> {
        Ok(self.cell_nodes.get_index(node_id).unwrap()) // TODO
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn fast_paths_bidirectional_routing() {
        let mut input_graph = fast_paths::InputGraph::new();
        input_graph.add_edge_bidir(1, 2, 10);
        input_graph.freeze();

        let graph = fast_paths::prepare(&input_graph);
        let p1 = fast_paths::calc_path(&graph, 1, 2);
        assert!(p1.is_some());

        let p2 = fast_paths::calc_path(&graph, 2, 1);
        assert!(p2.is_some());
    }

    #[test]
    fn fast_paths_unidirectional_routing() {
        let mut input_graph = fast_paths::InputGraph::new();
        input_graph.add_edge(1, 2, 10);
        input_graph.freeze();

        let graph = fast_paths::prepare(&input_graph);
        let p1 = fast_paths::calc_path(&graph, 1, 2);
        assert!(p1.is_some());

        let p2 = fast_paths::calc_path(&graph, 2, 1);
        assert!(p2.is_none());
    }
}
