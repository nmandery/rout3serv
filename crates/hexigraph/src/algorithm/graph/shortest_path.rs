//! Dijkstra shortest-path routing.
//!
use h3o::CellIndex;
use hashbrown::hash_map::Entry;
use std::borrow::Borrow;
use std::ops::Add;

use crate::algorithm::graph::dijkstra::edge_dijkstra;
use crate::algorithm::graph::path::Path;
use crate::algorithm::graph::NearestGraphNodes;
use crate::algorithm::resolution::transform_resolution;
use crate::container::treemap::H3Treemap;
use crate::container::CellMap;
use num_traits::Zero;
use rayon::prelude::*;
use tracing::debug;

use crate::error::Error;
use crate::graph::{GetCellEdges, GetCellNode};
use crate::HasH3Resolution;

///
/// Generic type parameters:
/// * `W`: The weight used in the graph.
pub trait ShortestPathOptions {
    /// Number of cells to be allowed to be missing between
    /// a cell and the graph while the cell is still counted as being connected
    /// to the graph.
    ///
    /// Implemented using see [`NearestGraphNodes`].
    fn max_distance_to_graph(&self) -> u32 {
        0
    }

    /// number of destinations to reach.
    /// Routing for the origin cell will stop when this number of destinations are reached. When not set,
    /// routing will continue until all destinations are reached
    fn num_destinations_to_reach(&self) -> Option<usize> {
        None
    }
}

/// Default implementation of a type implementing the `ShortestPathOptions`
/// trait.
#[derive(Default)]
pub struct DefaultShortestPathOptions {}

impl ShortestPathOptions for DefaultShortestPathOptions {}

impl DefaultShortestPathOptions {
    pub fn new() -> Self {
        Default::default()
    }
}

/// Implements a simple Dijkstra shortest path route finding.
///
/// While this is not the most efficient routing algorithm, it has the
/// benefit of finding the nearest destinations first. So it can be used
/// to answer questions like "which are the N nearest destinations" using a
/// large amount of possible destinations.
pub trait ShortestPath<W> {
    fn shortest_path<I, OPT: ShortestPathOptions>(
        &self,
        origin_cell: CellIndex,
        destination_cells: I,
        options: &OPT,
    ) -> Result<Vec<Path<W>>, Error>
    where
        I: IntoIterator,
        I::Item: Borrow<CellIndex>;
}

/// Variant of the [`ShortestPath`] trait routing from multiple
/// origins in parallel.
pub trait ShortestPathManyToMany<W>
where
    W: Send + Sync + Ord + Copy,
{
    /// Returns found paths keyed by the origin cell.
    ///
    /// All cells must be in the h3 resolution of the graph.
    #[inline]
    fn shortest_path_many_to_many<I, OPT>(
        &self,
        origin_cells: I,
        destination_cells: I,
        options: &OPT,
    ) -> Result<CellMap<Vec<Path<W>>>, Error>
    where
        I: IntoIterator,
        I::Item: Borrow<CellIndex>,
        OPT: ShortestPathOptions + Send + Sync,
    {
        self.shortest_path_many_to_many_map(origin_cells, destination_cells, options, Ok)
    }

    /// Returns found paths, transformed by the `path_map_fn` and keyed by the
    /// origin cell.
    ///
    /// `path_transform_fn` can be used to directly convert the paths to a less memory intensive
    /// type.
    ///
    /// All cells must be in the h3 resolution of the graph.
    fn shortest_path_many_to_many_map<I, OPT, PM, O>(
        &self,
        origin_cells: I,
        destination_cells: I,
        options: &OPT,
        path_transform_fn: PM,
    ) -> Result<CellMap<Vec<O>>, Error>
    where
        I: IntoIterator,
        I::Item: Borrow<CellIndex>,
        OPT: ShortestPathOptions + Send + Sync,
        PM: Fn(Path<W>) -> Result<O, Error> + Send + Sync,
        O: Send + Ord + Clone;
}

