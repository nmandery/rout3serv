//! vector geometry handling
//!
use std::convert::TryInto;

use crate::server::error::ToStatusResult;
use gdal::spatial_ref::SpatialRef;
use gdal::vector::{Geometry, ToGdal};
use geo::algorithm::centroid::Centroid;
use geo_types::Geometry as GTGeometry;
use h3ron::{H3Cell, ToH3Cells};
use tonic::{Code, Status};

/// read binary WKB into a gdal `Geometry`
pub fn read_wkb_to_gdal(wkb_bytes: &[u8]) -> Result<Geometry, Status> {
    Geometry::from_wkb(wkb_bytes)
        .to_status_result_with_message(Code::InvalidArgument, || "Can not parse WKB".to_string())
}

/// convert a gdal `Geometry` to `H3Cell`s.
pub fn gdal_geom_to_h3(
    geom: &Geometry,
    h3_resolution: u8,
    include_centroid: bool,
) -> Result<Vec<H3Cell>, Status> {
    let gt_geom: GTGeometry<f64> = geom.clone().try_into().to_status_result()?;
    let mut cells = gt_geom
        .to_h3_cells(h3_resolution)
        .to_status_result()?
        .iter()
        .collect::<Vec<_>>();

    if include_centroid {
        // add centroid in case of small geometries
        if let Some(point) = gt_geom.centroid() {
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
    buffer_meters_internal(geom, meters)
        .to_status_result_with_message(Code::Internal, || "geometry buffering failed".to_string())
}

/// mainly used for debugging
#[allow(dead_code)]
pub fn to_geojson(geom: GTGeometry<f64>) -> gdal::errors::Result<String> {
    geom.to_gdal()?.json()
}

fn buffer_meters_internal(geom: &Geometry, meters: f64) -> gdal::errors::Result<Geometry> {
    let srs_wgs84 = SpatialRef::from_epsg(4326)?;
    let srs_spherical_mercator = SpatialRef::from_epsg(3857)?;
    let mut geom_sm_buffered = {
        let mut geom_cloned = geom.clone();
        geom_cloned.set_spatial_ref(srs_wgs84.clone());
        geom_cloned
            .transform_to(&srs_spherical_mercator)?
            .buffer(meters, 4)?
    };
    geom_sm_buffered.set_spatial_ref(srs_spherical_mercator);
    geom_sm_buffered.transform_to(&srs_wgs84)
}

/// convert a geotypes `Geometry` to WKB using GDAL
pub fn to_wkb(geom: &GTGeometry<f64>) -> Result<Vec<u8>, Status> {
    to_wkb_internal(geom)
        .to_status_result_with_message(Code::Internal, || "can not encode WKB".to_string())
}

#[inline]
fn to_wkb_internal(geom: &GTGeometry<f64>) -> gdal::errors::Result<Vec<u8>> {
    geom.to_gdal()?.wkb()
}
