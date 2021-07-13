use tonic::Status;

use route3_core::gdal_util::buffer_meters;
use route3_core::geo_types::Coordinate;
use route3_core::h3ron::{H3Cell, Index};

use crate::server::api::route3::{DisturbanceOfPopulationMovementRequest, RouteWkb};
use crate::server::population_movement;
use crate::server::util::{gdal_geom_to_h3, read_wkb_to_gdal};
use crate::types::Weight;
use route3_core::routing::Route;
use route3_core::H3CellSet;

pub mod route3; // autogenerated by tonic build (see build.rs)

impl DisturbanceOfPopulationMovementRequest {
    pub fn get_input(
        &self,
        h3_resolution: u8,
    ) -> std::result::Result<population_movement::Input, Status> {
        let (disturbance, within_buffer) = self.disturbance_and_buffered_cells(h3_resolution)?;
        Ok(population_movement::Input {
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
    ) -> std::result::Result<(H3CellSet, Vec<H3Cell>), Status> {
        let disturbance_geom = read_wkb_to_gdal(&self.disturbance_wkb_geometry)?;
        let disturbed_cells: H3CellSet = gdal_geom_to_h3(&disturbance_geom, h3_resolution, true)?
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
        let mut destination_cells = self
            .destinations
            .iter()
            .map(|pt| H3Cell::from_coordinate(&Coordinate::from((pt.x, pt.y)), h3_resolution))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                log::error!("can not convert the target_points to h3: {:?}", e);
                Status::internal("can not convert the target_points to h3")
            })?;
        destination_cells.sort_unstable();
        destination_cells.dedup();
        Ok(destination_cells)
    }
}

impl RouteWkb {
    pub fn from_route(route: &Route<Weight>) -> Result<Self, Status> {
        let wkb_bytes = wkb::geom_to_wkb(&route.geometry()).map_err(|e| {
            log::error!("can not encode route to wkb: {:?}", e);
            Status::internal("can not encode wkb")
        })?;
        Ok(Self {
            origin_cell: route.origin_cell().map(|c| c.h3index()).unwrap_or(0),
            destination_cell: route.destination_cell().map(|c| c.h3index()).unwrap_or(0),
            cost: f64::from(route.cost),
            wkb: wkb_bytes,
        })
    }
}
