use std::cmp::Ordering;

use crate::algorithm::edge::cell_centroid_distance_m;
use geo::LineString;
use h3o::geom::ToGeo;
use h3o::{CellIndex, DirectedEdgeIndex};

use crate::algorithm::geom::edges_to_multilinestring;

use crate::error::Error;

/// [DirectedEdgePath] describes a path between a cell and another.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DirectedEdgePath {
    /// path is empty as origin and destination are the same.
    OriginIsDestination(CellIndex),

    /// a sequence of edges describing the path.
    ///
    /// The edges in the vec are expected to be consecutive.
    ///
    /// The cost is the total cost summed for all of the edges.
    DirectedEdgeSequence(Vec<DirectedEdgeIndex>),
}

impl DirectedEdgePath {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::OriginIsDestination(_) => true,
            Self::DirectedEdgeSequence(edges) => edges.is_empty(),
        }
    }

    /// Length of the path in number of edges
    pub fn len(&self) -> usize {
        match self {
            Self::OriginIsDestination(_) => 0,
            Self::DirectedEdgeSequence(edges) => edges.len(),
        }
    }

    pub fn origin_cell(&self) -> Result<CellIndex, Error> {
        match self {
            Self::OriginIsDestination(cell) => Ok(*cell),
            Self::DirectedEdgeSequence(edges) => {
                if let Some(edge) = edges.first() {
                    Ok(edge.origin())
                } else {
                    Err(Error::EmptyPath)
                }
            }
        }
    }

    pub fn destination_cell(&self) -> Result<CellIndex, Error> {
        match self {
            Self::OriginIsDestination(cell) => Ok(*cell),
            Self::DirectedEdgeSequence(edges) => {
                if let Some(edge) = edges.last() {
                    Ok(edge.destination())
                } else {
                    Err(Error::EmptyPath)
                }
            }
        }
    }

    pub fn to_linestring(&self) -> Result<LineString, Error> {
        match self {
            Self::OriginIsDestination(_) => Err(Error::InsufficientNumberOfEdges),
            Self::DirectedEdgeSequence(edges) => match edges.len() {
                0 => Err(Error::InsufficientNumberOfEdges),
                1 => Ok(edges[0].to_geom(true).unwrap().into()),
                _ => {
                    let mut multilinesstring = edges_to_multilinestring(edges.iter().copied());
                    match multilinesstring.0.len() {
                        0 => Err(Error::InsufficientNumberOfEdges),
                        1 => Ok(multilinesstring.0.remove(0)),
                        _ => Err(Error::SegmentedPath),
                    }
                }
            },
        }
    }

    pub fn edges(&self) -> &[DirectedEdgeIndex] {
        match self {
            Self::DirectedEdgeSequence(edges) => edges.as_slice(),
            Self::OriginIsDestination(_) => &[],
        }
    }

    /// return a vec of all [`CellIndex`] the path passes through.
    pub fn cells(&self) -> Vec<CellIndex> {
        match self {
            Self::OriginIsDestination(cell) => vec![*cell],
            Self::DirectedEdgeSequence(edges) => {
                let mut cells = Vec::with_capacity(edges.len() * 2);
                for edge in edges.iter() {
                    cells.push(edge.origin());
                    cells.push(edge.destination());
                }
                cells.dedup();
                cells.shrink_to_fit();
                cells
            }
        }
    }

    /// calculate the length of the path in meters using the exact length of the
    /// contained edges
    pub fn length_m(&self) -> f64 {
        match self {
            Self::OriginIsDestination(_) => 0.0,
            Self::DirectedEdgeSequence(edges) => {
                edges.iter().copied().map(cell_centroid_distance_m).sum()
            }
        }
    }
}