impl<W, G> ShortestPathManyToMany<W> for G
where
    G: GetCellEdges<EdgeWeightType = W> + GetCellNode + HasH3Resolution + NearestGraphNodes + Sync,
    W: PartialOrd + PartialEq + Add + Copy + Send + Ord + Zero + Sync,
{
    fn shortest_path_many_to_many_map<I, OPT, PM, O>(
        &self,
        origin_cells: I,
        destination_cells: I,
        options: &OPT,
        path_transform_fn: PM,
    ) -> Result<CellMap<Vec<O>>, Error>
    where
        I: IntoIterator,
        I::Item: Borrow<CellIndex>,
        OPT: ShortestPathOptions + Send + Sync,
        PM: Fn(Path<W>) -> Result<O, Error> + Send + Sync,
        O: Send + Ord + Clone,
    {
        let filtered_origin_cells = substitute_origin_cells(
            self,
            options.max_distance_to_graph(),
            origin_cells,
            true, // speeds up the creation of the treemap from the origins further below
        )?;
        if filtered_origin_cells.is_empty() {
            return Ok(Default::default());
        }

        let destination_substmap = {
            let origins_treemap: H3Treemap<CellIndex> =
                filtered_origin_cells.iter().map(|(k, _)| *k).collect();

            substitute_destination_cells(
                self,
                options.max_distance_to_graph(),
                destination_cells,
                &origins_treemap,
            )?
        };

        if destination_substmap.0.is_empty() {
            return Ok(Default::default());
        }

        let destination_treemap = H3Treemap::from_iter(destination_substmap.0.keys().copied());

        debug!(
            "shortest_path many-to-many: from {} cells to {} cells at resolution {} with max_distance_to_graph = {}",
            filtered_origin_cells.len(),
            destination_substmap.0.len(),
            self.h3_resolution(),
            options.max_distance_to_graph()
        );

        let mut cellmap: CellMap<Vec<O>> = Default::default();
        for par_result in filtered_origin_cells
            .par_iter()
            .map(|(graph_connected_origin_cell, output_origin_cells)| {
                shortest_path_many_worker(
                    self,
                    *graph_connected_origin_cell,
                    output_origin_cells.as_slice(),
                    &destination_treemap,
                    &destination_substmap,
                    options,
                    |path| {
                        let origin_cell = path.origin_cell;
                        path_transform_fn(path).map(|transformed| (origin_cell, transformed))
                    },
                )
            })
            .collect::<Result<Vec<_>, _>>()?
        {
            for (origin_cell, transformed) in par_result {
                match cellmap.entry(origin_cell) {
                    Entry::Occupied(mut entry) => entry.get_mut().push(transformed),
                    Entry::Vacant(entry) => {
                        entry.insert(vec![transformed]);
                    }
                }
            }
        }
        Ok(cellmap)
    }
}

impl<W, G> ShortestPath<W> for G
where
    G: GetCellEdges<EdgeWeightType = W> + GetCellNode + HasH3Resolution + NearestGraphNodes,
    W: PartialOrd + PartialEq + Add + Copy + Ord + Zero,
{
    fn shortest_path<I, OPT>(
        &self,
        origin_cell: CellIndex,
        destination_cells: I,
        options: &OPT,
    ) -> Result<Vec<Path<W>>, Error>
    where
        I: IntoIterator,
        I::Item: Borrow<CellIndex>,
        OPT: ShortestPathOptions,
    {
        let (graph_connected_origin_cell, requested_origin_cells) = {
            let mut filtered_origin_cells = substitute_origin_cells(
                self,
                options.max_distance_to_graph(),
                std::iter::once(origin_cell),
                false, // not necessary
            )?;
            if filtered_origin_cells.is_empty() {
                return Ok(Default::default());
            } else {
                filtered_origin_cells.remove(0)
            }
        };

        let destination_substmap = {
            let mut origins_treemap: H3Treemap<CellIndex> = Default::default();
            origins_treemap.insert(graph_connected_origin_cell);
            substitute_destination_cells(
                self,
                options.max_distance_to_graph(),
                destination_cells,
                &origins_treemap,
            )?
        };

        if destination_substmap.0.is_empty() {
            return Ok(Default::default());
        }

        let destination_treemap = H3Treemap::from_iter(destination_substmap.0.keys().copied());

        shortest_path_many_worker(
            self,
            graph_connected_origin_cell,
            requested_origin_cells.as_slice(),
            &destination_treemap,
            &destination_substmap,
            options,
            Ok,
        )
    }
}

fn shortest_path_many_worker<G, W, OPT, PM, O>(
    graph: &G,
    origin_cell: CellIndex,
    requested_origin_cells: &[CellIndex],
    destination_cells: &H3Treemap<CellIndex>,
    destination_substmap: &SubstituteMap,
    options: &OPT,
    path_transform_fn: PM,
) -> Result<Vec<O>, Error>
where
    G: GetCellEdges<EdgeWeightType = W>,
    W: Add + Copy + Ord + Zero,
    PM: Fn(Path<W>) -> Result<O, Error>,
    O: Clone,
    OPT: ShortestPathOptions,
{
    let found_paths = edge_dijkstra(
        graph,
        origin_cell,
        destination_cells,
        options.num_destinations_to_reach(),
    )?;

    let mut transformed_paths = Vec::with_capacity(found_paths.len());

    for path in found_paths.into_iter() {
        for destination_cell in destination_substmap.cells_substituted_by(path.destination_cell) {
            for origin_cell in requested_origin_cells {
                let mut this_path = path.clone();
                this_path.origin_cell = *origin_cell;
                this_path.destination_cell = *destination_cell;

                transformed_paths.push(path_transform_fn(this_path)?);
            }
        }
    }
    Ok(transformed_paths)
}

/// Maps Cells which are part of the graph - the keys - to requested
/// cells values.
#[derive(Default)]
struct SubstituteMap(CellMap<Vec<CellIndex>>);

impl SubstituteMap {
    /// get a slices of cells the given `cell` substitutes
    fn cells_substituted_by(&self, cell: CellIndex) -> &[CellIndex] {
        self.0
            .get(&cell)
            .map_or_else(|| &[] as &[CellIndex], |sub| sub.as_slice())
    }

