use std::collections::HashSet;
use std::ops::Add;

use geo::algorithm::simplify::Simplify;
use geo_types::MultiPolygon;
use h3ron::{H3Cell, H3Edge};
use serde::{Deserialize, Serialize};

use crate::error::Error;
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
    ) -> Result<(), Error> {
        let edge = cell_from.unidirectional_edge_to(&cell_to)?;
        self.add_edge(edge, weight)
    }

    pub fn add_edge_using_cells_bidirectional(
        &mut self,
        cell_from: H3Cell,
        cell_to: H3Cell,
        weight: T,
    ) -> Result<(), Error> {
        self.add_edge_using_cells(cell_from, cell_to, weight)?;
        self.add_edge_using_cells(cell_to, cell_from, weight)
    }

    pub fn add_edge(&mut self, edge: H3Edge, weight: T) -> Result<(), Error> {
        edgemap_add_edge(&mut self.edges, edge, weight);
        Ok(())
    }

    pub fn try_add(&mut self, mut other: H3Graph<T>) -> Result<(), Error> {
        if self.h3_resolution != other.h3_resolution {
            return Err(Error::MixedH3Resolutions(self.h3_resolution, other.h3_resolution).into());
        }
        for (edge, weight) in other.edges.drain() {
            self.add_edge(edge, weight)?;
        }
        Ok(())
    }

    pub fn downsample(&self, target_h3_resolution: u8) -> Result<Self, Error> {
        if target_h3_resolution >= self.h3_resolution {
            return Err(Error::TooHighH3Resolution(target_h3_resolution));
        }
        let mut downsampled_edges = Default::default();
        for (edge, weight) in self.edges.iter() {
            let cell_from = edge.origin_index()?.get_parent(target_h3_resolution)?;
            let cell_to = edge.destination_index()?.get_parent(target_h3_resolution)?;
            if cell_from == cell_to {
                // no need to add self-edges
                continue;
            }
            let downsampled_edge = cell_from.unidirectional_edge_to(&cell_to)?;
            edgemap_add_edge(&mut downsampled_edges, downsampled_edge, *weight)
        }
        Ok(Self {
            edges: downsampled_edges,
            h3_resolution: target_h3_resolution,
        })
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
    pub fn covered_area(&self) -> Result<MultiPolygon<f64>, Error> {
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

    /// cells which are valid targets to route to
    pub fn valid_target_cells(&self) -> Result<HashSet<H3Cell>, Error> {
        let cells = self
            .edges
            .keys()
            .map(|edge| edge.destination_index())
            .collect::<Result<HashSet<_>, _>>()?;
        Ok(cells)
    }
}

fn edgemap_add_edge<T>(edgemap: &mut H3EdgeMap<T>, edge: H3Edge, weight: T)
where
    T: Copy + PartialOrd,
{
    edgemap
        .entry(edge)
        .and_modify(|self_weight| {
            // lower weight takes precedence
            if weight < *self_weight {
                *self_weight = weight
            }
        })
        .or_insert(weight);
}

pub trait GraphBuilder<T>
where
    T: PartialOrd + PartialEq + Add + Copy,
{
    fn build_graph(self) -> Result<H3Graph<T>, Error>;
}

#[cfg(test)]
mod tests {
    use geo_types::{Coordinate, LineString};
    use h3ron::H3Cell;

    use crate::graph::H3Graph;
    use crate::h3ron::Index;
    use crate::H3EdgeMap;

    #[test]
    fn test_downsample() {
        let full_h3_res = 8;
        let cells = h3ron::line(
            &LineString::from(vec![
                Coordinate::from((23.3, 12.3)),
                Coordinate::from((24.2, 12.2)),
            ]),
            full_h3_res,
        )
        .unwrap();
        assert!(cells.len() > 100);

        let mut graph = H3Graph::new(full_h3_res);
        for w in cells.windows(2) {
            graph
                .add_edge_using_cells(H3Cell::new(w[0]), H3Cell::new(w[1]), 20)
                .unwrap();
        }
        assert!(graph.num_edges() > 50);
        let downsampled_graph = graph.downsample(full_h3_res.saturating_sub(3)).unwrap();
        assert!(downsampled_graph.num_edges() > 0);
        assert!(downsampled_graph.num_edges() < 20);
    }
}
