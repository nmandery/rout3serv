use std::ops::{Add, AddAssign};

use geo::algorithm::simplify::Simplify;
use geo_types::MultiPolygon;
use h3ron::{H3Cell, H3Edge};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::collections::{H3CellMap, H3CellSet, H3EdgeMap};
use crate::error::Error;
use crate::geo_types::Polygon;
use crate::h3ron::{Index, ToLinkedPolygons};
use crate::io::serde_support::h3edgemap as h3m_serde;
use crate::WithH3Resolution;

#[derive(Serialize)]
pub struct GraphStats {
    pub h3_resolution: u8,
    pub num_nodes: usize,
    pub num_edges: usize,
}

#[derive(Serialize, Deserialize, Clone)]
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
    T: PartialOrd + PartialEq + Add + Copy + Send,
{
    pub fn new(h3_resolution: u8) -> Self {
        Self {
            h3_resolution,
            edges: Default::default(),
        }
    }

    pub fn num_nodes(&self) -> Result<usize, Error> {
        Ok(self.nodes()?.len())
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

    /// get all edges in the graph leading to this cell from its neighbors
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
            return Err(Error::MixedH3Resolutions(
                self.h3_resolution,
                other.h3_resolution,
            ));
        }
        for (edge, weight) in other.edges.drain() {
            self.add_edge(edge, weight)?;
        }
        Ok(())
    }

    pub fn stats(&self) -> Result<GraphStats, Error> {
        Ok(GraphStats {
            h3_resolution: self.h3_resolution,
            num_nodes: self.num_nodes()?,
            num_edges: self.num_edges(),
        })
    }

    /// generate a - simplified and overestimating - multipolygon of the area
    /// covered by the graph.
    pub fn covered_area(&self) -> Result<MultiPolygon<f64>, Error> {
        let t_res = self.h3_resolution.saturating_sub(3);
        let mut cells = H3CellSet::new();
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
    pub fn nodes(&self) -> Result<H3CellMap<NodeType>, Error> {
        let mut graph_nodes: H3CellMap<NodeType> = Default::default();
        for edge in self.edges.keys() {
            let cell_from = edge.origin_index()?;
            graph_nodes
                .entry(cell_from)
                .and_modify(|node_type| *node_type += NodeType::Origin)
                .or_insert(NodeType::Origin);

            let cell_to = edge.destination_index()?;
            graph_nodes
                .entry(cell_to)
                .and_modify(|node_type| *node_type += NodeType::Destination)
                .or_insert(NodeType::Destination);
        }
        Ok(graph_nodes)
    }
}

impl<T> WithH3Resolution for H3Graph<T> {
    fn h3_resolution(&self) -> u8 {
        self.h3_resolution
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum NodeType {
    Origin,
    Destination,
    OriginAndDestination,
}

impl NodeType {
    pub fn is_origin(&self) -> bool {
        match self {
            NodeType::Origin => true,
            NodeType::Destination => false,
            NodeType::OriginAndDestination => true,
        }
    }

    pub fn is_destination(&self) -> bool {
        match self {
            NodeType::Origin => false,
            NodeType::Destination => true,
            NodeType::OriginAndDestination => true,
        }
    }
}

impl Add<NodeType> for NodeType {
    type Output = NodeType;

    fn add(self, rhs: NodeType) -> Self::Output {
        if rhs == self {
            self
        } else {
            Self::OriginAndDestination
        }
    }
}

impl AddAssign<NodeType> for NodeType {
    fn add_assign(&mut self, rhs: NodeType) {
        if self != &rhs {
            *self = Self::OriginAndDestination
        }
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

/// change the resolution of a graph to a lower resolution
///
/// the `weight_selector_fn` decides which weight is assigned to a downsampled edge
/// by selecting a weight from all edges between full-resolution childcells.
pub fn downsample_graph<T, F>(
    graph: H3Graph<T>,
    target_h3_resolution: u8,
    weight_selector_fn: F,
) -> Result<H3Graph<T>, Error>
where
    T: Send + Copy,
    F: Fn(T, T) -> T,
{
    if target_h3_resolution >= graph.h3_resolution {
        return Err(Error::TooHighH3Resolution(target_h3_resolution));
    }
    log::debug!(
        "downsampling graph from r={} to r={}",
        graph.h3_resolution(),
        target_h3_resolution
    );
    let cross_cell_edges = graph
        .edges
        .into_iter()
        .par_bridge()
        .filter_map(|(edge, weight)| {
            let cell_from = edge
                .origin_index_unchecked()
                .get_parent_unchecked(target_h3_resolution);
            let cell_to = edge
                .destination_index_unchecked()
                .get_parent_unchecked(target_h3_resolution);
            if cell_from == cell_to {
                None
            } else {
                Some(
                    cell_from
                        .unidirectional_edge_to(&cell_to)
                        .map(|downsamled_edge| (downsamled_edge, weight)),
                )
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut downsampled_edges = H3EdgeMap::new();
    for (edge, weight) in cross_cell_edges {
        downsampled_edges
            .entry(edge)
            .and_modify(|w| *w = weight_selector_fn(*w, weight))
            .or_insert(weight);
    }

    Ok(H3Graph {
        edges: downsampled_edges,
        h3_resolution: target_h3_resolution,
    })
}

pub trait GraphBuilder<T>
where
    T: PartialOrd + PartialEq + Add + Copy,
{
    fn build_graph(self) -> Result<H3Graph<T>, Error>;
}

#[cfg(test)]
mod tests {
    use std::cmp::min;

    use geo_types::{Coordinate, LineString};
    use h3ron::H3Cell;

    use crate::graph::{downsample_graph, H3Graph, NodeType};
    use crate::h3ron::Index;

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
        let downsampled_graph = downsample_graph(
            graph,
            full_h3_res.saturating_sub(3),
            |weight_a, weight_b| min(weight_a, weight_b),
        )
        .unwrap();
        assert!(downsampled_graph.num_edges() > 0);
        assert!(downsampled_graph.num_edges() < 20);
    }

    #[test]
    fn test_nodetype_add() {
        assert_eq!(NodeType::Origin, NodeType::Origin + NodeType::Origin);
        assert_eq!(
            NodeType::Destination,
            NodeType::Destination + NodeType::Destination
        );
        assert_eq!(
            NodeType::OriginAndDestination,
            NodeType::Origin + NodeType::Destination
        );
        assert_eq!(
            NodeType::OriginAndDestination,
            NodeType::OriginAndDestination + NodeType::Destination
        );
        assert_eq!(
            NodeType::OriginAndDestination,
            NodeType::Destination + NodeType::Origin
        );
    }

    #[test]
    fn test_nodetype_addassign() {
        let mut n1 = NodeType::Origin;
        n1 += NodeType::Origin;
        assert_eq!(n1, NodeType::Origin);

        let mut n2 = NodeType::Origin;
        n2 += NodeType::OriginAndDestination;
        assert_eq!(n2, NodeType::OriginAndDestination);

        let mut n3 = NodeType::Destination;
        n3 += NodeType::OriginAndDestination;
        assert_eq!(n3, NodeType::OriginAndDestination);

        let mut n4 = NodeType::Destination;
        n4 += NodeType::Origin;
        assert_eq!(n4, NodeType::OriginAndDestination);
    }
}
