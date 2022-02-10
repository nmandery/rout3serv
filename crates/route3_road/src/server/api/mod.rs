use std::convert::TryFrom;

use eyre::Report;
use geo::chaikin_smoothing::ChaikinSmoothing;
use geo::simplify::Simplify;
use geo_types::Geometry;
use h3ron::{H3Cell, Index};
use h3ron_graph::algorithm::path::Path;
use h3ron_graph::algorithm::shortest_path;
use tonic::Status;
use uom::si::time::second;

use crate::io::graph_store::GraphCacheKey;
use crate::server::api::generated::{
    GraphHandle, GraphInfo, RouteH3Indexes, RouteWkb, ShortestPathOptions,
};
use crate::server::vector::to_wkb;
use crate::weight::Weight;

pub mod generated; // autogenerated by tonic build (see build.rs)

pub trait Route {}

impl Route for RouteWkb {}

impl Route for RouteH3Indexes {}

#[inline(always)]
fn cell_h3index(cell_result: Result<H3Cell, h3ron_graph::error::Error>) -> u64 {
    cell_result.map(|c| c.h3index()).unwrap_or(0)
}

const SIMPLIFICATION_EPSILON: f64 = 0.00001;

impl RouteWkb {
    pub fn from_path<T>(path: &Path<T>, smoothen: bool) -> Result<Self, Status>
    where
        T: Weight,
    {
        let mut linestring = path.to_linestring().map_err(|e| {
            log::error!("can not build linestring from path: {:?}", e);
            Status::internal("can not build linestring from path")
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
            origin_cell: cell_h3index(path.origin_cell()),
            destination_cell: cell_h3index(path.destination_cell()),
            travel_duration_secs: path.cost().travel_duration().get::<second>() as f64,
            edge_preference: path.cost().edge_preference() as f64,
            wkb: wkb_bytes,
            path_length_m: path.length_m(),
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
                .cells()
                .iter()
                .map(|cell| cell.h3index() as u64)
                .collect(),
            RouteH3IndexesKind::Edges => path
                .edges()
                .iter()
                .map(|edge| edge.h3index() as u64)
                .collect(),
        };
        Ok(Self {
            origin_cell: cell_h3index(path.origin_cell()),
            destination_cell: cell_h3index(path.destination_cell()),
            travel_duration_secs: path.cost().travel_duration().get::<second>() as f64,
            edge_preference: path.cost().edge_preference() as f64,
            h3indexes,
            path_length_m: path.length_m(),
        })
    }
}

impl From<GraphCacheKey> for GraphHandle {
    fn from(graph_cache_key: GraphCacheKey) -> Self {
        Self {
            name: graph_cache_key.name,
            h3_resolution: graph_cache_key.h3_resolution as u32,
        }
    }
}

impl From<GraphCacheKey> for GraphInfo {
    fn from(gck: GraphCacheKey) -> Self {
        Self {
            handle: Some(gck.into()),
            is_cached: false,
            num_edges: 0,
            num_nodes: 0,
        }
    }
}

impl TryFrom<&GraphHandle> for GraphCacheKey {
    type Error = Report;

    fn try_from(gh: &GraphHandle) -> Result<Self, Self::Error> {
        if gh.name.is_empty() {
            return Err(Report::msg("empty graph name"));
        }
        if gh.h3_resolution < h3ron::H3_MIN_RESOLUTION as u32
            || gh.h3_resolution > h3ron::H3_MAX_RESOLUTION as u32
        {
            return Err(Report::msg("invalid h3 resolution in graph handle"));
        }
        Ok(Self {
            name: gh.name.clone(),
            h3_resolution: gh.h3_resolution as u8,
        })
    }
}

impl shortest_path::ShortestPathOptions for ShortestPathOptions {
    fn num_gap_cells_to_graph(&self) -> u32 {
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
