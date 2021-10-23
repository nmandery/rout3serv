use std::convert::TryFrom;

use eyre::Report;
use geo_types::Geometry;
use h3ron::Index;
use h3ron_graph::algorithm::path::Path;
use h3ron_graph::algorithm::shortest_path;
use tonic::Status;
use uom::si::time::second;

use crate::io::graph_store::GraphCacheKey;
use crate::server::api::generated::{GraphHandle, GraphInfo, RouteWkb, ShortestPathOptions};
use crate::server::vector::to_wkb;
use crate::weight::Weight;

pub mod generated; // autogenerated by tonic build (see build.rs)

impl RouteWkb {
    pub fn from_path<T>(path: &Path<T>) -> Result<Self, Status>
    where
        T: Weight,
    {
        let wkb_bytes = to_wkb(&Geometry::LineString(path.to_linestring().map_err(
            |e| {
                log::error!("can not build linestring from path: {:?}", e);
                Status::internal("can not build linestring from path")
            },
        )?))?;
        Ok(Self {
            origin_cell: path.origin_cell().map(|c| c.h3index()).unwrap_or(0),
            destination_cell: path.destination_cell().map(|c| c.h3index()).unwrap_or(0),
            travel_duration_secs: path.cost.travel_duration().get::<second>() as f64,
            category_weight: path.cost.category_weight() as f64,
            wkb: wkb_bytes,
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

    fn try_from(gh: &GraphHandle) -> std::result::Result<Self, Self::Error> {
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
