#![warn(
    clippy::all,
    clippy::correctness,
    clippy::suspicious,
    clippy::style,
    clippy::complexity,
    clippy::perf,
    nonstandard_style
)]

pub mod webmercator;

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::f64::consts::{E, PI};
use std::fmt;

use crate::webmercator::{
    WebmercatorToWgs84InPlace, CE, EPSILON, EXTEND_EPSG_3857, EXTEND_EPSG_4326,
};
use geo::algorithm::bounding_rect::BoundingRect;
use geo::algorithm::dimensions::HasDimensions;
use geo::dimensions::Dimensions;
use geo::prelude::Contains;
use geo::Area;
use geo_types::{Coordinate, Rect};
use h3ron::iter::change_resolution;
use h3ron::{Error, H3Cell, H3DirectedEdge, ToCoordinate, ToH3Cells, ToPolygon, H3_MAX_RESOLUTION};

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
        match self.webmercator_bounding_rect() {
            Ok(wbr) => wbr.width() * wbr.height(),
            Err(_) => 0.0,
        }
    }

    /// Get the web mercator bounding box of a tile
    pub fn webmercator_bounding_rect(&self) -> Result<Rect<f64>, Error> {
        let tile_size = CE / 2.0f64.powi(self.z as i32);
        let left = (self.x as f64 * tile_size) - (CE / 2.0);
        let top = (CE / 2.0) - (self.y as f64 * tile_size);
        EXTEND_EPSG_3857.clamp_rect(Rect::new(
            Coordinate::from((left, top)),
            Coordinate::from((left + tile_size, top - tile_size)),
        ))
    }

    /// find the tile the coordinate is located in
    #[allow(dead_code)]
    fn from_wgs84_coordinate(coord: Coordinate<f64>, z: u8) -> Result<Self, Error> {
        let coord = EXTEND_EPSG_4326.clamp_coordinate(coord);
        let z2 = 2.0_f64.powi(z as i32);

        let sinlat = coord.y.to_radians().sin();
        if sinlat == 1.0 {
            Err(Error::Failed)
        } else {
            let y = 0.5 - 0.25 * ((1.0 + sinlat) / (1.0 - sinlat)).log(E) / PI;
            let x = coord.x / 360.0 + 0.5;

            let x_tile = if x <= 0.0 {
                0
            } else if x >= 1.0 {
                (z2 as u32).saturating_sub(1)
            } else {
                // To address loss of precision in round-tripping between tile
                // and lng/lat, points within EPSILON of the right side of a tile
                // are counted in the next tile over.
                ((x + EPSILON) * z2).floor() as u32
            };

            let y_tile = if y <= 0.0 {
                0
            } else if y >= 1.0 {
                (z2 as u32).saturating_sub(1)
            } else {
                ((y + EPSILON) * z2).floor() as u32
            };

            Ok(Tile {
                x: x_tile,
                y: y_tile,
                z,
            })
        }
    }

    /// find the tile the centroid of cell is located in.
    #[allow(dead_code)]
    fn from_cell(cell: H3Cell, z: u8) -> Result<Self, Error> {
        Self::from_wgs84_coordinate(cell.to_coordinate()?, z)
    }

    /// find all tiles parts of the given cell are located in
    #[allow(dead_code)]
    fn all_intersecting_with_cell(cell: H3Cell, z: u8) -> Result<Vec<Self>, Error> {
        let cell_bounds = cell.to_polygon()?.bounding_rect().ok_or(Error::Failed)?;
        let tile_min = Self::from_wgs84_coordinate(cell_bounds.min(), z)?;
        let tile_max = Self::from_wgs84_coordinate(cell_bounds.max(), z)?;

        let mut tiles = vec![];
        for x in tile_min.x..=tile_max.x {
            for y in tile_max.y..=tile_min.y {
                tiles.push(Tile { x, y, z })
            }
        }
        Ok(tiles)
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
            let buffer_meters = H3DirectedEdge::edge_length_avg_m(
                h3_resolution.saturating_sub(h3_resolution_offset),
            )? * 1.8;
            let wm_bbox = match self.webmercator_bounding_rect() {
                Ok(wm_bbox) => wm_bbox,
                Err(Error::LatLonDomain) => return Ok(Default::default()), // out of coordinate system bounds
                Err(e) => return Err(e),
            };

            let mut buffered_bbox = Rect::new(
                Coordinate::from((
                    wm_bbox.min().x - buffer_meters,
                    wm_bbox.min().y - buffer_meters,
                )),
                Coordinate::from((
                    wm_bbox.max().x + buffer_meters,
                    wm_bbox.max().y + buffer_meters,
                )),
            );
            buffered_bbox.webmercator_to_wgs84_in_place();
            if buffered_bbox.unsigned_area() == 0.0 {
                // box is empty or just a line
                return Ok(Default::default());
            } else {
                buffered_bbox
            }
        };
        let buffered =
            buffered_bbox.to_h3_cells(h3_resolution.saturating_sub(h3_resolution_offset))?;
        let cells_iter = change_resolution(buffered.iter(), h3_resolution);
        let mut cells = Vec::with_capacity(cells_iter.size_hint().0);
        if remove_excess {
            // remove cells from outside the box bounding_rect, but this brings in additional cpu usage.
            let latlon_rect = self.bounding_rect();

            for cell in cells_iter {
                let cell = cell?;
                if latlon_rect.contains(&cell.to_coordinate()?) {
                    cells.push(cell);
                }
            }
        } else {
            for cell in cells_iter {
                let cell = cell?;
                cells.push(cell);
            }
        };
        Ok(cells)
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