/// [Path] describes a path between a cell and another with an associated cost
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Path<W> {
    /// The cell the path starts at.
    ///
    /// This is the cell the path was calculated from. The actual start cell of the
    /// path may differ in case `origin_cell` is not directly connected to the graph
    pub origin_cell: CellIndex,

    /// The cell the path ends at.
    ///
    /// This is the cell the path was calculated to. The actual end cell of the
    /// path may differ in case `destination_cell` is not directly connected to the graph
    pub destination_cell: CellIndex,

    pub cost: W,

    /// describes the path
    pub directed_edge_path: DirectedEdgePath,
}

impl<W> Path<W> {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.directed_edge_path.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.directed_edge_path.len()
    }
}

impl<W> TryFrom<(DirectedEdgePath, W)> for Path<W> {
    type Error = Error;

    fn try_from((path_directed_edges, cost): (DirectedEdgePath, W)) -> Result<Self, Self::Error> {
        let origin_cell = path_directed_edges.origin_cell()?;
        let destination_cell = path_directed_edges.destination_cell()?;
        Ok(Self {
            origin_cell,
            destination_cell,
            cost,
            directed_edge_path: path_directed_edges,
        })
    }
}

impl PartialOrd<Self> for DirectedEdgePath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DirectedEdgePath {
    fn cmp(&self, other: &Self) -> Ordering {
        let cmp_origin = index_or_zero(self.origin_cell()).cmp(&index_or_zero(other.origin_cell()));
        if cmp_origin == Ordering::Equal {
            index_or_zero(self.destination_cell()).cmp(&index_or_zero(other.destination_cell()))
        } else {
            cmp_origin
        }
    }
}

/// order by cost, origin index and destination_index.
///
/// This ordering can used to bring `Vec`s of routes in a deterministic order to make them
/// comparable
impl<W> Ord for Path<W>
where
    W: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        let cmp_cost = self.cost.cmp(&other.cost);
        if cmp_cost == Ordering::Equal {
            self.directed_edge_path.cmp(&other.directed_edge_path)
        } else {
            cmp_cost
        }
    }
}

impl<W> PartialOrd for Path<W>
where
    W: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[inline]
fn index_or_zero(cell: Result<CellIndex, Error>) -> u64 {
    cell.map(|c| c.into()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use h3o::DirectedEdgeIndex;

    use super::{DirectedEdgePath, Path};

    #[test]
    fn pathdirectededges_deterministic_ordering() {
        let r1 = DirectedEdgePath::DirectedEdgeSequence(vec![DirectedEdgeIndex::try_from(
            0x1176b49474ffffff,
        )
        .unwrap()]);
        let r2 = DirectedEdgePath::DirectedEdgeSequence(vec![DirectedEdgeIndex::try_from(
            0x1476b49474ffffff,
        )
        .unwrap()]);
        let mut paths = vec![r2.clone(), r1.clone()];
        paths.sort_unstable();
        assert_eq!(paths[0], r1);
        assert_eq!(paths[1], r2);
    }

    #[test]
    fn paths_deterministic_ordering() {
        let r1: Path<_> = (
            DirectedEdgePath::DirectedEdgeSequence(vec![DirectedEdgeIndex::try_from(
                0x1176b49474ffffff,
            )
            .unwrap()]),
            1,
        )
            .try_into()
            .unwrap();
        let r2: Path<_> = (
            DirectedEdgePath::DirectedEdgeSequence(vec![DirectedEdgeIndex::try_from(
                0x1476b49474ffffff,
            )
            .unwrap()]),
            3,
        )
            .try_into()
            .unwrap();
        let r3: Path<_> = (
            DirectedEdgePath::DirectedEdgeSequence(vec![DirectedEdgeIndex::try_from(
                0x1476b4b2c2ffffff,
            )
            .unwrap()]),
            3,
        )
            .try_into()
            .unwrap();
        let mut paths = vec![r3.clone(), r1.clone(), r2.clone()];
        paths.sort_unstable();
        assert_eq!(paths[0], r1);
        assert_eq!(paths[1], r2);
        assert_eq!(paths[2], r3);
    }
}
