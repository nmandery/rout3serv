use std::borrow::Borrow;

use crate::algorithm::geom::edges_to_multilinestring;
use geo::LineString;
use h3o::{CellIndex, DirectedEdgeIndex};

use crate::container::block::{Decompressor, IndexBlock};
use crate::container::treemap::H3Treemap;

use crate::error::Error;

/// A `FastForward` is an artificial construct to combine a continuous path
/// of [`DirectedEdgeIndex`] values into a single edge.
///
/// This intended to be used to compress longer paths into a single edge to
/// reduce the number of nodes to visit during routing.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct FastForward {
    pub in_edge: DirectedEdgeIndex,
    pub out_edge: DirectedEdgeIndex,

    /// the path of the fastforward described by multiple, successive
    /// [`DirectedEdgeIndex`] values.
    pub edge_path: IndexBlock<DirectedEdgeIndex>,

    /// provides an efficient lookup to check for intersection of
    /// the edge with [`CellIndex`] values.
    cell_lookup: H3Treemap<CellIndex>,
}

impl FastForward {
    pub fn destination_cell(&self) -> CellIndex {
        self.out_edge.destination()
    }

    pub fn origin_cell(&self) -> CellIndex {
        self.in_edge.origin()
    }

    pub fn is_disjoint(&self, celltreemap: &H3Treemap<CellIndex>) -> bool {
        self.cell_lookup.is_disjoint(celltreemap)
    }

    /// length of `self` as the number of contained h3edges
    pub const fn h3edges_len(&self) -> usize {
        self.edge_path.len().saturating_sub(1)
    }

    pub fn to_linestring(&self) -> Result<LineString<f64>, Error> {
        let mut decomp = Decompressor::new();
        let edges = decomp
            .decompress_block(&self.edge_path)?
            .collect::<Result<Vec<_>, _>>()?;
        let mut mls = edges_to_multilinestring(edges.into_iter());

        if mls.0.len() != 1 {
            Err(Error::SegmentedPath)
        } else {
            Ok(mls.0.swap_remove(0))
        }
    }
}

/// construct an [`FastForward`] from a vec of [`DirectedEdgeIndex`].
///
/// The [`DirectedEdgeIndex`] must be sorted according to the path they describe
impl TryFrom<Vec<DirectedEdgeIndex>> for FastForward {
    type Error = Error;

    fn try_from(mut h3edges: Vec<DirectedEdgeIndex>) -> Result<Self, Self::Error> {
        h3edges.dedup();
        h3edges.shrink_to_fit();
        if h3edges.len() >= 2 {
            let cell_lookup: H3Treemap<_> =
                h3edge_path_to_h3cell_path(&h3edges).into_iter().collect();
            Ok(Self {
                in_edge: h3edges[0],
                out_edge: *h3edges.last().unwrap(),
                edge_path: h3edges.into(),
                cell_lookup,
            })
        } else {
            Err(Error::InsufficientNumberOfEdges)
        }
    }
}

/// `h3dge_path` is a iterator of [`DirectedEdgeIndex`] where the edges form a continuous path
fn h3edge_path_to_h3cell_path<I>(h3edge_path: I) -> Vec<CellIndex>
where
    I: IntoIterator,
    I::Item: Borrow<DirectedEdgeIndex>,
{
    let mut iter = h3edge_path.into_iter();
    let mut out_vec = Vec::with_capacity(iter.size_hint().0 + 1);
    if let Some(h3edge) = iter.next() {
        out_vec.push(h3edge.borrow().origin());
        out_vec.push(h3edge.borrow().destination());
    }
    for h3edge in iter {
        out_vec.push(h3edge.borrow().destination());
    }
    out_vec
}
