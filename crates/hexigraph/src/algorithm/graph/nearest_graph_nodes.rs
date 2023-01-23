use crate::error::Error;
use crate::graph::node::NodeType;
use crate::graph::GetCellNode;
use h3o::CellIndex;

/// find the nearest nodes in the graph
pub trait NearestGraphNodes {
    /// get an iterator over the closest corresponding nodes in the graph to the
    /// given cell. The iterator will return all nodes with the
    /// same, smallest `k` <= `max_distance_k` which are part of the graph.
    fn nearest_graph_nodes(
        &self,
        cell: CellIndex,
        max_distance_k: u32,
    ) -> Result<NearestGraphNodesGetCellIter<Self>, Error>
    where
        Self: Sized;
}

pub struct NearestGraphNodesGetCellIter<'a, G> {
    graph: &'a G,
    neighbors_reversed: Vec<(CellIndex, u32)>,
    found_max_k: u32,
}

impl<'a, G> Iterator for NearestGraphNodesGetCellIter<'a, G>
where
    G: GetCellNode,
{
    type Item = (CellIndex, NodeType, u32);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((neighbor_cell, neighbor_k)) = self.neighbors_reversed.pop() {
            if neighbor_k > self.found_max_k {
                break;
            }

            if let Some(node_type) = self.graph.get_cell_node(neighbor_cell) {
                self.found_max_k = neighbor_k;
                return Some((neighbor_cell, node_type, neighbor_k));
            }
        }
        None
    }
}

impl<G> NearestGraphNodes for G
where
    G: GetCellNode,
{
    fn nearest_graph_nodes(
        &self,
        cell: CellIndex,
        max_distance_k: u32,
    ) -> Result<NearestGraphNodesGetCellIter<G>, Error> {
        let mut neighbors: Vec<_> = cell.grid_disk_distances(max_distance_k);

        // reverse the order to gave the nearest neighbors first
        neighbors.sort_unstable_by_key(|(_, k)| max_distance_k - *k);

        Ok(NearestGraphNodesGetCellIter {
            graph: self,
            neighbors_reversed: neighbors,
            found_max_k: max_distance_k,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::{CellSet, HashSet};
    use crate::graph::node::NodeType;
    use crate::graph::GetCellNode;
    use h3o::CellIndex;

    impl GetCellNode for HashSet<CellIndex> {
        fn get_cell_node(&self, cell: CellIndex) -> Option<NodeType> {
            self.get(&cell).map(|_| NodeType::OriginAndDestination)
        }
    }

    #[test]
    fn nearest_finds_given_cell_first() {
        let cell: CellIndex = 0x89283080ddbffff_u64.try_into().unwrap();
        let cellset: HashSet<_> = cell.grid_disk(3);
        assert_eq!(cellset.nearest_graph_nodes(cell, 3).unwrap().count(), 1);
        assert_eq!(
            cellset.nearest_graph_nodes(cell, 3).unwrap().next(),
            Some((cell, NodeType::OriginAndDestination, 0))
        );
    }

    #[test]
    fn nearest_finds_all_with_same_k() {
        let cell = CellIndex::try_from(0x89283080ddbffff_u64).unwrap();
        let mut cellset = CellSet::default();
        let mut expected = CellSet::default();
        let distances: Vec<_> = cell.grid_disk_distances(3);
        for (ring_cell, _) in distances.into_iter().filter(|(_, k)| *k >= 2).take(2) {
            cellset.insert(ring_cell);
            expected.insert(ring_cell);
        }
        let distances: Vec<_> = cell.grid_disk_distances(5);
        for (ring_cell, _) in distances.into_iter().filter(|(_, k)| *k >= 4).take(2) {
            cellset.insert(ring_cell);
        }
        assert_eq!(cellset.nearest_graph_nodes(cell, 8).unwrap().count(), 2);
        for (nearest_cell, _, _) in cellset.nearest_graph_nodes(cell, 8).unwrap() {
            assert!(expected.contains(&nearest_cell));
        }
    }
}
