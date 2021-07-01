use std::collections::HashSet;
use std::convert::TryFrom;
use std::ops::Add;

use crate::error::Error;
use crate::graph::H3Graph;
use crate::h3ron::H3Cell;

pub struct RoutingGraph<T> {
    pub graph: H3Graph<T>,

    pub known_target_cells: HashSet<H3Cell>,
}

impl<T> RoutingGraph<T> where T: PartialOrd + PartialEq + Add + Copy {}

impl<T> TryFrom<H3Graph<T>> for RoutingGraph<T>
where
    T: PartialOrd + PartialEq + Add + Copy,
{
    type Error = Error;

    fn try_from(graph: H3Graph<T>) -> std::result::Result<Self, Self::Error> {
        let mut known_target_cells = HashSet::new();
        for edge in graph.edges.keys() {
            known_target_cells.insert(edge.destination_index()?);
        }

        Ok(Self {
            graph,
            known_target_cells,
        })
    }
}
