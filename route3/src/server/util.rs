/// utility functions to use within the server context, most of them
/// return a `tonic::Status` on error.
use tonic::Status;

use route3_core::gdal::vector::Geometry;
use route3_core::geo_types::Geometry as GTGeometry;
use route3_core::h3ron::{H3Cell, ToH3Indexes};

pub fn read_wkb_to_gdal(wkb_bytes: &[u8]) -> std::result::Result<Geometry, Status> {
    Geometry::from_wkb(wkb_bytes).map_err(|e| Status::invalid_argument("Can not parse WKB"))
}

pub fn gdal_geom_to_h3(
    geom: &Geometry,
    h3_resolution: u8,
) -> std::result::Result<Vec<H3Cell>, Status> {
    let gt_geom: GTGeometry<f64> = geom.clone().into();
    let mut cells = gt_geom.to_h3_indexes(h3_resolution).map_err(|e| {
        log::error!("could not convert to h3: {:?}", e);
        Status::internal("could not convert to h3")
    })?;

    // TODO: always add centroid in case of small geometries?

    // remove duplicates in case of multi* geometries
    cells.sort_unstable();
    cells.dedup();
    Ok(cells)
}
