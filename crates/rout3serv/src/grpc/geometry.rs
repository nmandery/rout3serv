//! vector geometry handling
//!
use geo::algorithm::centroid::Centroid;
use geo_types::Geometry;
use h3ron::{H3Cell, ToH3Cells};
use tonic::{Code, Status};
use tracing::log::Level;
use uom::si::f64::Length;
use uom::si::length::meter;

use crate::grpc::error::{logged_status_with_cause, ToStatusResult};

/// read binary WKB into a gdal `Geometry`
pub fn from_wkb(wkb_bytes: &[u8]) -> Result<Geometry, Status> {
    crate::geo::wkb::from_wkb(wkb_bytes).map_err(|e| {
        logged_status_with_cause("Can not parse WKB", Code::InvalidArgument, Level::Warn, &e)
    })
}

/// convert a`Geometry` to `H3Cell`s.
pub fn geom_to_h3(
    geom: &Geometry,
    h3_resolution: u8,
    include_centroid: bool,
) -> Result<Vec<H3Cell>, Status> {
    let mut cells = geom
        .to_h3_cells(h3_resolution)
        .to_status_result()?
        .iter()
        .collect::<Vec<_>>();

    if include_centroid {
        // add centroid in case of small geometries
        if let Some(point) = geom.centroid() {
            if let Ok(cell) = H3Cell::from_coordinate(point.0, h3_resolution) {
                cells.push(cell);
            }
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
        logged_status_with_cause(
            "geometry buffering failed",
            Code::Internal,
            Level::Error,
            &e,
        )
    })
}

/// convert a geotypes `Geometry` to WKB using GDAL
pub fn to_wkb(geom: &Geometry) -> Result<Vec<u8>, Status> {
    crate::geo::wkb::to_wkb(geom).map_err(|e| {
        logged_status_with_cause(
            "Unable to convert geometry to WKB",
            Code::Internal,
            Level::Error,
            &e,
        )
    })
}
