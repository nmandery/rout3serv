use std::borrow::Borrow;
use std::cmp::max;
use std::sync::Arc;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::algo::path::Path;
use crate::algo::shortest_path::{ManyToManyOptions, ShortestPath};
use crate::collections::H3CellSet;
use crate::error::Error;
use crate::h3ron::iter::change_cell_resolution;
use crate::h3ron::{H3Cell, H3Edge, Index};
use crate::WithH3Resolution;

#[derive(Serialize, Deserialize)]
pub struct DifferentialShortestPath<O> {
    pub origin_cell: H3Cell,
    pub without_disturbance: Vec<O>,
    pub with_disturbance: Vec<O>,
}

/// differential routing calculates the shortest path from (multiple) origin cells
/// to the `N` nearest destinations.
/// This done once to the un-modified graph, and once the the graph with a set of nodes
/// being removed - the `exclude_cells` of the given options.
///
/// Setting a `downsampled_graph` will allow performing an initial routing at a lower resolution
/// to reduce the number of routings to perform on the full-resolution graph by concentrating on the
/// origin cells which are affected by the `exclude_cells`. This has the potential
/// to skew the results as a reduction in resolution may change the graph topology, but decreases the
/// running time in most cases.
/// The reduction should be no more than two resolutions.
#[inline]
pub fn differential_shortest_path<G, T, I>(
    graph: Arc<G>,
    origin_cells: I,
    destination_cells: I,
    downsampled_graph: Option<Arc<G>>,
    many_to_many_options: ManyToManyOptions,
) -> Result<Vec<DifferentialShortestPath<Path<T>>>, Error>
where
    T: PartialEq + Ord + Send + Copy + Sync,
    I: IntoIterator,
    I::Item: Borrow<H3Cell>,
    G: ShortestPath<T> + WithH3Resolution,
{
    differential_shortest_path_map(
        graph,
        origin_cells,
        destination_cells,
        downsampled_graph,
        many_to_many_options,
        |path| path,
    )
}

