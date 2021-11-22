use std::borrow::Borrow;
use std::f64::consts::{E, PI};
use std::fmt;

use geo::algorithm::bounding_rect::BoundingRect;
use geo::algorithm::dimensions::HasDimensions;
use geo_types::{Coordinate, Rect};
use h3ron::collections::indexvec::IndexVec;
use h3ron::{Error, H3Cell, H3Edge, ToH3Cells, H3_MAX_RESOLUTION};

/// parts of this file have been ported from
/// <https://github.com/mapbox/mercantile/blob/fe3762d14001ca400caf7462f59433b906fc25bd/mercantile/__init__.py#L200>
/// and
/// <https://github.com/openlayers/openlayers/blob/fdba3ecf0e47503dd8e8711a44cf34620be70b2d/src/ol/proj/epsg3857.js#L26>

#[derive(PartialEq, Debug)]
pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub z: u16,
}

impl fmt::Display for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tile({}, {}, {})", self.x, self.y, self.z)
    }
}

impl Tile {
    pub fn area_m2(&self) -> f64 {
        // TODO: look at the tiles and decide which area calculation to use
        //area_m2(&self.bounding_rect())
        let wbr = self.webmercator_bounding_rect();
        wbr.width() * wbr.height()
    }

    /// Get the web mercator bounding box of a tile
    pub fn webmercator_bounding_rect(&self) -> Rect<f64> {
        let tile_size = CE / 2.0f64.powi(self.z as i32);
        let left = (self.x as f64 * tile_size) - (CE / 2.0);
        let top = (CE / 2.0) - (self.y as f64 * tile_size);
        Rect::new(
            Coordinate::from((left, top)),
            Coordinate::from((left + tile_size, top - tile_size)),
        )
    }
}

#[inline(always)]
fn tile_coord_to_latlng(x: u32, y: u32, z2: f64) -> Coordinate<f64> {
    let lon_deg = x as f64 / z2 * 360.0 - 180.0;
    let lat_deg = (PI * (1.0 - 2.0 * (y as f64) / z2))
        .sinh()
        .atan()
        .to_degrees();

    Coordinate::from((lon_deg, lat_deg))
}

impl BoundingRect<f64> for Tile {
    type Output = Rect<f64>;

    /// bounding rect in wgs84 coordinates
    fn bounding_rect(&self) -> Self::Output {
        let z2 = 2.0_f64.powi(self.z as i32);
        Rect::new(
            tile_coord_to_latlng(self.x, self.y, z2),
            tile_coord_to_latlng(self.x + 1, self.y + 1, z2),
        )
    }
}

impl ToH3Cells for Tile {
    fn to_h3_cells(&self, h3_resolution: u8) -> Result<IndexVec<H3Cell>, Error> {
        // buffer the box using the cells size to ensure cells with just their
        // centroid outside of the box, but parts of the outline intersecting
        // are also included.
        // web-mercator uses meters as units.
        let buffered_bbox = {
            let buffer_meters = H3Edge::edge_length_m(h3_resolution) * 1.8;
            let wm_bbox = self.webmercator_bounding_rect();

            Rect::new(
                webmercator_to_lnglat(&truncate_to(
                    &Coordinate::from((
                        wm_bbox.min().x - buffer_meters,
                        wm_bbox.min().y - buffer_meters,
                    )),
                    &EXTEND_EPSG_3857,
                )),
                webmercator_to_lnglat(&truncate_to(
                    &Coordinate::from((
                        wm_bbox.max().x + buffer_meters,
                        wm_bbox.max().y + buffer_meters,
                    )),
                    &EXTEND_EPSG_3857,
                )),
            )
        };
        if buffered_bbox.is_empty() {
            Ok(Default::default())
        } else {
            buffered_bbox.to_h3_cells(h3_resolution)
        }
    }
}

const EARTH_RADIUS_EQUATOR: f64 = 6378137.0;
const R2D: f64 = 180.0 / PI;
const CE: f64 = 2.0 * PI * EARTH_RADIUS_EQUATOR;
const HALF_SIZE: f64 = EARTH_RADIUS_EQUATOR * PI;

// spherical mercator
const EXTEND_EPSG_3857: [f64; 4] = [-HALF_SIZE, -HALF_SIZE, HALF_SIZE, HALF_SIZE];

// bounds of spherical mercator in WGS84 coordinates
const EXTEND_EPSG_4326: [f64; 4] = [-180.0, -85.0, 180.0, 85.0];

