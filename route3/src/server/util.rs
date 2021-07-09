/// utility functions to use within the server context, most of them
/// return a `tonic::Status` on error.
use tonic::Status;

use crate::io::recordbatch_to_bytes;
use arrow::record_batch::RecordBatch;
use geo::algorithm::centroid::Centroid;
use route3_core::gdal::vector::Geometry;
use route3_core::geo_types::Geometry as GTGeometry;
use route3_core::h3ron::{H3Cell, ToH3Indexes};

/// read binary WKB into a gdal `Geometry`
pub fn read_wkb_to_gdal(wkb_bytes: &[u8]) -> std::result::Result<Geometry, Status> {
    Geometry::from_wkb(wkb_bytes).map_err(|_e| Status::invalid_argument("Can not parse WKB"))
}

/// convert a gdal `Geometry` to `H3Cell`s.
pub fn gdal_geom_to_h3(
    geom: &Geometry,
    h3_resolution: u8,
    include_centroid: bool,
) -> std::result::Result<Vec<H3Cell>, Status> {
    let gt_geom: GTGeometry<f64> = geom.clone().into(); // TODO: gdal crate 0.7 will panic on converting unsupported geometry types
    let mut cells = gt_geom.to_h3_indexes(h3_resolution).map_err(|e| {
        log::error!("could not convert to h3: {:?}", e);
        Status::internal("could not convert to h3")
    })?;

    if include_centroid {
        // add centroid in case of small geometries
        if let Some(point) = gt_geom.centroid() {
            if let Ok(cell) = H3Cell::from_coordinate(&point.0, h3_resolution) {
                cells.push(cell);
            }
        }
    }

    // remove duplicates in case of multi* geometries
    cells.sort_unstable();
    cells.dedup();
    Ok(cells)
}

/// wrapper around tokios `spawn_blocking` to directly
/// return the `JoinHandle` as a tonic `Status`.
pub async fn spawn_blocking_status<F, R>(f: F) -> Result<R, Status>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(f).await.map_err(|e| {
        log::error!("joining blocking task failed: {:?}", e);
        Status::internal("join error")
    })
}

pub fn recordbatch_to_bytes_status(recordbatch: &RecordBatch) -> Result<Vec<u8>, Status> {
    let recordbatch_bytes = recordbatch_to_bytes(&recordbatch).map_err(|e| {
        log::error!("serializing recordbatch failed: {:?}", e);
        Status::internal("serializing recordbatch failed")
    })?;
    Ok(recordbatch_bytes)
}
