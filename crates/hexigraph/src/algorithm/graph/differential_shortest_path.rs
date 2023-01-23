use ahash::RandomState;
use h3o::{CellIndex, Resolution};
use std::borrow::Borrow;
use std::ops::Add;

use num_traits::Zero;

use crate::algorithm::graph::path::Path;
use crate::algorithm::graph::shortest_path::ShortestPathOptions;
use crate::algorithm::graph::{NearestGraphNodes, ShortestPathManyToMany};

use crate::container::treemap::H3Treemap;
use crate::container::CellMap;
use crate::error::Error;
use crate::graph::modifiers::ExcludeCells;
use crate::graph::{GetCellEdges, GetCellNode};
use crate::HasH3Resolution;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExclusionDiff<T> {
    /// the results of the shortest-path calculation before the cells have been
    /// excluded from the graph.
    pub before_cell_exclusion: Vec<T>,

    /// the results of the shortest-path calculation after the cells have been
    /// excluded from the graph.
    pub after_cell_exclusion: Vec<T>,
}

/// "Differential" routing calculates the shortest path from (multiple) origin cells
/// to the `N` nearest destinations.
/// This done once to the un-modified graph, and once the the graph with a set of nodes
/// being removed, the `exclude_cells` parameter.
pub trait DifferentialShortestPath<W>
where
    W: Send + Sync + Ord + Copy,
{
    fn differential_shortest_path<I, OPT>(
        &self,
        origin_cells: I,
        destination_cells: I,
        exclude_cells: &H3Treemap<CellIndex>,
        options: &OPT,
    ) -> Result<CellMap<ExclusionDiff<Path<W>>>, Error>
    where
        I: IntoIterator,
        I::Item: Borrow<CellIndex>,
        OPT: ShortestPathOptions + Send + Sync,
    {
        self.differential_shortest_path_map(
            origin_cells,
            destination_cells,
            exclude_cells,
            options,
            Ok,
        )
    }

    fn differential_shortest_path_map<I, OPT, PM, O>(
        &self,
        origin_cells: I,
        destination_cells: I,
        exclude_cells: &H3Treemap<CellIndex>,
        options: &OPT,
        path_transform_fn: PM,
    ) -> Result<CellMap<ExclusionDiff<O>>, Error>
    where
        I: IntoIterator,
        I::Item: Borrow<CellIndex>,
        OPT: ShortestPathOptions + Send + Sync,
        O: Send + Ord + Clone,
        PM: Fn(Path<W>) -> Result<O, Error> + Send + Sync;
}

impl<G, W> DifferentialShortestPath<W> for G
where
    W: PartialOrd + PartialEq + Add + Copy + Send + Ord + Zero + Sync,
    G: GetCellEdges<EdgeWeightType = W>
        + GetCellNode
        + HasH3Resolution
        + NearestGraphNodes
        + Sync
        + ShortestPathManyToMany<W>,
{
    fn differential_shortest_path_map<I, OPT, PM, O>(
        &self,
        origin_cells: I,
        destination_cells: I,
        exclude_cells: &H3Treemap<CellIndex>,
        options: &OPT,
        path_transform_fn: PM,
    ) -> Result<CellMap<ExclusionDiff<O>>, Error>
    where
        I: IntoIterator,
        I::Item: Borrow<CellIndex>,
        OPT: ShortestPathOptions + Send + Sync,
        O: Send + Ord + Clone,
        PM: Fn(Path<W>) -> Result<O, Error> + Send + Sync,
    {
        if exclude_cells.is_empty() {
            return Err(Error::EmptyExcludeCells);
        };
        let origin_cells = check_resolution_and_collect(
            origin_cells.into_iter().filter(|c| {
                // exclude the cells of the disturbance itself from routing
                !exclude_cells.contains(c.borrow())
            }),
            self.h3_resolution(),
        )?;
        let destination_cells =
            check_resolution_and_collect(destination_cells, self.h3_resolution())?;

        let mut paths_before = self.shortest_path_many_to_many_map(
            &origin_cells,
            &destination_cells,
            options,
            &path_transform_fn,
        )?;

        let exclude_wrapper = ExcludeCells::new(self, exclude_cells);
        let mut paths_after = exclude_wrapper.shortest_path_many_to_many_map(
            &origin_cells,
            &destination_cells,
            options,
            path_transform_fn,
        )?;

        let mut out_diffs =
            CellMap::with_capacity_and_hasher(paths_before.len(), RandomState::default());
        for (cell, paths) in paths_before.drain() {
            out_diffs.insert(
                cell,
                ExclusionDiff {
                    before_cell_exclusion: paths,
                    after_cell_exclusion: paths_after.remove(&cell).unwrap_or_default(),
                },
            );
        }
        Ok(out_diffs)
    }
}

fn check_resolution_and_collect<I>(
    in_cells: I,
    h3_resolution: Resolution,
) -> Result<Vec<CellIndex>, Error>
where
    I: IntoIterator,
    I::Item: Borrow<CellIndex>,
{
    let mut out_cells = in_cells
        .into_iter()
        .map(|cell| {
            let r = cell.borrow().resolution();
            if r != h3_resolution {
                Err(Error::MixedH3Resolutions(h3_resolution, r))
            } else {
                Ok(*cell.borrow())
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    out_cells.sort_unstable();
    out_cells.dedup();
    Ok(out_cells)
}
