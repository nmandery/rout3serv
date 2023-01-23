use std::ops::Add;

use geo::bounding_rect::BoundingRect;
use geo::concave_hull::ConcaveHull;
use geo::{Coord, MultiPoint, MultiPolygon, Point, Polygon, Rect};
use h3o::{CellIndex, DirectedEdgeIndex, LatLng, Resolution};
use hashbrown::hash_map::Entry;
use num_traits::Zero;
use rayon::prelude::*;

use crate::algorithm::edge::reverse_directed_edge;
use crate::algorithm::graph::covered_area::cells_covered_area;
use crate::algorithm::graph::CoveredArea;
use crate::container::block::Decompressor;
use crate::container::treemap::H3Treemap;
use crate::container::{CellMap, DirectedEdgeMap};
use crate::error::Error;
use crate::graph::fastforward::FastForward;
use crate::graph::node::NodeType;
use crate::graph::{
    EdgeWeight, GetCellEdges, GetCellNode, GetStats, GraphStats, H3EdgeGraph, IterateCellNodes,
};
use crate::HasH3Resolution;

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct OwnedEdgeWeight<W> {
    pub weight: W,

    /// the fastforward is a shortcut which includes many consequent edges while
    /// allowing to visit each of then individually.
    ///
    /// The Box takes care of allocating the LongEdge on the heap. That reduces
    /// the footprint of the OwnedEdgeValue - when W = f32 to nearly 10% compared to
    /// allocating the LongEdge on the stack.
    pub fastforward: Option<Box<(FastForward, W)>>,
}

impl<'a, W> From<&'a OwnedEdgeWeight<W>> for EdgeWeight<'a, W>
where
    W: Copy,
{
    fn from(owned_edge_value: &'a OwnedEdgeWeight<W>) -> Self {
        EdgeWeight {
            weight: owned_edge_value.weight,
            fastforward: owned_edge_value
                .fastforward
                .as_ref()
                .map(|boxed| (&boxed.0, boxed.1)),
        }
    }
}

type OwnedEdgeTuple<W> = (DirectedEdgeIndex, OwnedEdgeWeight<W>);
type OwnedEdgeTupleList<W> = Box<[OwnedEdgeTuple<W>]>;

/// A prepared graph which can be used with a few algorithms.
///
/// Consequent [`DirectedEdgeIndex`] without forks get extended by a [`FastForward`] to allow
/// skipping the individual [`DirectedEdgeIndex`] values for a more efficient graph
/// traversal.
///
/// <p>
#[doc=include_str!("../../doc/images/edges-and-fastforwards.svg")]
/// </p>
///
/// <p>
#[doc=include_str!("../../doc/images/prepared_h3_edge_graph.svg")]
/// </p>
///
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct PreparedH3EdgeGraph<W> {
    outgoing_edges: CellMap<OwnedEdgeTupleList<W>>,
    h3_resolution: Resolution,
    graph_nodes: CellMap<NodeType>,
}

unsafe impl<W> Sync for PreparedH3EdgeGraph<W> where W: Sync {}

impl<W> PreparedH3EdgeGraph<W> {
    /// count the number of edges in the graph
    ///
    /// The returned tuple is (`num_edges`, `num_fast_forwards`)
    pub fn count_edges(&self) -> (usize, usize) {
        let mut num_edges = 0usize;
        let mut num_fast_forwards = 0usize;

        for (_cell, oevs) in self.outgoing_edges.iter() {
            num_edges += oevs.len();
            num_fast_forwards += oevs
                .iter()
                .filter(|(_, oev)| oev.fastforward.is_some())
                .count();
        }
        (num_edges, num_fast_forwards)
    }
}

impl<W> PreparedH3EdgeGraph<W>
where
    W: Copy,
{
    /// iterate over all edges of the graph
    pub fn iter_edges(&self) -> impl Iterator<Item = (DirectedEdgeIndex, EdgeWeight<W>)> {
        self.outgoing_edges
            .iter()
            .flat_map(|(_, oevs)| oevs.iter().map(|(edge, oev)| (*edge, oev.into())))
    }

    /// iterate over all edges of the graph, while skipping simple [`DirectedEdgeIndex`]
    /// which are already covered in other [`FastForward`] instances of the graph.
    ///
    /// This function iterates the graph twice - the first time to collect
    /// all edges which are part of long-edges.
    pub fn iter_edges_non_overlapping(
        &self,
    ) -> Result<impl Iterator<Item = (DirectedEdgeIndex, EdgeWeight<W>)>, Error> {
        let mut covered_edges = H3Treemap::<DirectedEdgeIndex>::default();
        let mut decompressor = Decompressor::default();
        for (_, owned_edge_values) in self.outgoing_edges.iter() {
            for (_, owned_edge_value) in owned_edge_values.iter() {
                if let Some(boxed_fastforward) = owned_edge_value.fastforward.as_ref() {
                    for edge in decompressor
                        .decompress_block::<DirectedEdgeIndex>(&boxed_fastforward.0.edge_path)?
                        .skip(1)
                    {
                        covered_edges.insert(edge?);
                    }
                }
            }
        }
        Ok(self.iter_edges().filter_map(move |(edge, weight)| {
            if covered_edges.contains(&edge) {
                None
            } else {
                Some((edge, weight))
            }
        }))
    }
}

