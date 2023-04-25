use h3o::CellIndex;
use hashbrown::hash_map::Entry;
use std::borrow::Borrow;
use std::ops::Add;

use crate::algorithm::graph::dijkstra::edge_dijkstra_weight_threshold;
use crate::container::CellMap;
use num_traits::Zero;
use rayon::prelude::*;

use crate::error::Error;
use crate::graph::GetCellEdges;

/// Find all cells connected to the graph around a origin cell within a given threshold
pub trait WithinWeightThreshold<W> {
    /// Find all cells connected to the graph within a given `weight_threshold` around the
    /// given `origin_cell`
    fn cells_within_weight_threshold(
        &self,
        origin_cell: CellIndex,
        weight_threshold: W,
    ) -> Result<CellMap<W>, Error>;
}

impl<W, G> WithinWeightThreshold<W> for G
where
    G: GetCellEdges<EdgeWeightType = W>,
    W: Zero + Ord + Copy + Add,
{
    fn cells_within_weight_threshold(
        &self,
        origin_cell: CellIndex,
        weight_threshold: W,
    ) -> Result<CellMap<W>, Error> {
        edge_dijkstra_weight_threshold(self, origin_cell, weight_threshold)
    }
}

/// Find all cells connected to the graph around a origin cell within a given threshold
pub trait WithinWeightThresholdMany<W> {
    /// Find all cells connected to the graph within a given `weight_threshold` around the
    /// given `origin_cells`.
    ///
    /// The weights for cells which are traversed from multiple `origin_cells` are aggregated using
    /// `agg_fn`. This can be used - for example - to find the minimum or maximum weight for a cell.
    fn cells_within_weight_threshold_many<I, AGG>(
        &self,
        origin_cells: I,
        weight_threshold: W,
        agg_fn: AGG,
    ) -> Result<CellMap<W>, Error>
    where
        I: IntoParallelIterator,
        I::Item: Borrow<CellIndex>,
        AGG: Fn(&mut W, W) + Sync;
}

impl<W, G> WithinWeightThresholdMany<W> for G
where
    G: GetCellEdges<EdgeWeightType = W> + WithinWeightThreshold<W> + Sync,
    W: Zero + Ord + Copy + Add + Send + Sync,
{
    fn cells_within_weight_threshold_many<I, AGG>(
        &self,
        origin_cells: I,
        weight_threshold: W,
        agg_fn: AGG,
    ) -> Result<CellMap<W>, Error>
    where
        I: IntoParallelIterator,
        I::Item: Borrow<CellIndex>,
        AGG: Fn(&mut W, W) + Sync,
    {
        origin_cells
            .into_par_iter()
            .map(|item| self.cells_within_weight_threshold(*item.borrow(), weight_threshold))
            .try_reduce_with(|cellmap1, cellmap2| {
                // select the source and target maps, to move the contents of the map with fewer elements, to the map
                // with more elements. This should save quite a few hashing operations.
                let (source_cellmap, mut target_cellmap) = if cellmap1.len() < cellmap2.len() {
                    (cellmap1, cellmap2)
                } else {
                    (cellmap2, cellmap1)
                };

                for (cell, weight) in source_cellmap {
                    match target_cellmap.entry(cell) {
                        Entry::Occupied(mut entry) => {
                            agg_fn(entry.get_mut(), weight);
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(weight);
                        }
                    };
                }
                Ok(target_cellmap)
            })
            .unwrap_or_else(|| Ok(Default::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithm::edge::continuous_cells_to_edges;
    use crate::container::HashMap;
    use crate::graph::{GetStats, H3EdgeGraph, PreparedH3EdgeGraph};
    use geo::Line;
    use h3o::geom::ToCells;
    use h3o::Resolution;

    /// a simple graph consisting of a single line
    fn line_graph(default_weight: u32) -> (Vec<CellIndex>, PreparedH3EdgeGraph<u32>) {
        let h3_resolution = Resolution::Four;
        let cell_sequence: Vec<_> = h3o::geom::Line::from_degrees(Line {
            start: (10.0f64, 20.0f64).into(),
            end: (20., 20.).into(),
        })
        .unwrap()
        .to_cells(h3_resolution)
        .collect();

        let mut g = H3EdgeGraph::new(h3_resolution);
        for edge in continuous_cells_to_edges(&cell_sequence) {
            g.add_edge(edge, default_weight);
        }
        (cell_sequence, g.try_into().unwrap())
    }

    #[test]
    fn test_cells_within_weight_threshold() {
        let (cell_sequence, prepared_graph) = line_graph(10);
        assert!(prepared_graph.get_stats().unwrap().num_edges > 10);
        let within_threshold = prepared_graph
            .cells_within_weight_threshold(cell_sequence[0], 30)
            .unwrap();
        assert_eq!(within_threshold.len(), 4);
        let weights: Vec<_> = within_threshold.values().copied().collect();
        assert!(weights.contains(&0));
        assert!(weights.contains(&10));
        assert!(weights.contains(&20));
        assert!(weights.contains(&30));
    }

    #[test]
    fn test_cells_within_weight_threshold_many() {
        let (cell_sequence, prepared_graph) = line_graph(10);
        assert!(prepared_graph.get_stats().unwrap().num_edges > 20);

        let origin_cells = vec![
            cell_sequence[0],
            cell_sequence[1], // overlaps with the previous cell
            cell_sequence[10],
        ];

        let within_threshold = prepared_graph
            .cells_within_weight_threshold_many(
                origin_cells,
                30,
                // use the minimum weight encountered
                |existing, new| {
                    if new < *existing {
                        *existing = new
                    }
                },
            )
            .unwrap();
        assert_eq!(within_threshold.len(), 9);
        let weights_freq =
            within_threshold
                .iter()
                .fold(HashMap::default(), |mut agg, (_, weight)| {
                    agg.entry(weight).and_modify(|c| *c += 1).or_insert(1u32);
                    agg
                });
        assert_eq!(weights_freq[&0], 3);
        assert_eq!(weights_freq[&10], 2);
        assert_eq!(weights_freq[&20], 2);
        assert_eq!(weights_freq[&30], 2);
    }
}
