use h3o::{CellIndex, DirectedEdgeIndex, Resolution};
use std::borrow::Borrow;

pub fn reverse_directed_edge(directed_edge: DirectedEdgeIndex) -> DirectedEdgeIndex {
    let (origin, destination) = directed_edge.cells();
    destination.edge(origin).expect("edge not reversable")
}

/// convert an iterator of continuous (= neighboring) cells to edges connecting
/// consecutive cells from the iterator.
pub fn continuous_cells_to_edges<I>(cells: I) -> CellsToEdgesIter<<I as IntoIterator>::IntoIter>
where
    I: IntoIterator,
    I::Item: Borrow<CellIndex>,
{
    CellsToEdgesIter {
        last_cell: None,
        iter: cells.into_iter(),
    }
}

pub struct CellsToEdgesIter<I> {
    last_cell: Option<CellIndex>,
    iter: I,
}

impl<I> Iterator for CellsToEdgesIter<I>
where
    I: Iterator,
    I::Item: Borrow<CellIndex>,
{
    type Item = DirectedEdgeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        for cell_item in self.iter.by_ref() {
            let cell = *cell_item.borrow();
            let last_cell = if let Some(cell) = self.last_cell {
                cell
            } else {
                self.last_cell = Some(cell);
                continue;
            };
            if cell == last_cell {
                // duplicate cell, skipping
                continue;
            }

            let edge_result = last_cell.edge(cell);
            self.last_cell = Some(cell);
            return edge_result;
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (remaining, upper_bound) = self.iter.size_hint();
        (
            remaining.saturating_sub(1),
            upper_bound.map(|ub| ub.saturating_sub(1)),
        )
    }
}

/// The approximate distance between the centroids of two neighboring cells
/// at the given `resolution`.
///
/// Based on the approximate edge length. See [`cell_centroid_distance_m`] for a
/// more exact variant of this function.
pub fn cell_centroid_distance_avg_m_at_resolution(resolution: Resolution) -> f64 {
    cell_centroid_distance_m_by_edge_length(resolution.edge_length_m())
}

/// The approximate distance between the centroids of two neighboring cells
/// at the given `resolution`.
///
/// Based on the exact edge length. See [`cell_centroid_distance_avg_m_at_resolution`]
/// for a resolution based variant.
pub fn cell_centroid_distance_m(edge: DirectedEdgeIndex) -> f64 {
    cell_centroid_distance_m_by_edge_length(edge.length_m())
}

/// avoid repeated calculations by using this constant of the
/// result of `3.0_f64.sqrt()`.
const F64_SQRT_3: f64 = 1.7320508075688772_f64;

/// the height of two equilateral triangles with a shared side calculated using
/// the `edge_length`.
///     .
///    /_\
///    \ /
///     `
///
/// For one triangle:  `h = (edge_length / 2.0) * 3.0.sqrt()`
#[inline(always)]
fn cell_centroid_distance_m_by_edge_length(edge_length: f64) -> f64 {
    edge_length * F64_SQRT_3
}