fn truncate_to(c: &Coordinate<f64>, extend: &[f64; 4]) -> Coordinate<f64> {
    Coordinate::from((
        restrict_between(extend[0], extend[2], c.x),
        restrict_between(extend[1], extend[3], c.y),
    ))
}

#[inline(always)]
fn restrict_between(value_min: f64, value_max: f64, value: f64) -> f64 {
    if value > value_max {
        value_max
    } else if value < value_min {
        value_min
    } else {
        value
    }
}

/// Convert longitude and latitude to web mercator
#[allow(dead_code)]
fn lnglat_to_webmercator(c: &Coordinate<f64>) -> Coordinate<f64> {
    let c = truncate_to(c, &EXTEND_EPSG_4326);
    Coordinate::from((
        EARTH_RADIUS_EQUATOR * c.x.to_radians(),
        EARTH_RADIUS_EQUATOR * ((PI * 0.25) + (0.5 * c.y.to_radians())).tan().log(E),
    ))
}

/// Convert web mercator x, y to longitude and latitude
fn webmercator_to_lnglat(c: &Coordinate<f64>) -> Coordinate<f64> {
    let ll_c = Coordinate::from((
        c.x * R2D / EARTH_RADIUS_EQUATOR,
        ((PI * 0.5) - 2.0 * (-1.0 * c.y / EARTH_RADIUS_EQUATOR).exp().atan()) * R2D,
    ));
    truncate_to(&ll_c, &EXTEND_EPSG_4326)
}

pub struct CellBuilder {
    /// valid resolutions to build cells on
    h3_resolutions: Vec<u8>,
}

impl CellBuilder {
    pub fn new<I>(resolution_iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: Borrow<u8>,
    {
        let mut h3_resolutions: Vec<_> = resolution_iter
            .into_iter()
            .filter_map(|r| {
                if r.borrow() <= &H3_MAX_RESOLUTION {
                    Some(*r.borrow())
                } else {
                    None
                }
            })
            .collect();
        h3_resolutions.sort_unstable();
        h3_resolutions.dedup();
        Self { h3_resolutions }
    }

    ///
    /// may slightly exceed `max_num_cells`, as cells at the boundary of the tile
    /// will be added in a second step to prevent having missing cells at the sides
    /// of the tile.
    pub fn cells_bounded(
        &self,
        tile: &Tile,
        max_num_cells: usize,
    ) -> Result<Option<(u8, IndexVec<H3Cell>)>, Error> {
        let area_tile_m2 = tile.area_m2();
        let mut select_h3_resolution = None;
        for h3_resolution in self.h3_resolutions.iter() {
            if ((area_tile_m2 / H3Cell::area_m2(*h3_resolution)) as usize) > max_num_cells {
                break;
            }
            select_h3_resolution = Some(*h3_resolution);
        }
        if let Some(h3_resolution) = select_h3_resolution {
            Ok(Some((h3_resolution, tile.to_h3_cells(h3_resolution)?)))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub fn h3_resolutions(&self) -> &[u8] {
        self.h3_resolutions.as_slice()
    }
}

/*
/// Calculate the approximate area of the given linestring ring (wgs84 coordinates) in square meters
///
/// Roughly taken from [stackoverflow](https://gis.stackexchange.com/questions/711/how-can-i-measure-area-from-geographic-coordinates).
///
/// Published in Chamberlain, R. and W. Duquette. “Some algorithms for polygons on a sphere.” (2007).
/// The full paper is available [here](https://www.semanticscholar.org/paper/Some-algorithms-for-polygons-on-a-sphere.-Chamberlain-Duquette/79668c0fe32788176758a2285dd674fa8e7b8fa8).
fn area_m2(rect: &Rect<f64>) -> f64 {
    rect.to_polygon()
        .exterior()
        .0
        .windows(2)
        .map(|coords| {
            (coords[1].x - coords[0].x).to_radians()
                * (2.0 + coords[0].y.to_radians().sin() + coords[1].y.to_radians().sin())
        })
        .sum::<f64>()
        .abs()
        * EARTH_RADIUS_EQUATOR.powi(2)
        / 2.0
}

 */

#[cfg(test)]
mod tests {
    use super::{CellBuilder, Tile};

    #[test]
    fn it_works() {
        let tile = Tile { x: 10, y: 10, z: 5 };
        let cell_builder = CellBuilder::new(&[1, 2, 3, 4, 5, 6, 7]);
        let (h3_res, cells) = cell_builder.cells_bounded(&tile, 2000).unwrap().unwrap();
        assert!(h3_res <= 7);
        assert!(cells.count() < 2000);
        assert!(cells.count() > 200);
    }
}
