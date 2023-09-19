use std::borrow::Borrow;

use geo::algorithm::simplify::Simplify;
use geo::{MultiPolygon, Polygon};

use crate::algorithm::resolution::transform_resolution;
use crate::container::CellSet;
use h3o::geom::ToGeo;
use h3o::{CellIndex, Resolution};

use crate::error::Error;

/// calculates a [`MultiPolygon`] of the area covered by a graph
pub trait CoveredArea {
    type Error;

    /// calculates a [`MultiPolygon`] of the area covered by a graph
    ///
    /// As the resulting geometry will be quite complex, it is recommended
    /// to reduce the h3 resolution using `reduce_resolution_by`. A value of 3
    /// will make the calculation based on resolution 7 for a graph of resolution 10.
    /// Reducing the resolution leads to a overestimation of the area.
    ///
    /// A slight simplification will be applied to the output geometry and
    /// eventual holes will be removed.
    fn covered_area(&self, reduce_resolution_by: u8) -> Result<MultiPolygon<f64>, Self::Error>;
}

/// calculates a [`MultiPolygon`] of the area covered by a [`CellIndex`] iterator.
pub(crate) fn cells_covered_area<I>(
    cell_iter: I,
    cell_iter_resolution: Resolution,
    reduce_resolution_by: u8,
) -> Result<MultiPolygon<f64>, Error>
where
    I: IntoIterator,
    I::Item: Borrow<CellIndex>,
{
    let t_res: Resolution = {
        let r: u8 = cell_iter_resolution.into();
        r.saturating_sub(reduce_resolution_by).try_into().unwrap()
    };
    let cells: CellSet = transform_resolution(cell_iter, t_res).collect();
    Ok(MultiPolygon::new(
        cells
            .into_iter()
            .to_geom(true)?
            .0
            .into_iter()
            // reduce the number of vertices again and discard all holes
            .map(|p| Polygon::new(p.exterior().simplify(&0.000001), vec![]))
            .collect::<Vec<_>>(),
    ))
}
