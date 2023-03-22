#![allow(clippy::derive_partial_eq_without_eq)] // for the generated code. https://github.com/tokio-rs/prost/issues/661

use std::convert::TryFrom;

use geo::chaikin_smoothing::ChaikinSmoothing;
use geo::simplify::Simplify;
use geo_types::Geometry;
use h3o::Resolution;
use hexigraph::algorithm::graph::path::Path;
use hexigraph::algorithm::graph::shortest_path;
use tonic::{Code, Status};
use tracing::Level;
use uom::si::time::second;

use crate::grpc::api::generated::{GraphHandle, RouteH3Indexes, RouteWkb, ShortestPathOptions};
use crate::grpc::error::{logged_status, ToStatusResult};
use crate::grpc::geometry::to_wkb;
use crate::io::GraphKey;
use crate::weight::Weight;

pub mod generated; // autogenerated by tonic build (see build.rs)

pub trait Route {}

impl Route for RouteWkb {}

impl Route for RouteH3Indexes {}

const SIMPLIFICATION_EPSILON: f64 = 0.00001;

impl RouteWkb {
    pub fn from_path<T>(path: &Path<T>, smoothen: bool) -> Result<Self, Status>
    where
        T: Weight,
    {
        let mut linestring = path
            .directed_edge_path
            .to_linestring()
            .to_status_result_with_message(Code::Internal, || {
                "can not build linestring from path".to_string()
            })?;

        if smoothen {
            // apply only one iteration to break edges
            linestring = linestring.chaikin_smoothing(1);
        }

        // remove redundant vertices. This reduces the amount of data to transfer
        // without losing any significant information
        linestring = linestring.simplify(&SIMPLIFICATION_EPSILON);

        let wkb_bytes = to_wkb(&Geometry::LineString(linestring))?;
        Ok(Self {
            origin_cell: u64::from(path.origin_cell),
            destination_cell: u64::from(path.destination_cell),
            travel_duration_secs: path.cost.travel_duration().get::<second>() as f64,
            edge_preference: path.cost.edge_preference() as f64,
            wkb: wkb_bytes,
            path_length_m: path.directed_edge_path.length_m(),
        })
    }
}

#[derive(Clone, Debug, Copy)]
pub enum RouteH3IndexesKind {
    Cells,
    Edges,
}

impl RouteH3Indexes {
    pub fn from_path<T>(path: &Path<T>, kind: RouteH3IndexesKind) -> Result<Self, Status>
    where
        T: Weight,
    {
        let h3indexes = match kind {
            RouteH3IndexesKind::Cells => path
                .directed_edge_path
                .cells()
                .into_iter()
                .map(u64::from)
                .collect(),
            RouteH3IndexesKind::Edges => path
                .directed_edge_path
                .edges()
                .iter()
                .copied()
                .map(u64::from)
                .collect(),
        };
        Ok(Self {
            origin_cell: u64::from(path.origin_cell),
            destination_cell: u64::from(path.destination_cell),
            travel_duration_secs: path.cost.travel_duration().get::<second>() as f64,
            edge_preference: path.cost.edge_preference() as f64,
            h3indexes,
            path_length_m: path.directed_edge_path.length_m(),
        })
    }
}

impl From<GraphKey> for GraphHandle {
    fn from(graph_key: GraphKey) -> Self {
        Self {
            name: graph_key.name,
            h3_resolution: graph_key.h3_resolution as u32,
        }
    }
}

impl TryFrom<&GraphHandle> for GraphKey {
    type Error = Status;

    fn try_from(gh: &GraphHandle) -> Result<Self, Self::Error> {
        if gh.name.is_empty() {
            return Err(logged_status!(
                "empty graph name",
                Code::InvalidArgument,
                Level::INFO
            ));
        }
        let h3_resolution = Resolution::try_from(gh.h3_resolution as u8).map_err(|_| {
            logged_status!(
                "invalid h3 resolution in graph handle",
                Code::InvalidArgument,
                Level::INFO
            )
        })?;
        Ok(Self {
            name: gh.name.clone(),
            h3_resolution,
        })
    }
}

impl TryFrom<&Option<GraphHandle>> for GraphKey {
    type Error = Status;

    fn try_from(gh: &Option<GraphHandle>) -> Result<Self, Self::Error> {
        if let Some(gh) = gh {
            gh.try_into()
        } else {
            Err(logged_status!(
                "graph handle not set",
                Code::InvalidArgument,
                Level::INFO
            ))
        }
    }
}

impl shortest_path::ShortestPathOptions for ShortestPathOptions {
    fn max_distance_to_graph(&self) -> u32 {
        self.num_gap_cells_to_graph
    }

    fn num_destinations_to_reach(&self) -> Option<usize> {
        if self.num_destinations_to_reach == 0 {
            // 0 means nothing has been set
            None
        } else {
            Some(self.num_destinations_to_reach as usize)
        }
    }
}
