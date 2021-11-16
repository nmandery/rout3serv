use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{AddExtensionLayer, Json, Router};
use eyre::Result;
use geo::bounding_rect::BoundingRect;
use h3io::dataframe::H3DataFrame;
use h3io::s3::{S3Client, S3RecordBatchLoader};
use h3ron::{H3Cell, Index};
use polars_core::prelude::{DataFrame, NamedFrom, Series};
use tokio::task::spawn_blocking;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;

use crate::config::{ServerConfig, TileDataset};
use crate::response::{Msg, OutDataFrame, OutputFormat};
use crate::tile::{CellBuilder, Tile};

async fn serve_tile(
    Path((dataset_name, z, x, y)): Path<(String, u16, u32, u32)>,
    registry_state: Extension<Arc<Registry>>,
) -> Result<OutDataFrame, StatusCode> {
    let tile = Tile { x, y, z };
    build_tile(tile, dataset_name, OutputFormat::ArrowIPC, registry_state.0).await
}

async fn serve_tile_with_format(
    Path((dataset_name, z, x, y, format)): Path<(String, u16, u32, u32, String)>,
    registry_state: Extension<Arc<Registry>>,
) -> Result<OutDataFrame, StatusCode> {
    let tile = Tile { x, y, z };
    let output_format = OutputFormat::from_name(&format)?;
    build_tile(tile, dataset_name, output_format, registry_state.0).await
}

async fn build_tile(
    tile: Tile,
    dataset_name: String,
    output_format: OutputFormat,
    registry: Arc<Registry>,
) -> Result<OutDataFrame, StatusCode> {
    log::debug!("received request for {} of dataset {}", tile, dataset_name);

    let wrapped_tds = match registry.datasets.get(&dataset_name) {
        Some(wrapped_tds) => wrapped_tds,
        None => return Err(StatusCode::NOT_FOUND),
    };

    if let Some((h3_resolution, cells)) = wrapped_tds
        .cell_builder
        .cells_bounded(&tile, 10000)
        .map_err(|e| {
            log::error!(
                "no suitable cells for {} of {} : {:?}",
                tile,
                dataset_name,
                e
            );
            StatusCode::NO_CONTENT
        })?
    {
        let cell_vec: Vec<_> = cells.iter().collect();
        log::debug!(
            "using h3_resolution {} for {} of {} (len: {})",
            h3_resolution,
            tile,
            dataset_name,
            cell_vec.len()
        );

        let loaded_dataframe = registry
            .loader
            .load_h3_dataset_dataframe(&wrapped_tds.tile_dataset, &cell_vec, h3_resolution)
            .await
            .map_err(|e| {
                log::error!("fetching dataframe from upstream failed: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let response_dataframe =
            spawn_blocking(move || condense_response_dataframe(loaded_dataframe, &cell_vec))
                .await
                .map_err(|join_err| {
                    log::error!("joining condensing task failed: {:?}", join_err);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
                .map_err(|e| {
                    log::error!("condensing dataframe to selectin failed: {:?}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

        let outdf = OutDataFrame {
            output_format,
            h3_resolution,
            dataframe: response_dataframe,
        };

        Ok(outdf)
    } else {
        Err(StatusCode::NO_CONTENT)
    }
}

/// reduce the loaded dateframe the the requested subset
fn condense_response_dataframe(
    loaded_dataframe: H3DataFrame,
    selected_cells: &[H3Cell],
) -> eyre::Result<DataFrame> {
    let selection_df = DataFrame::new(vec![Series::new(
        "h3index",
        selected_cells
            .iter()
            .map(|c| c.h3index() as u64)
            .collect::<Vec<_>>()
            .as_slice(),
    )])?;
    Ok(selection_df
        .inner_join(
            &loaded_dataframe.dataframe,
            "h3index",
            loaded_dataframe.h3index_column_name.as_str(),
        )?
        // sort dataframe for better compression
        .sort("h3index", false)?)
}

async fn dataset_metadata(Path(dataset): Path<String>) -> Result<String, (StatusCode, Json<Msg>)> {
    Err((StatusCode::INTERNAL_SERVER_ERROR, Default::default()))
}

async fn list_datasets(
    registry_state: Extension<Arc<Registry>>,
) -> (StatusCode, Json<Vec<String>>) {
    let datasets: Vec<String> = registry_state.datasets.keys().cloned().collect();
    (StatusCode::OK, Json::from(datasets))
}

pub async fn run_server(server_config: ServerConfig) -> Result<()> {
    let addr = server_config.bind_to.parse()?;
    log::info!("{} is listening on {}", env!("CARGO_PKG_NAME"), addr);

    let registry: Arc<Registry> = Arc::new(server_config.try_into()?);

    // build our application
    let app = Router::new()
        .route("/tiles", get(list_datasets))
        .route("/tiles/:dataset_name", get(dataset_metadata))
        .route("/tiles/:dataset_name/:z/:x/:y", get(serve_tile))
        .route(
            "/tiles/:dataset_name/:z/:x/:y/:format",
            get(serve_tile_with_format),
        )
        .layer(TraceLayer::new_for_http())
        .layer(AddExtensionLayer::new(registry))
        .layer(CompressionLayer::new());

    // run it with hyper
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

struct WrappedTileDataset {
    tile_dataset: TileDataset,
    cell_builder: CellBuilder,
}

impl From<TileDataset> for WrappedTileDataset {
    fn from(tds: TileDataset) -> Self {
        let cell_builder = CellBuilder::new(tds.resolutions.keys());
        Self {
            tile_dataset: tds,
            cell_builder,
        }
    }
}

struct Registry {
    pub datasets: HashMap<String, WrappedTileDataset>,
    pub loader: S3RecordBatchLoader,
}

impl TryFrom<ServerConfig> for Registry {
    type Error = eyre::Error;

    fn try_from(mut server_config: ServerConfig) -> Result<Self, Self::Error> {
        let s3_client: Arc<S3Client> = Arc::new(S3Client::from_config(&server_config.s3)?);
        let reg = Self {
            datasets: server_config
                .datasets
                .drain()
                .map(|(name, tds)| (name, tds.into()))
                .collect(),
            loader: S3RecordBatchLoader::new(
                s3_client,
                server_config.cache_capacity.unwrap_or(120),
            ),
        };
        Ok(reg)
    }
}
