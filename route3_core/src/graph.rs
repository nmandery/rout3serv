use std::collections::HashSet;
use std::ops::Add;

use eyre::{Report, Result};
use geo::algorithm::simplify::Simplify;
use geo_types::MultiPolygon;
use h3ron::{H3Cell, H3Edge};
use serde::{Deserialize, Serialize};

use crate::geo_types::Polygon;
use crate::h3ron::{Index, ToLinkedPolygons};
use crate::serde::h3edgemap as h3m_serde;
use crate::H3EdgeMap;

#[derive(Serialize)]
pub struct GraphStats {
    pub h3_resolution: u8,
    pub num_nodes: usize,
    pub num_edges: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct H3Graph<T> {
    // flexbuffers can only handle maps with string keys
    #[serde(
        deserialize_with = "h3m_serde::deserialize",
        serialize_with = "h3m_serde::serialize"
    )]
    pub edges: H3EdgeMap<T>,
    pub h3_resolution: u8,
}

impl<T> H3Graph<T>
where
    T: PartialOrd + PartialEq + Add + Copy,
{
    pub fn new(h3_resolution: u8) -> Self {
        Self {
            h3_resolution,
            edges: Default::default(),
        }
    }

    pub fn num_nodes(&self) -> usize {
        let mut node_set = HashSet::new();
        for (edge, _) in self.edges.iter() {
            node_set.insert(edge.destination_index_unchecked());
            node_set.insert(edge.origin_index_unchecked());
        }
        node_set.len()
    }

    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }

    pub fn edge_weight(&self, edge: &H3Edge) -> Option<&T> {
        self.edges.get(edge)
    }

    /// get all edges in the graph leading from this edge to neighbors
    pub fn edges_from_cell(&self, cell: &H3Cell) -> Vec<(&H3Edge, &T)> {
        cell.unidirectional_edges()
            .iter()
            .filter_map(|edge| self.edges.get_key_value(edge))
            .collect()
    }

    /// get all edges in the graph leading to this cell to neighbors
    pub fn edges_to_cell(&self, cell: &H3Cell) -> Vec<(&H3Edge, &T)> {
        cell.k_ring(1)
            .drain(..)
            .filter(|ring_cell| ring_cell != cell)
            .flat_map(|ring_cell| {
                ring_cell
                    .unidirectional_edges()
                    .drain(..)
                    .filter_map(|edge| self.edges.get_key_value(&edge))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    pub fn add_edge_using_cells(
        &mut self,
        cell_from: H3Cell,
        cell_to: H3Cell,
        weight: T,
    ) -> Result<()> {
        let edge = cell_from.unidirectional_edge_to(&cell_to)?;
        self.add_edge(edge, weight);
        Ok(())
    }

    pub fn add_edge_using_cells_bidirectional(
        &mut self,
        cell_from: H3Cell,
        cell_to: H3Cell,
        weight: T,
    ) -> Result<()> {
        self.add_edge_using_cells(cell_from, cell_to, weight)?;
        self.add_edge_using_cells(cell_to, cell_from, weight)
    }

    pub fn add_edge(&mut self, edge: H3Edge, weight: T) {
        self.edges
            .entry(edge)
            .and_modify(|self_weight| {
                // lower weight takes precedence
                if weight < *self_weight {
                    *self_weight = weight
                }
            })
            .or_insert(weight);
    }

    pub fn try_add(&mut self, mut other: H3Graph<T>) -> Result<()> {
        if self.h3_resolution != other.h3_resolution {
            return Err(Report::from(h3ron::error::Error::MixedResolutions(
                self.h3_resolution,
                other.h3_resolution,
            )));
        }
        for (edge, weight) in other.edges.drain() {
            self.add_edge(edge, weight);
        }
        Ok(())
    }

    pub fn stats(&self) -> GraphStats {
        GraphStats {
            h3_resolution: self.h3_resolution,
            num_nodes: self.num_nodes(),
            num_edges: self.num_edges(),
        }
    }

    /// generate a - simplified and overestimating - multipolygon of the area
    /// covered by the graph.
    pub fn covered_area(&self) -> Result<MultiPolygon<f64>> {
        let t_res = self.h3_resolution.saturating_sub(3);
        let mut cells = HashSet::new();
        for (edge, _) in self.edges.iter() {
            cells.insert(edge.origin_index_unchecked().get_parent(t_res)?);
            cells.insert(edge.origin_index_unchecked().get_parent(t_res)?);
        }
        let cell_vec: Vec<_> = cells.drain().collect();
        let mp = MultiPolygon::from(
            cell_vec
                // remove the number of vertices by smoothing
                .to_linked_polygons(true)
                .drain(..)
                // reduce the number of vertices again and discard all holes
                .map(|p| Polygon::new(p.exterior().simplify(&0.000001), vec![]))
                .collect::<Vec<_>>(),
        );
        Ok(mp)
    }
}

pub trait GraphBuilder<T>
where
    T: PartialOrd + PartialEq + Add + Copy,
{
    fn build_graph(self) -> Result<H3Graph<T>>;
}

#[cfg(test)]
mod tests {}
