use crate::container::treemap::H3Treemap;
use h3o::{CellIndex, DirectedEdgeIndex, Resolution};
use std::marker::PhantomData;

use crate::graph::node::NodeType;
use crate::graph::{EdgeWeight, GetCellEdges, GetCellNode};
use crate::HasH3Resolution;

/// wrapper to exclude cells from traversal during routing
pub struct ExcludeCells<'a, G, W> {
    cells_to_exclude: &'a H3Treemap<CellIndex>,
    inner_graph: &'a G,
    phantom_weight: PhantomData<W>,
}

impl<'a, G, W> ExcludeCells<'a, G, W>
where
    G: GetCellNode + GetCellEdges<EdgeWeightType = W> + HasH3Resolution,
{
    pub fn new(inner_graph: &'a G, cells_to_exclude: &'a H3Treemap<CellIndex>) -> Self {
        Self {
            cells_to_exclude,
            inner_graph,
            phantom_weight: Default::default(),
        }
    }
}

impl<'a, G, W> GetCellNode for ExcludeCells<'a, G, W>
where
    G: GetCellNode,
{
    fn get_cell_node(&self, cell: CellIndex) -> Option<NodeType> {
        if self.cells_to_exclude.contains(&cell) {
            None
        } else {
            self.inner_graph.get_cell_node(cell)
        }
    }
}

impl<'a, G, W> GetCellEdges for ExcludeCells<'a, G, W>
where
    G: GetCellEdges<EdgeWeightType = W>,
{
    type EdgeWeightType = G::EdgeWeightType;

    fn get_edges_originating_from(
        &self,
        cell: CellIndex,
    ) -> Vec<(DirectedEdgeIndex, EdgeWeight<Self::EdgeWeightType>)> {
        if self.cells_to_exclude.contains(&cell) {
            vec![]
        } else {
            let found = self.inner_graph.get_edges_originating_from(cell);
            let mut not_excluded = Vec::with_capacity(found.len());
            for (edge, edge_value) in found {
                if self.cells_to_exclude.contains(&edge.destination()) {
                    continue;
                }

                // remove the fastforward when it contains any excluded cell
                let filtered_fastforward_opt =
                    if let Some((fastforward, fastforward_weight)) = edge_value.fastforward {
                        if fastforward.is_disjoint(self.cells_to_exclude) {
                            Some((fastforward, fastforward_weight))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                not_excluded.push((
                    edge,
                    EdgeWeight {
                        weight: edge_value.weight,
                        fastforward: filtered_fastforward_opt,
                    },
                ));
            }
            not_excluded
        }
    }
}

impl<'a, G, W> HasH3Resolution for ExcludeCells<'a, G, W>
where
    G: HasH3Resolution,
{
    fn h3_resolution(&self) -> Resolution {
        self.inner_graph.h3_resolution()
    }
}
