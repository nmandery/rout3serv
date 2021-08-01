use std::borrow::Borrow;
use std::cmp::Ordering;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::ops::Add;

use geo_types::{LineString, Point};
use num_traits::Zero;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::algo::dijkstra::{build_path_with_cost, dijkstra_partial};
use crate::collections::{H3CellMap, H3CellSet, HashMap};
use crate::error::Error;
use crate::geo_types::Geometry;
use crate::graph::{H3Graph, NodeType};
use crate::h3ron::{H3Cell, H3Edge, Index, ToCoordinate};
use crate::iter::change_h3_resolution;
use crate::WithH3Resolution;

#[derive(Clone, Debug)]
pub struct ManyToManyOptions {
    /// number of destinations to reach.
    /// Routing for the origin cell will stop when this number of targets are reached. When not set,
    /// routing will continue until all destinations are reached
    pub num_destinations_to_reach: Option<usize>,

    /// cells which are not allowed to be used for routing
    pub exclude_cells: Option<H3CellSet>,

    /// Number of cells to be allowed to be missing between
    /// a cell and the graph while the cell is still counted as being connected
    /// to the graph
    pub num_gap_cells_to_graph: u32,
}

impl Default for ManyToManyOptions {
    fn default() -> Self {
        Self {
            num_destinations_to_reach: None,
            exclude_cells: None,
            num_gap_cells_to_graph: 0,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Route<T> {
    /// cells of the route in the order origin -> destination
    pub cells: Vec<H3Cell>,

    /// the total cost of the route.
    /// Sum of all edge weights
    pub cost: T,
}

impl<T> Route<T> {
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
    pub fn len(&self) -> usize {
        self.cells.len()
    }
    pub fn origin_cell(&self) -> Result<H3Cell, Error> {
        self.cells.first().cloned().ok_or(Error::EmptyRoute)
    }
    pub fn destination_cell(&self) -> Result<H3Cell, Error> {
        self.cells.last().cloned().ok_or(Error::EmptyRoute)
    }
    pub fn geometry(&self) -> Geometry<f64> {
        match self.cells.len() {
            0 => unreachable!(),
            1 => Point::from(self.cells[0].to_coordinate()).into(),
            _ => LineString::from(
                self.cells
                    .iter()
                    .map(|cell| cell.to_coordinate())
                    .collect::<Vec<_>>(),
            )
            .into(),
        }
    }

    pub fn to_h3_edges(&self) -> Result<Vec<H3Edge>, Error> {
        self.cells
            .windows(2)
            .map(|wdow| wdow[0].unidirectional_edge_to(&wdow[1]))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }
}

/// order by cost, origin index and destination_index.
///
/// This ordering can used to bring `Vec`s of routes in a deterministic order to make them
/// comparable
impl<T> Ord for Route<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        let cmp_cost = self.cost.cmp(&other.cost);
        if cmp_cost == Ordering::Equal {
            let cmp_origin =
                index_or_zero(self.origin_cell()).cmp(&index_or_zero(other.origin_cell()));
            if cmp_origin == Ordering::Equal {
                index_or_zero(self.destination_cell()).cmp(&index_or_zero(other.destination_cell()))
            } else {
                cmp_origin
            }
        } else {
            cmp_cost
        }
    }
}

impl<T> PartialOrd for Route<T>
where
    T: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[inline]
fn index_or_zero(cell: Result<H3Cell, Error>) -> u64 {
    cell.map(|c| c.h3index()).unwrap_or(0)
}

pub struct RoutingGraph<T> {
    pub graph: H3Graph<T>,
    graph_nodes: H3CellMap<NodeType>,
}

impl<T> WithH3Resolution for RoutingGraph<T> {
    fn h3_resolution(&self) -> u8 {
        self.graph.h3_resolution()
    }
}

enum CellGraphMembership {
    /// the cell is connected to the graph
    DirectConnection(H3Cell),

    /// cell `self.0` is not connected to the graph, but the next best neighbor `self.1` is
    WithGap(H3Cell, H3Cell),