    /// `substituted_by` is the cell connected to the graph
    fn add_substitute(&mut self, cell: CellIndex, substituted_by: CellIndex) {
        match self.0.entry(substituted_by) {
            Entry::Occupied(mut occupied) => occupied.get_mut().push(cell),
            Entry::Vacant(vacant) => {
                vacant.insert(vec![cell]);
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// finds the corresponding cells in the graph for the given
/// destinations. When no corresponding cell is found, that destination
/// is filtered out.
///
/// The cell resolution is changed to the resolution of the graph.
///
/// There must be at least one destination to get Result::Ok, otherwise
/// the complete graph would be traversed.
fn substitute_destination_cells<G, I>(
    graph: &G,
    max_distance_to_graph: u32,
    destination_cells: I,
    origins_treemap: &H3Treemap<CellIndex>,
) -> Result<SubstituteMap, Error>
where
    G: GetCellNode + NearestGraphNodes + HasH3Resolution,
    I: IntoIterator,
    I::Item: Borrow<CellIndex>,
{
    // maps cells which are members of the graph to their closest found neighbors which are contained in the
    // destinations_cells
    let mut destination_substmap = SubstituteMap::default();

    for destination in transform_resolution(destination_cells, graph.h3_resolution()) {
        for (graph_cell, node_type, _) in
            graph.nearest_graph_nodes(destination, max_distance_to_graph)?
        {
            // destinations which are origins at the same time are always allowed as they can
            // always be reached even when they are not a destination in the graph.
            if node_type.is_destination() || origins_treemap.contains(&graph_cell) {
                destination_substmap.add_substitute(destination, graph_cell);
                break;
            }
        }
    }

    if destination_substmap.is_empty() {
        return Err(Error::DestinationsNotInGraph);
    }
    Ok(destination_substmap)
}

/// Locates the corresponding cells for the given ones in the graph.
///
/// The returned hashmap maps cells, which are members of the graph to all
/// surrounding cells which are not directly part of the graph. This depends
/// on the gap-bridging in the options. With no gap bridging, cells are only mapped
/// to themselves.
///
/// The cell resolution is changed to the resolution of the graph.
fn substitute_origin_cells<G, I>(
    graph: &G,
    max_distance_to_graph: u32,
    origin_cells: I,
    return_sorted: bool,
) -> Result<Vec<(CellIndex, Vec<CellIndex>)>, Error>
where
    G: GetCellNode + NearestGraphNodes + HasH3Resolution,
    I: IntoIterator,
    I::Item: Borrow<CellIndex>,
{
    // maps cells which are members of the graph to their closest found neighbors which are contained in the
    // origin_cells.
    let mut origin_substmap = SubstituteMap::default();

    for cell in transform_resolution(origin_cells, graph.h3_resolution()) {
        for (graph_cell, node_type, _) in graph.nearest_graph_nodes(cell, max_distance_to_graph)? {
            if node_type.is_origin() {
                origin_substmap.add_substitute(cell, graph_cell);
                break;
            }
        }
    }

    let mut out_vec: Vec<_> = origin_substmap.0.into_iter().collect();
    if return_sorted {
        out_vec.sort_unstable_by_key(|v| v.0);
    }
    Ok(out_vec)
}

#[cfg(test)]
mod tests {
    use h3o::{LatLng, Resolution};
    use std::convert::TryInto;

    use crate::algorithm::graph::shortest_path::DefaultShortestPathOptions;
    use crate::algorithm::graph::ShortestPathManyToMany;
    use crate::graph::{H3EdgeGraph, PreparedH3EdgeGraph};
    use geo::Coord;

    #[test]
    fn test_shortest_path_same_origin_and_destination() {
        let res = Resolution::Eight;
        let origin = {
            let ll: LatLng = Coord::from((23.3, 12.3)).try_into().unwrap();
            ll.to_cell(res)
        };
        let edge = origin.edges().next().unwrap();
        let destination = edge.destination();

        // build a micro-graph
        let prepared_graph: PreparedH3EdgeGraph<_> = {
            let mut graph = H3EdgeGraph::new(res);
            graph.add_edge(edge, 5_u32);
            graph.try_into().unwrap()
        };

        let paths = prepared_graph
            .shortest_path_many_to_many(
                &vec![origin],
                // find the path to the origin cell itself, and to the neighbor
                &vec![origin, destination],
                &DefaultShortestPathOptions::default(),
            )
            .unwrap();

        assert_eq!(paths.len(), 1);
        let path_vec = paths.get(&origin).unwrap();
        assert_eq!(path_vec.len(), 2);
        for path in path_vec.iter() {
            if path.destination_cell == origin {
                assert!(path.is_empty());
                assert_eq!(path.cost, 0);
            } else if path.destination_cell == destination {
                assert!(!path.is_empty());
                assert_eq!(path.cost, 5);
            } else {
                unreachable!()
            }
        }
    }
}
