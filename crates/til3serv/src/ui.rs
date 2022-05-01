use crate::build_info::{app_name, version};
use crate::config::UiBaseLayer;
use crate::state::Registry;
use axum::extract::{Extension, Path};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use minijinja::filters::{safe, tojson};
use minijinja::Environment;
use once_cell::sync::Lazy;
use s3io::s3::S3H3Dataset;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// geojson string with a feature collection of country boundaries.
/// Strongly simplified shapes.
const COUNTRIES_GEOJSON: &[u8] = include_bytes!("../data/countries.geojson");

/// js bundle of the view
const VIEWER_JS: &[u8] = include_bytes!("../dist/viewer.js");

static MJ_ENV: Lazy<Environment<'static>> = Lazy::new(|| {
    let mut env = Environment::new();
    env.add_template("base.html", include_str!("../templates/base.html"))
        .unwrap();
    env.add_template("viewer.html", include_str!("../templates/viewer.html"))
        .unwrap();
    env.add_template("main.html", include_str!("../templates/main.html"))
        .unwrap();
    env.add_filter("tojson", tojson);
    env.add_filter("safe", safe);
    env
});

fn render_template<S: Serialize>(template_name: &str, context: &S) -> eyre::Result<String> {
    let template = MJ_ENV.get_template(template_name)?;
    Ok(template.render(context)?)
}

fn respond_html_template<S: Serialize>(
    template_name: &str,
    context: &S,
) -> Result<(HeaderMap, String), StatusCode> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));
    let html = render_template(template_name, context).map_err(|e| {
        log::error!("creating template {} failed: {:?}", template_name, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok((headers, html))
}

pub async fn ui_static_files(
    Path(filename): Path<String>,
) -> Result<(HeaderMap, &'static [u8]), StatusCode> {
    match filename.as_str() {
        "countries.geojson" => {
            let mut headers = HeaderMap::new();
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            Ok((headers, crate::ui::COUNTRIES_GEOJSON))
        }
        "viewer.js" => {
            let mut headers = HeaderMap::new();
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/javascript"),
            );
            Ok((headers, crate::ui::VIEWER_JS))
        }
        _ => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ViewerStyleConfig {
    /// property used for styling
    #[serde(rename(serialize = "propertyName"))]
    pub property_name: String,

    /// the value range to apply the colors to
    #[serde(rename(serialize = "valueRange"))]
    pub value_range: Vec<f32>,

    /// the colors for the range. can be anything d3 understands
    #[serde(rename(serialize = "colorRange"))]
    pub color_range: Vec<String>,
}

#[derive(Serialize)]
struct ViewerConfig<'a> {
    /// root of the applications routing, or relative path to that
    /// location
    #[serde(rename(serialize = "baseUrl"))]
    pub base_url: String,

    #[serde(rename(serialize = "datasetName"))]
    pub dataset_name: String,

    #[serde(rename(serialize = "h3indexPropertyName"))]
    pub h3index_property_name: String,

    #[serde(rename(serialize = "styleConfig"))]
    pub style_config: Option<&'a ViewerStyleConfig>,

    #[serde(rename(serialize = "baseLayer"))]
    pub base_layer: UiBaseLayer,
}

#[derive(Serialize)]
struct ViewerContext<'a> {
    pub app_name: &'static str,
    pub viewer_config: ViewerConfig<'a>,
}

pub async fn tile_viewer(
    Path(dataset_name): Path<String>,
    registry: Extension<Arc<Registry>>,
) -> Result<(HeaderMap, String), StatusCode> {
    let wrapped_tds = match registry.datasets.get(&dataset_name) {
        Some(wrapped_tds) => wrapped_tds,
        None => return Err(StatusCode::NOT_FOUND),
    };
    respond_html_template(
        "viewer.html",
        &ViewerContext {
            app_name: app_name(),
            viewer_config: ViewerConfig {
                base_url: "../../..".to_string(),
                dataset_name,
                h3index_property_name: wrapped_tds.tile_dataset.h3index_column(),
                style_config: wrapped_tds.tile_dataset.style.as_ref(),
                base_layer: registry.ui.base_layer.clone(),
            },
        },
    )
}

#[derive(Serialize)]
struct MainContext {
    pub version: &'static str,
    pub dataset_names: Vec<String>,
    pub app_name: &'static str,
}

pub async fn main_page(
    registry: Extension<Arc<Registry>>,
) -> Result<(HeaderMap, String), StatusCode> {
    respond_html_template(
        "main.html",
        &MainContext {
            version: version(),
            dataset_names: registry.datasets.keys().cloned().collect(),
            app_name: app_name(),
        },
    )
}
