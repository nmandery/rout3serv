use tonic::{include_proto, Status};

use route3_core::gdal_util::buffer_meters;
use route3_core::geo_types::Coordinate;
use route3_core::h3ron::{H3Cell, Index};
use serde::{Deserialize, Serialize};

use crate::constants::Weight;
use crate::io::recordbatch_to_bytes;
use crate::server::algo::DisturbanceOfPopulationMovementOutput;
use crate::server::util::{gdal_geom_to_h3, read_wkb_to_gdal};
use route3_core::routing::Route;
use route3_core::H3CellSet;

include_proto!("grpc.route3");

#[derive(Serialize, Deserialize)]
pub struct DisturbanceOfPopulationMovementInput {
    /// the cells within the disturbance
    pub disturbance: H3CellSet,

    /// the cells of the disturbance and within the surrounding buffer
    pub within_buffer: Vec<H3Cell>,

    /// the destination cells to route to
    pub destinations: Vec<H3Cell>,

    pub num_destinations_to_reach: Option<usize>,
}

impl DisturbanceOfPopulationMovementRequest {
    pub fn get_input(
        &self,
        h3_resolution: u8,
    ) -> std::result::Result<DisturbanceOfPopulationMovementInput, Status> {
        let (disturbance, within_buffer) = self.disturbance_and_buffered_cells(h3_resolution)?;
        Ok(DisturbanceOfPopulationMovementInput {
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
                log::error!("can not convert the target_points to h3: {}", e);
                Status::internal("can not convert the target_points to h3")
            })?;
        destination_cells.sort_unstable();
        destination_cells.dedup();
        Ok(destination_cells)
    }
}

impl DisturbanceOfPopulationMovementStats {
    pub fn from_output(
        output: &DisturbanceOfPopulationMovementOutput,
    ) -> std::result::Result<Self, Status> {
        let recordbatch = output.stats_recordbatch().map_err(|e| {
            log::error!("creating recordbatch failed: {:?}", e);
            Status::internal("creating recordbatch failed")
        })?;

        let recordbatch_bytes = recordbatch_to_bytes(&recordbatch).map_err(|e| {
            log::error!("serializing recordbatch failed: {:?}", e);
            Status::internal("serializing recordbatch failed")
        })?;

        Ok(Self {
            population_within_disturbance: output.population_within_disturbance,
            recordbatch: recordbatch_bytes,
        })
    }
}

impl RouteWkb {
    pub fn from_route(route: &Route<Weight>) -> Result<Self, Status> {
        let wkb_bytes = wkb::geom_to_wkb(&route.to_linestring().into()).map_err(|e| {
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