pub struct BoundingBox([f64; 4]);

impl BoundingBox {
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

    pub fn clamp_coordinate(&self, coord: Coordinate<f64>) -> Coordinate<f64> {
        Coordinate::from((
            coord.x.clamp(self.min_x(), self.max_x()),
            coord.y.clamp(self.min_y(), self.max_y()),
        ))
    }

    pub fn clamp_rect(&self, rect: Rect<f64>) -> Result<Rect<f64>, Error> {
        let truncated = Rect::new(
            self.clamp_coordinate(rect.min()),
            self.clamp_coordinate(rect.max()),
        );
        if truncated.dimensions() == Dimensions::TwoDimensional {
            Ok(truncated)
        } else {
            Err(Error::LatLonDomain)
        }
    }
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
            if ((area_tile_m2 / H3Cell::area_avg_m2(*h3_resolution)?) as usize) > max_num_cells {
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

#[cfg(test)]
mod tests {
    use crate::webmercator::Wgs84ToWebmercator;
    use approx::assert_ulps_eq;
    use geo::prelude::BoundingRect;
    use geo_types::Coordinate;
    use h3ron::H3Cell;

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
        assert!(cells.len() < 2000);
        assert!(cells.len() > 200);
    }

    #[test]
    fn tiles_ordering() {
        let mut tiles = vec![
            Tile::new(22, 23, 10),
            Tile::new(22, 23, 5),
            Tile::new(10, 23, 5),
        ];
        tiles.sort_unstable();
        assert_eq!(tiles[0], Tile::new(10, 23, 5));
        assert_eq!(tiles[1], Tile::new(22, 23, 5));
        assert_eq!(tiles[2], Tile::new(22, 23, 10));
    }

    #[test]
    fn cell_tile_roundtrip() {
        let tile = Tile::new(50, 50, 7);

        let cells = tile.to_h3_cells(5, true).unwrap();
        assert!(cells.len() > 200);
        for cell in cells {
            assert_eq!(Tile::from_cell(cell, tile.z).unwrap(), tile);
        }
    }

    #[test]
    fn tile_box_coordtransform() {
        let tile = Tile::new(50, 50, 7);
        let rect_sphericalmercator = tile.webmercator_bounding_rect().unwrap();
        let rect_sphericalmercator_transformed = tile.bounding_rect().wgs84_to_webmercator();

        assert_ulps_eq!(
            rect_sphericalmercator.min().x,
            rect_sphericalmercator_transformed.min().x,
            max_ulps = 8
        );
        assert_ulps_eq!(
            rect_sphericalmercator.min().y,
            rect_sphericalmercator_transformed.min().y,
            max_ulps = 8
        );
        assert_ulps_eq!(
            rect_sphericalmercator.max().x,
            rect_sphericalmercator_transformed.max().x,
            max_ulps = 8
        );
        assert_ulps_eq!(
            rect_sphericalmercator.max().y,
            rect_sphericalmercator_transformed.max().y,
            max_ulps = 8
        );
    }

    #[test]
    fn test_tiles_intersecting_cell_many_tiles() {
        let cell = H3Cell::from_coordinate(Coordinate::from((20.0, 30.0)), 5).unwrap();
        let tile_z = 12;
        let main_tile = Tile::from_cell(cell, tile_z).unwrap();

        let tiles_touched = Tile::all_intersecting_with_cell(cell, tile_z).unwrap();
        assert!(tiles_touched.len() > 3);
        assert!(tiles_touched.contains(&main_tile));
    }

    #[test]
    fn test_all_intersecting_cell_single_tile() {
        let cell = H3Cell::from_coordinate(Coordinate::from((20.0, 30.0)), 5).unwrap();
        let tile_z = 5;
        let main_tile = Tile::from_cell(cell, tile_z).unwrap();

        let tiles_touched = Tile::all_intersecting_with_cell(cell, tile_z).unwrap();
        assert_eq!(tiles_touched.len(), 1);
        assert!(tiles_touched.contains(&main_tile));
    }
}
