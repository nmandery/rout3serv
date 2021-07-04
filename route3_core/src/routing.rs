use std::borrow::Borrow;
use std::collections::HashSet;
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::io::empty;
use std::iter::Filter;
use std::ops::Add;

use num_traits::Zero;
use pathfinding::directed::dijkstra::dijkstra_partial;
use rayon::prelude::*;

use crate::algo::iter::{change_h3_resolution, ChangeH3ResolutionIterator};
use crate::error::Error;
use crate::graph::{downsample_graph, H3Graph, NodeType};
use crate::h3ron::{H3Cell, Index};
use crate::{H3CellMap, H3CellSet, WithH3Resolution};

pub struct SearchSpace {
    /// h3 resolution used for the cells of the search space
    h3_resolution: u8,

    cells: H3CellSet,
}

/// search space to constrain the area dijkstra is working on
impl SearchSpace {
    /// should only be called with cells with a resolution >= self.h3_resolution
    pub fn contains(&self, cell: &H3Cell) -> bool {
        self.cells
            .contains(&cell.get_parent_unchecked(self.h3_resolution))
    }
}

#[derive(Clone, Debug)]
pub struct ManyToManyOptions {
    /// number of destinations to reach.
    /// Routing for the origin cell will stop when this number of targets are reached. When not set,
    /// routing will continue until all destinations are reached
    num_destinations_to_reach: Option<usize>,
    //// search space to limit the nodes to be visited during routing
    //search_space: Option<SearchSpace>,
}

impl Default for ManyToManyOptions {
    fn default() -> Self {
        Self {
            num_destinations_to_reach: None,
            //search_space: None,
        }
    }
}

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

///
///
/// All routing methods will silently ignore origin and destination cells which are not
/// part of the graph.
impl<T> RoutingGraph<T>
where
    T: PartialOrd + PartialEq + Add + Copy + Send + Ord + Zero + Sync,
{
    pub fn route_many_to_many<I>(
        &self,
        origin_cells: I,
        destination_cells: I,
        options: ManyToManyOptions,
        search_space: Option<SearchSpace>,
    ) -> Result<Vec<Route<T>>, Error>
    where
        I: IntoIterator,
        I::Item: Borrow<H3Cell>,
    {
        let filtered_origin_cells = {
            let mut o_cells = change_h3_resolution(origin_cells, self.h3_resolution())
                .filter(|cell| {
                    self.graph_nodes
                        .get(&cell)
                        .map(|node_type| node_type.is_origin())
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>();
            o_cells.sort_unstable();
            o_cells.dedup();
            o_cells
        };

        if filtered_origin_cells.is_empty() {
            return Ok(vec![]);
        }

        let filtered_destination_cells =
            change_h3_resolution(destination_cells, self.h3_resolution())
                .filter(|cell| {
                    self.graph_nodes
                        .get(&cell)
                        .map(|node_type| node_type.is_destination())
                        .unwrap_or(false)
                })
                .collect::<H3CellSet>();

        if filtered_destination_cells.is_empty() {
            return Err(Error::DestinationsNotInGraph);
        }

        log::debug!(
            "routing many-to-many: from {} cells to {} cells at resolution {}",
            filtered_origin_cells.len(),
            filtered_destination_cells.len(),
            self.h3_resolution()
        );
        let routes = filtered_origin_cells
            .par_iter()
            .map(|origin_cell| {
                let mut destination_cells_reached = H3CellSet::new();

                // TODO: add timeout to not continue routing forever
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
                                    if search_space
                                        .as_ref()
                                        .map(|s_space| s_space.contains(&destination_cell))
                                        .unwrap_or(true)
                                    {
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
                    |cell| {
                        if filtered_destination_cells.contains(cell) {
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
                let mut routes = vec![];
                for dest in destination_cells_reached.iter() {
                    let mut last_cell = *dest;
                    let mut route_cells = vec![*dest];
                    let mut cost = T::zero();
                    while let Some((cell, weight)) = routemap.get(&last_cell) {
                        route_cells.push(*cell);
                        last_cell = *cell;
                        cost = cost + *weight;
                    }
                    route_cells.reverse();
                    routes.push(Route {
                        cells: route_cells,
                        cost,
                    })
                }
                routes
            })
            .flatten()
            .collect::<Vec<_>>();
        Ok(routes)
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

pub struct RoutingContext<T>
where
    T: PartialOrd + PartialEq + Add + Copy + Send + Ord + Zero,
{
    routing_graph: RoutingGraph<T>,
    downsampled_routing_graph: RoutingGraph<T>,
}

impl<T> RoutingContext<T> where T: PartialOrd + PartialEq + Add + Copy + Send + Ord + Zero {}

impl<T> WithH3Resolution for RoutingContext<T>
where
    T: PartialOrd + PartialEq + Add + Copy + Send + Ord + Zero,
{
    fn h3_resolution(&self) -> u8 {
        self.routing_graph.h3_resolution()
    }
}

impl<T> TryFrom<H3Graph<T>> for RoutingContext<T>
where
    T: PartialOrd + PartialEq + Add + Copy + Send + Ord + Zero,
{
    type Error = Error;

    fn try_from(graph: H3Graph<T>) -> std::result::Result<Self, Self::Error> {
        let routing_graph: RoutingGraph<T> = graph.try_into()?;
        let downsampled_routing_graph = downsample_graph(
            routing_graph.graph.clone(),
            routing_graph.graph.h3_resolution.saturating_sub(3),
        )?
        .try_into()?;

        Ok(Self {
            routing_graph,
            downsampled_routing_graph,
        })
    }
}
