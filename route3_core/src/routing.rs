use std::borrow::Borrow;
use std::cmp::Ordering;
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::ops::Add;

use geo_types::LineString;
use num_traits::Zero;
use pathfinding::directed::dijkstra::dijkstra_partial;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::graph::{downsample_graph, H3Graph, NodeType};
use crate::h3ron::{H3Cell, H3Edge, Index, ToCoordinate};
use crate::iter::change_h3_resolution;
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

    pub fn len(&self) -> usize {
        self.cells.len()
    }
}

impl WithH3Resolution for SearchSpace {
    fn h3_resolution(&self) -> u8 {
        self.h3_resolution
    }
}

#[derive(Clone, Debug)]
pub struct ManyToManyOptions {
    /// number of destinations to reach.
    /// Routing for the origin cell will stop when this number of targets are reached. When not set,
    /// routing will continue until all destinations are reached
    pub num_destinations_to_reach: Option<usize>,
}

impl Default for ManyToManyOptions {
    fn default() -> Self {
        Self {
            num_destinations_to_reach: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Route<T> {
    /// cells of the route in the order origin -> destination
    pub cells: Vec<H3Cell>,

    /// the total cost of the route.
    /// Sum of all edge weights
    pub cost: T,
}

impl<T> Route<T>
where
    T: Debug,
{
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
    pub fn to_linestring(&self) -> LineString<f64> {
        LineString::from(
            self.cells
                .iter()
                .map(|cell| cell.to_coordinate())
                .collect::<Vec<_>>(),
        )
    }

    pub fn to_h3_edges(&self) -> Result<Vec<H3Edge>, Error> {
        self.cells
            .windows(2)
            .map(|wdow| wdow[0].unidirectional_edge_to(&wdow[1]))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }
}

/// order simply by cost
impl<T> PartialOrd for Route<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.cost.partial_cmp(&other.cost)
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
    ///
    /// `search_space` limits the routing to child nodes contained in the search space.
    pub fn route_many_to_many<I>(
        &self,
        origin_cells: I,
        destination_cells: I,
        options: &ManyToManyOptions,
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

impl<T> RoutingContext<T>
where
    T: PartialOrd + PartialEq + Add + Copy + Send + Ord + Zero + Sync,
{
    /// create a constrained search space first to limit the spread of the dijkstra routing
    /// by prerouting on a lower resolutions (-> less nodes to visit)
    /// https://i11www.iti.kit.edu/_media/teaching/theses/files/studienarbeit-schuetz-05.pdf
    fn searchspace_from_routes<I>(&self, routes: I, h3_resolution: u8) -> SearchSpace
    where
        I: Iterator,
        I::Item: Borrow<Route<T>>,
    {
        let mut cells = H3CellSet::new();
        for route in routes {
            cells.extend(change_h3_resolution(&route.borrow().cells, h3_resolution));
        }
        log::debug!(
            "search space uses {} cells at r={}",
            cells.len(),
            h3_resolution
        );
        SearchSpace {
            h3_resolution,
            cells,
        }
    }

    pub fn route_many_to_many(
        &self,
        origin_cells: &[H3Cell],
        destination_cells: &[H3Cell],
        options: &ManyToManyOptions,
    ) -> Result<Vec<Route<T>>, Error> {
        let search_space = self.searchspace_from_routes(
            self.downsampled_routing_graph
                .route_many_to_many(origin_cells, destination_cells, options, None)?
                .into_iter(),
            self.downsampled_routing_graph.h3_resolution(),
        );
        self.routing_graph.route_many_to_many(
            origin_cells,
            destination_cells,
            options,
            Some(search_space),
        )
    }
}

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