pub fn differential_shortest_path_map<G, T, I, F, O>(
    graph: Arc<G>,
    origin_cells: I,
    destination_cells: I,
    downsampled_graph: Option<Arc<G>>,
    many_to_many_options: ManyToManyOptions,
    path_map_fn: F,
) -> Result<Vec<DifferentialShortestPath<O>>, Error>
where
    T: PartialEq + Ord + Send + Copy + Sync,
    I: IntoIterator,
    I::Item: Borrow<H3Cell>,
    G: ShortestPath<T> + WithH3Resolution,
    F: Fn(Path<T>) -> O + Send + Sync + Clone,
    O: Send + Ord + Clone + Sync,
{
    let exclude_cells = if let Some(ex) = many_to_many_options.exclude_cells {
        ex
    } else {
        return Err(Error::ParameterError(
            "exclude_cells must not be none".to_string(),
        ));
    };
    let origin_cells = check_resolution_and_collect(
        origin_cells.into_iter().filter(|c| {
            // exclude the cells of the disturbance itself from routing
            !exclude_cells.contains(c.borrow())
        }),
        graph.h3_resolution(),
    )?;
    let destination_cells = check_resolution_and_collect(destination_cells, graph.h3_resolution())?;

    let selected_origin_cells: Vec<H3Cell> = {
        if let Some(ds_graph) = downsampled_graph {
            if ds_graph.h3_resolution() >= graph.h3_resolution() {
                return Err(Error::TooHighH3Resolution(ds_graph.h3_resolution()));
            }

            // perform a routing at a reduced resolution to get a reduced subset for the origin cells at the
            // full resolution without most unaffected cells. This will reduce the number of full resolution
            // routings to be performed later.
            // This overestimates the number of affected cells a bit due to the reduced resolution.
            //
            // Gap bridging is set to 0 as this is already accomplished by the reduction in resolution.
            let mut downsampled_origins: Vec<_> =
                change_cell_resolution(&origin_cells, ds_graph.h3_resolution()).collect();
            downsampled_origins.sort_unstable();
            downsampled_origins.dedup();

            let mut downsampled_destinations: Vec<_> =
                change_cell_resolution(&destination_cells, ds_graph.h3_resolution()).collect();
            downsampled_destinations.sort_unstable();
            downsampled_destinations.dedup();

            let without_disturbance = ds_graph.shortest_path_many_to_many_map(
                &downsampled_origins,
                &downsampled_destinations,
                &ManyToManyOptions {
                    num_destinations_to_reach: many_to_many_options.num_destinations_to_reach,
                    num_gap_cells_to_graph: 0,
                    ..Default::default()
                },
                path_map_fn.clone(),
            )?;
            let exclude_cells_downsampled: H3CellSet =
                change_cell_resolution(&exclude_cells, ds_graph.h3_resolution()).collect();
            let with_disturbance = ds_graph.shortest_path_many_to_many_map(
                &downsampled_origins,
                &downsampled_destinations,
                &ManyToManyOptions {
                    num_destinations_to_reach: many_to_many_options.num_destinations_to_reach,
                    exclude_cells: Some(exclude_cells_downsampled.clone()),
                    num_gap_cells_to_graph: 0,
                },
                path_map_fn.clone(),
            )?;

            // determinate the size of the k-ring to use to include enough full-resolution
            // cells around the found disturbance effect. This is essentially a buffering.
            let k_affected = max(
                1,
                (1500.0 / H3Edge::edge_length_m(ds_graph.h3_resolution())).ceil() as u32,
            );
            let affected_downsampled: H3CellSet = without_disturbance
                .par_keys()
                .filter(|cell| {
                    // the k_ring creates essentially a buffer so the skew-effects of the
                    // reduction of the resolution at the borders of the disturbance effect
                    // are reduced. The result is a larger number of full-resolution routing runs
                    // is performed.
                    !cell.k_ring(k_affected).iter().all(|ring_cell| {
                        with_disturbance.get(ring_cell) == without_disturbance.get(ring_cell)
                    })
                })
                .copied()
                .collect();

            origin_cells
                .iter()
                .filter(|cell| {
                    let parent_cell = cell.get_parent_unchecked(ds_graph.h3_resolution());
                    // always add cells within the downsampled disturbance to avoid ignoring cells directly
                    // bordering to the disturbance.
                    affected_downsampled.contains(&parent_cell)
                        || exclude_cells_downsampled.contains(&parent_cell)
                })
                .copied()
                .collect()
        } else {
            origin_cells
        }
    };

    let mut paths_without_disturbance = graph.shortest_path_many_to_many_map(
        &selected_origin_cells,
        &destination_cells,
        &ManyToManyOptions {
            num_destinations_to_reach: many_to_many_options.num_destinations_to_reach,
            num_gap_cells_to_graph: many_to_many_options.num_gap_cells_to_graph,
            ..Default::default()
        },
        path_map_fn.clone(),
    )?;

    let mut paths_with_disturbance = graph.shortest_path_many_to_many_map(
        &selected_origin_cells,
        &destination_cells,
        &ManyToManyOptions {
            num_destinations_to_reach: many_to_many_options.num_destinations_to_reach,
            exclude_cells: Some(exclude_cells),
            num_gap_cells_to_graph: many_to_many_options.num_gap_cells_to_graph,
        },
        path_map_fn,
    )?;

    Ok(paths_without_disturbance
        .drain()
        .map(|(cell, routes_without)| DifferentialShortestPath {
            origin_cell: cell,
            without_disturbance: routes_without,
            with_disturbance: paths_with_disturbance.remove(&cell).unwrap_or_default(),
        })
        .collect())
}

fn check_resolution_and_collect<I>(in_cells: I, h3_resolution: u8) -> Result<Vec<H3Cell>, Error>
where
    I: IntoIterator,
    I::Item: Borrow<H3Cell>,
{
    let mut out_cells = in_cells
        .into_iter()
        .map(|cell| {
            if cell.borrow().resolution() != h3_resolution {
                Err(Error::MixedH3Resolutions(
                    h3_resolution,
                    cell.borrow().resolution(),
                ))
            } else {
                Ok(*cell.borrow())
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    out_cells.sort_unstable();
    out_cells.dedup();
    Ok(out_cells)
}
