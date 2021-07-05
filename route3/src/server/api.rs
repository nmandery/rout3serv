use std::collections::HashSet;

use tonic::{include_proto, Status};

use route3_core::gdal_util::buffer_meters;
use route3_core::geo_types::Coordinate;
use route3_core::h3ron::H3Cell;

use crate::server::util::{gdal_geom_to_h3, read_wkb_to_gdal};

include_proto!("grpc.route3");

pub struct RadiusDisturbanceCells {
    /// the cells within the disturbance
    pub disturbance: HashSet<H3Cell>,

    /// the cells of the disturbance and within the surrounding buffer
    pub within_buffer: Vec<H3Cell>,

    /// the target cells to route to
    pub targets: Vec<H3Cell>,
}

impl AnalyzeDisturbanceRequest {
    pub fn requested_cells(
        &self,
        h3_resolution: u8,
    ) -> std::result::Result<RadiusDisturbanceCells, Status> {
        let (disturbance, within_buffer) = self.disturbance_and_buffered_cells(h3_resolution)?;
        Ok(RadiusDisturbanceCells {
            disturbance,
            within_buffer,
            targets: self.target_cells(h3_resolution)?,
        })
    }

    fn disturbance_and_buffered_cells(
        &self,
        h3_resolution: u8,
    ) -> std::result::Result<(HashSet<H3Cell>, Vec<H3Cell>), Status> {
        let disturbance_geom = read_wkb_to_gdal(&self.wkb_geometry)?;
        let disturbed_cells: HashSet<_> = gdal_geom_to_h3(&disturbance_geom, h3_resolution, true)?
            .drain(..)
            .collect();

        let buffered_cells: Vec<_> = gdal_geom_to_h3(
            &buffer_meters(&disturbance_geom, self.radius_meters).map_err(|e| {
                log::error!("Buffering disturbance geom failed: {:?}", e);
                Status::internal("buffer failed")
            })?,
            h3_resolution,
            true,
        )?;
        Ok((disturbed_cells, buffered_cells))
    }

    /// cells to route to
    fn target_cells(&self, h3_resolution: u8) -> std::result::Result<Vec<H3Cell>, Status> {
        let target_cells = self
            .target_points
            .iter()
            .map(|pt| H3Cell::from_coordinate(&Coordinate::from((pt.x, pt.y)), h3_resolution))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                log::error!("can not convert the target_points to h3: {}", e);
                Status::internal("can not convert the target_points to h3")
            })?;
        Ok(target_cells)
    }
}