/// Iterator item type to build [`PreparedH3EdgeGraph`] from
pub type FromIterItem<W> = (DirectedEdgeIndex, W, Option<(Vec<DirectedEdgeIndex>, W)>);

impl<W> PreparedH3EdgeGraph<W>
where
    W: Copy + Send + Sync,
{
    pub fn try_from_iter<I>(iter: I) -> Result<Self, Error>
    where
        I: Iterator<Item = FromIterItem<W>>,
    {
        let mut h3_resolution = None;
        let mut outgoing_edges: CellMap<Vec<OwnedEdgeTuple<W>>> = Default::default();
        let mut graph_nodes: CellMap<NodeType> = Default::default();

        for (edge, edge_weight, fastforward_components) in iter {
            let (origin, destination) = edge.cells();

            // ensure no mixed h3 resolutions
            if let Some(h3_resolution) = h3_resolution {
                if h3_resolution != origin.resolution() {
                    return Err(Error::MixedH3Resolutions(
                        h3_resolution,
                        origin.resolution(),
                    ));
                }
            } else {
                h3_resolution = Some(origin.resolution());
            }

            graph_nodes
                .entry(origin)
                .and_modify(|nt| *nt += NodeType::Origin)
                .or_insert(NodeType::Origin);
            graph_nodes
                .entry(destination)
                .and_modify(|nt| *nt += NodeType::Destination)
                .or_insert(NodeType::Destination);

            let edge_with_weight = (
                edge,
                OwnedEdgeWeight {
                    weight: edge_weight,
                    fastforward: match fastforward_components {
                        Some((le_edges, le_weight)) => {
                            Some(Box::new((FastForward::try_from(le_edges)?, le_weight)))
                        }
                        None => None,
                    },
                },
            );
            match outgoing_edges.entry(origin) {
                Entry::Occupied(mut occ) => {
                    occ.get_mut().push(edge_with_weight);
                }
                Entry::Vacant(vac) => {
                    vac.insert(vec![edge_with_weight]);
                }
            }
        }

        let outgoing_edges = remove_duplicated_edges(outgoing_edges);

        if let Some(h3_resolution) = h3_resolution {
            Ok(Self {
                outgoing_edges,
                h3_resolution,
                graph_nodes,
            })
        } else {
            Err(Error::InsufficientNumberOfEdges)
        }
    }
}

impl<W> HasH3Resolution for PreparedH3EdgeGraph<W> {
    fn h3_resolution(&self) -> Resolution {
        self.h3_resolution
    }
}

impl<W> GetStats for PreparedH3EdgeGraph<W> {
    fn get_stats(&self) -> Result<GraphStats, Error> {
        Ok(GraphStats {
            h3_resolution: self.h3_resolution,
            num_nodes: self.graph_nodes.len(),
            num_edges: self.count_edges().0,
        })
    }
}

impl<W> GetCellNode for PreparedH3EdgeGraph<W> {
    fn get_cell_node(&self, cell: CellIndex) -> Option<NodeType> {
        self.graph_nodes.get(&cell).copied()
    }
}

impl<W: Copy> GetCellEdges for PreparedH3EdgeGraph<W> {
    type EdgeWeightType = W;

    fn get_edges_originating_from(
        &self,
        cell: CellIndex,
    ) -> Vec<(DirectedEdgeIndex, EdgeWeight<Self::EdgeWeightType>)> {
        let mut out_vec = Vec::with_capacity(7);
        if let Some(edges_with_weights) = self.outgoing_edges.get(&cell) {
            out_vec.extend(
                edges_with_weights
                    .iter()
                    .map(|(edge, owv)| (*edge, owv.into())),
            );
        }
        out_vec
    }
}

const MIN_LONGEDGE_LENGTH: usize = 3;

