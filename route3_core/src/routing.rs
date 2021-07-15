use std::borrow::Borrow;
use std::cmp::Ordering;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::ops::Add;

use crate::algo::dijkstra::{build_path_with_cost, dijkstra_partial};
use geo_types::{LineString, Point};
use num_traits::Zero;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::collections::{H3CellMap, H3CellSet};
use crate::error::Error;
use crate::geo_types::Geometry;
use crate::graph::{H3Graph, NodeType};
use crate::h3ron::{H3Cell, H3Edge, ToCoordinate};
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
}

impl Default for ManyToManyOptions {
    fn default() -> Self {
        Self {
            num_destinations_to_reach: None,
            exclude_cells: None,
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
    /// Returns found routes keyed by the origin cell.
    ///
    /// `search_space` limits the routing to child nodes contained in the search space.
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
            return Ok(Default::default());
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

        let is_excluded = |cell: H3Cell| {
            options
                .exclude_cells
                .as_ref()
                .map(|exclude| exclude.contains(&cell))
                .unwrap_or(false)
        };

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
                    let (route_cells, cost) = build_path_with_cost(dest, &routemap);
                    routes.push(Route {
                        cells: route_cells,
                        cost,
                    })
                }
                (*origin_cell, routes)
            })
            .collect::<H3CellMap<_>>();
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
