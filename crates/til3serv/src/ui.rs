use crate::build_info::{app_name, version};
use crate::state::Registry;
use axum::extract::{Extension, Path};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use minijinja::filters::{safe, tojson};
use minijinja::Environment;
use serde::Serialize;
use std::sync::Arc;

/// geojson string with a feature collection of country boundaries.
/// Strongly simplified shapes.
const COUNTRIES_GEOJSON: &[u8] = include_bytes!("../ui/data/countries.geojson");

/// js bundle of the view
const VIEWER_JS: &[u8] = include_bytes!("../ui/dist/viewer.js");

lazy_static::lazy_static! {
    static ref MJ_ENV: Environment<'static> = {
        let mut env = Environment::new();
        env.add_template("base.html", include_str!("../ui/templates/base.html")).unwrap();
        env.add_template("viewer.html", include_str!("../ui/templates/viewer.html")).unwrap();
        env.add_template("main.html", include_str!("../ui/templates/main.html")).unwrap();
        env.add_filter("tojson", tojson);
        env.add_filter("safe", safe);
        env
    };
}

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

#[derive(Serialize)]
struct ViewerContext {
    /// root of the applications routing, or relative path to that
    /// location
    pub base_url: String,

    pub dataset_name: String,

    pub app_name: &'static str,
}

pub async fn tile_viewer(
    Path(dataset_name): Path<String>,
    registry: Extension<Arc<Registry>>,
) -> Result<(HeaderMap, String), StatusCode> {
    let _wrapped_tds = match registry.datasets.get(&dataset_name) {
        Some(wrapped_tds) => wrapped_tds,
        None => return Err(StatusCode::NOT_FOUND),
    };
    respond_html_template(
        "viewer.html",
        &ViewerContext {
            base_url: "../../..".to_string(),
            dataset_name,
            app_name: app_name(),
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