fn to_fastforward_edges<W>(
    input_graph: H3EdgeGraph<W>,
    min_fastforward_length: usize,
) -> Result<CellMap<OwnedEdgeTupleList<W>>, Error>
where
    W: PartialOrd + PartialEq + Add<Output = W> + Copy + Send + Sync,
{
    if min_fastforward_length < MIN_LONGEDGE_LENGTH {
        return Err(Error::TooShortLongEdge(min_fastforward_length));
    }

    let outgoing_edge_vecs = input_graph
        .edges
        .par_iter()
        .try_fold(Vec::new, |mut output_vec, (edge, weight)| {
            assemble_edge_with_fastforward(
                &input_graph.edges,
                min_fastforward_length,
                *edge,
                weight,
            )
            .map(|cell_edge| {
                output_vec.push(cell_edge);
                output_vec
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut outgoing_edges: CellMap<Vec<_>> = Default::default();
    for outgoing_edge_vec in outgoing_edge_vecs.into_iter() {
        for (cell, edge_with_weight) in outgoing_edge_vec.into_iter() {
            match outgoing_edges.entry(cell) {
                Entry::Occupied(mut occ) => occ.get_mut().push(edge_with_weight),
                Entry::Vacant(vac) => {
                    vac.insert(vec![edge_with_weight]);
                }
            }
        }
    }

    let outgoing_edges = remove_duplicated_edges(outgoing_edges);

    Ok(outgoing_edges)
}

/// remove duplicates if there are any. Ignores any differences in weights
fn remove_duplicated_edges<W>(
    outgoing_edges: CellMap<Vec<OwnedEdgeTuple<W>>>,
) -> CellMap<OwnedEdgeTupleList<W>>
where
    W: Send + Sync,
{
    outgoing_edges
        .into_par_iter()
        .map(|(cell, mut edges_with_weights)| {
            edges_with_weights.sort_unstable_by_key(|eww| eww.0);
            edges_with_weights.dedup_by(|a, b| a.0 == b.0);

            (cell, edges_with_weights.into_boxed_slice())
        })
        .collect()
}

fn assemble_edge_with_fastforward<W>(
    input_edges: &DirectedEdgeMap<W>,
    min_fastforward_length: usize,
    edge: DirectedEdgeIndex,
    weight: &W,
) -> Result<(CellIndex, OwnedEdgeTuple<W>), Error>
where
    W: PartialOrd + PartialEq + Add<Output = W> + Copy,
{
    let mut graph_entry = OwnedEdgeWeight {
        weight: *weight,
        fastforward: None,
    };

    let origin_cell = edge.origin();

    // number of upstream edges leading to this one
    let num_edges_leading_to_this_one = origin_cell
        .edges()
        .filter(|new_edge| *new_edge != edge) // ignore the backwards edge
        .filter(|new_edge| input_edges.get(&reverse_directed_edge(*new_edge)).is_some())
        .count();

    // attempt to build a fastforward when this edge is either the end of a path, or a path
    // starting after a conjunction of multiple edges
    if num_edges_leading_to_this_one != 1 {
        let mut edge_path = vec![edge];
        let mut fastforward_weight = *weight;

        let mut last_edge = edge;
        loop {
            let last_edge_reverse = reverse_directed_edge(last_edge);
            // follow the edges until the end or a conjunction is reached
            let following_edges: Vec<_> = last_edge
                .destination()
                .edges()
                .filter_map(|this_edge| {
                    if this_edge != last_edge_reverse {
                        input_edges.get_key_value(&this_edge)
                    } else {
                        None
                    }
                })
                .collect();

            // found no further continuing edge or conjunction
            if following_edges.len() != 1 {
                break;
            }
            let following_edge = *(following_edges[0].0);

            // stop when encountering circles
            if edge_path.contains(&following_edge) {
                break;
            }

            edge_path.push(following_edge);
            fastforward_weight = *(following_edges[0].1) + fastforward_weight;
            // find the next following edge in the next iteration of the loop
            last_edge = following_edge;
        }

        if edge_path.len() >= min_fastforward_length {
            graph_entry.fastforward = Some(Box::new((
                FastForward::try_from(edge_path)?,
                fastforward_weight,
            )));
        }
    }
    Ok((origin_cell, (edge, graph_entry)))
}

impl<W> PreparedH3EdgeGraph<W>
where
    W: PartialOrd + PartialEq + Add + Copy + Ord + Zero + Send + Sync,
{
    pub fn from_h3edge_graph(
        graph: H3EdgeGraph<W>,
        min_fastforward_length: usize,
    ) -> Result<Self, Error> {
        let h3_resolution = graph.h3_resolution();
        let graph_nodes = graph.nodes();
        let outgoing_edges = to_fastforward_edges(graph, min_fastforward_length)?;
        Ok(Self {
            graph_nodes,
            h3_resolution,
            outgoing_edges,
        })
    }
}

impl<W> TryFrom<H3EdgeGraph<W>> for PreparedH3EdgeGraph<W>
where
    W: PartialOrd + PartialEq + Add + Copy + Ord + Zero + Send + Sync,
{
    type Error = Error;

    fn try_from(graph: H3EdgeGraph<W>) -> Result<Self, Self::Error> {
        Self::from_h3edge_graph(graph, 4)
    }
}

impl<W> From<PreparedH3EdgeGraph<W>> for H3EdgeGraph<W>
where
    W: PartialOrd + PartialEq + Add + Copy + Ord + Zero,
{
    fn from(prepared_graph: PreparedH3EdgeGraph<W>) -> Self {
        Self {
            edges: prepared_graph
                .iter_edges()
                .map(|(edge, edge_value)| (edge, edge_value.weight))
                .collect(),
            h3_resolution: prepared_graph.h3_resolution,
        }
    }
}

impl<W> CoveredArea for PreparedH3EdgeGraph<W> {
    type Error = Error;

    fn covered_area(&self, reduce_resolution_by: u8) -> Result<MultiPolygon<f64>, Self::Error> {
        cells_covered_area(
            self.graph_nodes.iter().map(|(cell, _)| cell),
            self.h3_resolution(),
            reduce_resolution_by,
        )
    }
}

impl<'a, W> IterateCellNodes<'a> for PreparedH3EdgeGraph<W> {
    type CellNodeIterator = hashbrown::hash_map::Iter<'a, CellIndex, NodeType>;

    fn iter_cell_nodes(&'a self) -> Self::CellNodeIterator {
        self.graph_nodes.iter()
    }
}

impl<W> ConcaveHull for PreparedH3EdgeGraph<W> {
    type Scalar = f64;

    /// concave hull - this implementation leaves out invalid cells
    fn concave_hull(&self, concavity: Self::Scalar) -> Polygon<Self::Scalar> {
        let mpoint = MultiPoint::from(
            self.iter_cell_nodes()
                .map(|(cell, _)| {
                    let coord: Coord = LatLng::from(*cell).into();
                    Point::from(coord)
                })
                .collect::<Vec<_>>(),
        );
        mpoint.concave_hull(concavity)
    }
}

impl<W> BoundingRect<f64> for PreparedH3EdgeGraph<W> {
    type Output = Option<Rect<f64>>;

    fn bounding_rect(&self) -> Self::Output {
        let mut iter = self.iter_cell_nodes();
        let mut rect = {
            // consume until encountering the first valid cell
            if let Some(coord) = iter
                .next()
                .map(|(cell, _)| -> Coord { LatLng::from(*cell).into() })
            {
                Point::from(coord).bounding_rect()
            } else {
                return None;
            }
        };

        for (cell, _) in iter {
            let coord: Coord = LatLng::from(*cell).into();
            rect = Rect::new(
                Coord {
                    x: if coord.x < rect.min().x {
                        coord.x
                    } else {
                        rect.min().x
                    },
                    y: if coord.y < rect.min().y {
                        coord.y
                    } else {
                        rect.min().y
                    },
                },
                Coord {
                    x: if coord.x > rect.max().x {
                        coord.x
                    } else {
                        rect.max().x
                    },
                    y: if coord.y > rect.max().y {
                        coord.y
                    } else {
                        rect.max().y
                    },
                },
            );
        }
        Some(rect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::LineString;
    use h3o::geom::ToCells;

    fn build_line_prepared_graph() -> PreparedH3EdgeGraph<u32> {
        let full_h3_res = Resolution::Eight;
        let cells: Vec<_> = h3o::geom::LineString::from_degrees(LineString::from(vec![
            Coord::from((23.3, 12.3)),
            Coord::from((24.2, 12.2)),
        ]))
        .unwrap()
        .to_cells(full_h3_res)
        .collect();
        assert!(cells.len() > 100);

        let mut graph = H3EdgeGraph::new(full_h3_res);
        for w in cells.windows(2) {
            graph.add_edge(w[0].edge(w[1]).unwrap(), 20u32);
        }
        assert!(graph.num_edges() > 50);
        let prep_graph: PreparedH3EdgeGraph<_> = graph.try_into().unwrap();
        assert_eq!(prep_graph.count_edges().1, 1);
        prep_graph
    }

    #[test]
    fn test_iter_edges() {
        let graph = build_line_prepared_graph();
        assert!(graph.iter_edges().count() > 50);
    }

    #[test]
    fn test_iter_non_overlapping_edges() {
        let graph = build_line_prepared_graph();
        assert_eq!(graph.iter_edges_non_overlapping().unwrap().count(), 1);
    }
}
