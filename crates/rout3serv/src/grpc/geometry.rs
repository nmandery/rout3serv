//! vector geometry handling
//!
use geo::algorithm::centroid::Centroid;
use geo_types::Geometry;
use h3o::geom::ToCells;
use h3o::{CellIndex, LatLng, Resolution};
use tonic::{Code, Status};
use tracing::Level;
use uom::si::f64::Length;
use uom::si::length::meter;

use crate::grpc::error::{logged_status, ToStatusResult};

/// read binary WKB into a gdal `Geometry`
pub fn from_wkb(wkb_bytes: &[u8]) -> Result<Geometry, Status> {
    crate::geo::wkb::from_wkb(wkb_bytes)
        .map_err(|e| logged_status!("Can not parse WKB", Code::InvalidArgument, Level::WARN, &e))
}

/// convert a [`Geometry`] to a vec of [`CellIndex`].
pub fn geom_to_h3(
    geom: Geometry,
    h3_resolution: Resolution,
    include_centroid: bool,
) -> Result<Vec<CellIndex>, Status> {
    let mut cells = h3o::geom::Geometry::from_degrees(geom.clone())
        .to_status_result()?
        .to_cells(h3_resolution)
        .collect::<Vec<_>>();

    if include_centroid {
        // add centroid in case of small geometries
        if let Some(ll) = geom.centroid().and_then(|pt| LatLng::try_from(pt.0).ok()) {
            cells.push(ll.to_cell(h3_resolution));
            // TODO: port intersecting cells
        }
    }

    // remove duplicates in case of multi* geometries
    cells.sort_unstable();
    cells.dedup();
    Ok(cells)
}

/// buffer a geometry in meters
///
/// This function creates some distortion as the geometry is transformed
/// between WGS84 and Spherical Mercator
pub fn buffer_meters(geom: &Geometry, meters: f64) -> Result<Geometry, Status> {
    crate::geo::buffer(geom, Length::new::<meter>(meters)).map_err(|e| {
        logged_status!(
            "geometry buffering failed",
            Code::Internal,
            Level::ERROR,
            &e
        )
    })
}

/// convert a geotypes `Geometry` to WKB using GDAL
pub fn to_wkb(geom: &Geometry) -> Result<Vec<u8>, Status> {
    crate::geo::wkb::to_wkb(geom).map_err(|e| {
        logged_status!(
            "Unable to convert geometry to WKB",
            Code::Internal,
            Level::ERROR,
            &e
        )
    })
}
