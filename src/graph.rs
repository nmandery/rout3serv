use eyre::Result;
use h3ron::H3Cell;
use indexmap::set::IndexSet;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Graph {
    pub input_graph: fast_paths::InputGraph,
    pub graph: fast_paths::FastGraph,

    /// lookup to correlate the continuous NodeIds of the FastGraph
    /// to H3Cells
    pub cell_nodes: IndexSet<H3Cell>,
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
    /// check if fast_paths routes in both directions or just in one
    fn fast_paths_bidirectional_routing() {
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
