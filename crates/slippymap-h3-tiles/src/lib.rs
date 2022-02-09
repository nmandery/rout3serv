#![warn(
    clippy::all,
    clippy::correctness,
    clippy::suspicious,
    clippy::style,
    clippy::complexity,
    clippy::perf,
    nonstandard_style
)]

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::f64::consts::PI;
use std::fmt;

use geo::algorithm::bounding_rect::BoundingRect;
use geo::algorithm::dimensions::HasDimensions;
use geo::prelude::Contains;
use geo_types::{Coordinate, Rect};
use h3ron::iter::change_resolution;
use h3ron::{Error, H3Cell, H3Edge, ToCoordinate, ToH3Cells, H3_MAX_RESOLUTION};

/// parts of this file have been ported from
/// <https://github.com/mapbox/mercantile/blob/fe3762d14001ca400caf7462f59433b906fc25bd/mercantile/__init__.py#L200>
/// and
/// <https://github.com/openlayers/openlayers/blob/fdba3ecf0e47503dd8e8711a44cf34620be70b2d/src/ol/proj/epsg3857.js#L26>

#[derive(Eq, PartialEq, Debug)]
pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub z: u8,
}

impl Tile {
    pub fn new(x: u32, y: u32, z: u8) -> Self {
        Self { x, y, z }
    }
}

impl PartialOrd<Self> for Tile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// order by z, x, y values
impl Ord for Tile {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.z.cmp(&other.z) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => match self.x.cmp(&other.x) {
                Ordering::Less => Ordering::Less,
                Ordering::Equal => self.y.cmp(&other.y),
                Ordering::Greater => Ordering::Greater,
            },
            Ordering::Greater => Ordering::Greater,
        }
    }
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

    /// `remove_excess` removes cells from outside the box bounding_rect, but this brings in additional cpu usage.
    fn to_h3_cells(&self, h3_resolution: u8, remove_excess: bool) -> Result<Vec<H3Cell>, Error> {
        // operating on a lower resolution reduces the number of point-in-polygon
        // operations to perform during polyfill. As a drawback this over-fetches and
        // and delivers more cells than required to the browser.
        let h3_resolution_offset = 1;

        // buffer the box using the cells size to ensure cells with just their
        // centroid outside of the box, but parts of the outline intersecting
        // are also included.
        // web-mercator uses meters as units.
        let buffered_bbox = {
            let buffer_meters =
                H3Edge::edge_length_m(h3_resolution.saturating_sub(h3_resolution_offset)) * 1.8;
            let wm_bbox = self.webmercator_bounding_rect();

            Rect::new(
                webmercator_to_lnglat(EXTEND_EPSG_3857.truncate_coordinate(Coordinate::from((
                    wm_bbox.min().x - buffer_meters,
                    wm_bbox.min().y - buffer_meters,
                )))),
                webmercator_to_lnglat(EXTEND_EPSG_3857.truncate_coordinate(Coordinate::from((
                    wm_bbox.max().x + buffer_meters,
                    wm_bbox.max().y + buffer_meters,
                )))),
            )
        };
        if buffered_bbox.is_empty() {
            Ok(Default::default())
        } else {
            let buffered =
                buffered_bbox.to_h3_cells(h3_resolution.saturating_sub(h3_resolution_offset))?;
            let cells_iter = change_resolution(buffered.iter(), h3_resolution);
            let cells = if remove_excess {
                // remove cells from outside the box bounding_rect, but this brings in additional cpu usage.
                let latlon_rect = self.bounding_rect();
                cells_iter
                    .filter(|cell| latlon_rect.contains(&cell.to_coordinate()))
                    .collect()
            } else {
                cells_iter.collect()
            };
            Ok(cells)
        }
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

pub struct Extend([f64; 4]);

impl Extend {
    pub const fn min_x(&self) -> f64 {
        self.0[0]
    }

    pub const fn min_y(&self) -> f64 {
        self.0[1]
    }

    pub const fn max_x(&self) -> f64 {
        self.0[2]
    }

    pub const fn max_y(&self) -> f64 {
        self.0[3]
    }

    pub fn truncate_coordinate(&self, coord: Coordinate<f64>) -> Coordinate<f64> {
        Coordinate::from((
            restrict_between(self.min_x(), self.max_x(), coord.x),
            restrict_between(self.min_y(), self.max_y(), coord.y),
        ))
    }
}

const EARTH_RADIUS_EQUATOR: f64 = 6378137.0;
const R2D: f64 = 180.0 / PI;
const CE: f64 = 2.0 * PI * EARTH_RADIUS_EQUATOR;
const HALF_SIZE: f64 = EARTH_RADIUS_EQUATOR * PI;

/// spherical mercator
pub const EXTEND_EPSG_3857: Extend = Extend([-HALF_SIZE, -HALF_SIZE, HALF_SIZE, HALF_SIZE]);

/// bounds of spherical mercator in WGS84 coordinates
pub const EXTEND_EPSG_4326: Extend = Extend([-180.0, -85.0, 180.0, 85.0]);

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
fn lnglat_to_webmercator(c: Coordinate<f64>) -> Coordinate<f64> {
    let c = EXTEND_EPSG_4326.truncate_coordinate(c);
    Coordinate::from((
        EARTH_RADIUS_EQUATOR * c.x.to_radians(),
        EARTH_RADIUS_EQUATOR * PI.mul_add(0.25, 0.5 * c.y.to_radians()).tan().ln(),
    ))
}

/// Convert web mercator x, y to longitude and latitude
fn webmercator_to_lnglat(c: Coordinate<f64>) -> Coordinate<f64> {
    let ll_c = Coordinate::from((
        c.x * R2D / EARTH_RADIUS_EQUATOR,
        ((PI * 0.5) - 2.0 * (-1.0 * c.y / EARTH_RADIUS_EQUATOR).exp().atan()) * R2D,
    ));
    EXTEND_EPSG_4326.truncate_coordinate(ll_c)
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
        remove_excess: bool,
    ) -> Result<Option<(u8, Vec<H3Cell>)>, Error> {
        let area_tile_m2 = tile.area_m2();
        let mut select_h3_resolution = None;
        for h3_resolution in self.h3_resolutions.iter() {
            if ((area_tile_m2 / H3Cell::area_m2(*h3_resolution)) as usize) > max_num_cells {
                break;
            }
            select_h3_resolution = Some(*h3_resolution);
        }
        if let Some(h3_resolution) = select_h3_resolution {
            Ok(Some((
                h3_resolution,
                tile.to_h3_cells(h3_resolution, remove_excess)?,
            )))
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
    fn cell_builder_cells_bounded() {
        let tile = Tile { x: 10, y: 10, z: 5 };
        let cell_builder = CellBuilder::new(&[1, 2, 3, 4, 5, 6, 7]);
        let (h3_res, cells) = cell_builder
            .cells_bounded(&tile, 2000, false)
            .unwrap()
            .unwrap();
        assert!(h3_res <= 7);
        assert!(cells.iter().count() < 2000);
        assert!(cells.iter().count() > 200);
    }

    #[test]
    fn tiles_ordering() {
        let mut tiles = vec![
            Tile::new(56, 23, 10),
            Tile::new(56, 23, 5),
            Tile::new(10, 23, 5),
        ];
        tiles.sort_unstable();
        assert_eq!(tiles[0], Tile::new(10, 23, 5));
        assert_eq!(tiles[1], Tile::new(56, 23, 5));
        assert_eq!(tiles[2], Tile::new(56, 23, 10));
    }
}
