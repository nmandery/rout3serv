use std::collections::HashSet;
use std::convert::TryFrom;
use std::ops::Add;

use crate::error::Error;
use crate::graph::H3Graph;
use crate::h3ron::H3Cell;

pub struct RoutingGraph<T> {
    pub graph: H3Graph<T>,
    pub downsampled_graph: H3Graph<T>,

    pub valid_target_cells: HashSet<H3Cell>,
    pub downsampled_valid_target_cells: HashSet<H3Cell>,
}

impl<T> RoutingGraph<T> where T: PartialOrd + PartialEq + Add + Copy {}

impl<T> TryFrom<H3Graph<T>> for RoutingGraph<T>
where
    T: PartialOrd + PartialEq + Add + Copy,
{
    type Error = Error;

    fn try_from(graph: H3Graph<T>) -> std::result::Result<Self, Self::Error> {
        let downsampled_graph = graph.downsample(graph.h3_resolution.saturating_sub(3))?;

        let valid_target_cells = graph.valid_target_cells()?;
        let downsampled_valid_target_cells = downsampled_graph.valid_target_cells()?;
        Ok(Self {
            graph,
            downsampled_graph,
            valid_target_cells,
            downsampled_valid_target_cells,
        })
    }
}
