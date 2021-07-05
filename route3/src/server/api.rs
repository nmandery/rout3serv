use std::collections::HashSet;

use tonic::{include_proto, Status};

use route3_core::gdal_util::buffer_meters;
use route3_core::geo_types::Coordinate;
use route3_core::h3ron::H3Cell;

use crate::server::util::{gdal_geom_to_h3, read_wkb_to_gdal};

include_proto!("grpc.route3");

pub struct AnalyzeDisturbanceInput {
    /// the cells within the disturbance
    pub disturbance: HashSet<H3Cell>,

    /// the cells of the disturbance and within the surrounding buffer
    pub within_buffer: Vec<H3Cell>,

    /// the destination cells to route to
    pub destinations: Vec<H3Cell>,

    pub num_destinations_to_reach: Option<usize>,
}

impl AnalyzeDisturbanceRequest {
    pub fn get_input(
        &self,
        h3_resolution: u8,
    ) -> std::result::Result<AnalyzeDisturbanceInput, Status> {
        let (disturbance, within_buffer) = self.disturbance_and_buffered_cells(h3_resolution)?;
        Ok(AnalyzeDisturbanceInput {
            disturbance,
            within_buffer,
            destinations: self.destination_cells(h3_resolution)?,
            num_destinations_to_reach: if self.num_destinations_to_reach == 0 {
                None
            } else {
                Some(self.num_destinations_to_reach as usize)
            },
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
    fn destination_cells(&self, h3_resolution: u8) -> std::result::Result<Vec<H3Cell>, Status> {
        let destination_cells = self
            .destinations
            .iter()
            .map(|pt| H3Cell::from_coordinate(&Coordinate::from((pt.x, pt.y)), h3_resolution))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                log::error!("can not convert the target_points to h3: {}", e);
                Status::internal("can not convert the target_points to h3")
            })?;
        Ok(destination_cells)
    }
}