    /// cell is outside of the graph
    NoConnection(H3Cell),
}

impl CellGraphMembership {
    pub fn cell(&self) -> H3Cell {
        match self {
            Self::DirectConnection(cell) => *cell,
            Self::WithGap(cell, _) => *cell,
            Self::NoConnection(cell) => *cell,
        }
    }

    pub fn corresponding_cell_in_graph(&self) -> Option<H3Cell> {
        match self {
            Self::DirectConnection(cell) => Some(*cell),
            Self::WithGap(_, cell) => Some(*cell),
            _ => None,
        }
    }
}

///
///
/// All routing methods will silently ignore origin and destination cells which are not
/// part of the graph.
impl<T> RoutingGraph<T>
where
    T: PartialOrd + PartialEq + Add + Copy + Send + Ord + Zero + Sync + Debug,
{
    ///
    /// Returns found routes keyed by the origin cell.
    ///
    /// All cells must be in the h3 resolution of the graph.
    pub fn route_many_to_many<I>(
        &self,
        origin_cells: I,
        destination_cells: I,
        options: &ManyToManyOptions,
    ) -> Result<H3CellMap<Vec<Route<T>>>, Error>
    where
        I: IntoIterator,
        I::Item: Borrow<H3Cell>,
    {
        let filtered_origin_cells: Vec<_> = {
            // maps cells to their closest found neighbors in the graph
            let mut origin_cell_map = H3CellMap::default();
            for gm in self
                .filtered_graph_membership::<Vec<_>, _>(
                    change_h3_resolution(origin_cells, self.h3_resolution()).collect(),
                    |node_type| node_type.is_origin(),
                    options.num_gap_cells_to_graph,
                )
                .drain(..)
            {
                if let Some(corr_cell) = gm.corresponding_cell_in_graph() {
                    origin_cell_map
                        .entry(corr_cell)
                        .and_modify(|ccs: &mut Vec<H3Cell>| ccs.push(gm.cell()))
                        .or_insert_with(|| vec![gm.cell()]);
                }
            }
            origin_cell_map.drain().collect()
        };

        if filtered_origin_cells.is_empty() {
            return Ok(Default::default());
        }

        // maps directly to the graph connected cells to the cells outside the
        // graph where they are used as a substitute. For direct graph members
        // both cells are the same
        // TODO: this shoud be a 1:n relationship in case multiple cells map to
        //      the same cell in the graph
        let filtered_destination_cells: HashMap<_, _> = self
            .filtered_graph_membership::<Vec<_>, _>(
                change_h3_resolution(destination_cells, self.h3_resolution()).collect(),
                |node_type| node_type.is_destination(),
                options.num_gap_cells_to_graph,
            )
            .drain(..)
            .filter_map(|connected_cell| {
                // ignore all non-connected destinations
                connected_cell
                    .corresponding_cell_in_graph()
                    .map(|cor_cell| (cor_cell, connected_cell.cell()))
            })
            .collect();

        if filtered_destination_cells.is_empty() {
            return Err(Error::DestinationsNotInGraph);
        }

        let is_excluded = |cell: H3Cell| {
            options
                .exclude_cells
                .as_ref()
                .map(|exclude| exclude.contains(&cell))
                .unwrap_or(false)
        };

        log::debug!(
            "routing many-to-many: from {} cells to {} cells at resolution {} with num_gap_cells_to_graph = {}",
            filtered_origin_cells.len(),
            filtered_destination_cells.len(),
            self.h3_resolution(),
            options.num_gap_cells_to_graph
        );
        let routes = filtered_origin_cells
            .par_iter()
            .map(|(origin_cell, output_origin_cells)| {
                let mut destination_cells_reached = H3CellSet::default();

                // Possible improvement: add timeout to avoid continuing routing forever
                let (routemap, _) = dijkstra_partial(
                    // start cell
                    origin_cell,
                    // successor cells
                    |cell| {
                        let neighbors = cell
                            .unidirectional_edges()
                            .iter()
                            .filter_map(|edge| {
                                if let Some((edge, weight)) = self.graph.edges.get_key_value(edge) {
                                    let destination_cell = edge.destination_index_unchecked();
                                    if !is_excluded(destination_cell) {
                                        Some((destination_cell, *weight))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();
                        neighbors
                    },
                    // stop condition
                    |graph_cell| {
                        if let Some(cell) = filtered_destination_cells.get(graph_cell) {
                            destination_cells_reached.insert(*cell);

                            // stop when enough destination cells are reached
                            destination_cells_reached.len()
                                >= options
                                    .num_destinations_to_reach
                                    .unwrap_or_else(|| filtered_destination_cells.len())
                        } else {
                            false
                        }
                    },
                );

                // build the routes
                let mut routes = Vec::with_capacity(destination_cells_reached.len());
                for dest in destination_cells_reached.iter() {
                    let (route_cells, cost) = build_path_with_cost(dest, &routemap);
                    routes.push(Route {
                        cells: route_cells,
                        cost,
                    })
                }
                // return sorted from lowest to highest cost, use destination cell as second criteria
                // to make route vecs directly comparable using this deterministic order
                routes.sort_unstable();

                output_origin_cells
                    .iter()
                    .map(|out_cell| (*out_cell, routes.clone()))
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<H3CellMap<_>>();
        Ok(routes)
    }

    fn filtered_graph_membership<B, F>(
        &self,
        mut cells: Vec<H3Cell>,
        nodetype_filter_fn: F,
        num_gap_cells_to_graph: u32,
    ) -> B
    where
        B: FromParallelIterator<CellGraphMembership>,
        F: Fn(&NodeType) -> bool + Send + Sync + Copy,
    {
        // TODO: This function should probably take an iterator instead of a vec
        cells.sort_unstable();
        cells.dedup();
        cells
            .par_iter()
            .map(|cell: &H3Cell| {
                if self
                    .graph_nodes
                    .get(cell)
                    .map(nodetype_filter_fn)
                    .unwrap_or(false)
                {
                    CellGraphMembership::DirectConnection(*cell)
                } else if num_gap_cells_to_graph > 0 {
                    // attempt to find the next neighboring cell which is part of the graph
                    let mut neighbors = cell.k_ring_distances(1, num_gap_cells_to_graph.max(1));
                    neighbors.sort_unstable_by_key(|neighbor| neighbor.0);

                    // possible improvement: choose the neighbor with the best connectivity or
                    // the edge with the smallest weight
                    let mut selected_neighbor: Option<H3Cell> = None;
                    for neighbor in neighbors {
                        if self
                            .graph_nodes
                            .get(&neighbor.1)
                            .map(nodetype_filter_fn)
                            .unwrap_or(false)
                        {
                            selected_neighbor = Some(neighbor.1);
                            break;
                        }
                    }
                    selected_neighbor
                        .map(|neighbor| CellGraphMembership::WithGap(*cell, neighbor))
                        .unwrap_or_else(|| CellGraphMembership::NoConnection(*cell))
                } else {
                    CellGraphMembership::NoConnection(*cell)
                }
            })
            .collect()
    }
}

impl<T> TryFrom<H3Graph<T>> for RoutingGraph<T>
where
    T: PartialOrd + PartialEq + Add + Copy + Send + Ord + Zero,
{
    type Error = Error;

    fn try_from(graph: H3Graph<T>) -> std::result::Result<Self, Self::Error> {
        let graph_nodes = graph.nodes()?;
        Ok(Self { graph, graph_nodes })
    }
}

#[cfg(test)]
mod tests {
    use h3ron::H3Cell;

    use crate::h3ron::Index;
    use crate::routing::Route;

    #[test]
    fn route_deterministic_ordering() {
        let r1 = Route {
            cells: vec![H3Cell::new(0), H3Cell::new(5)],
            cost: 1,
        };
        let r2 = Route {
            cells: vec![H3Cell::new(1), H3Cell::new(2)],
            cost: 3,
        };
        let r3 = Route {
            cells: vec![H3Cell::new(1), H3Cell::new(3)],
            cost: 3,
        };
        let mut routes = vec![r3.clone(), r1.clone(), r2.clone()];
        routes.sort_unstable();
        assert_eq!(routes[0], r1);
        assert_eq!(routes[1], r2);
        assert_eq!(routes[2], r3);
    }
}
