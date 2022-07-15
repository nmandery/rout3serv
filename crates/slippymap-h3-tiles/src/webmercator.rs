use crate::BoundingBox;
use geo::{MapCoords, MapCoordsInPlace};
use geo_types::Coordinate;
use std::f64::consts::PI;

pub trait Wgs84ToWebmercator {
    fn wgs84_to_webmercator(self) -> Self;
}

pub trait Wgs84ToWebmercatorInPlace {
    fn wgs84_to_webmercator_in_place(&mut self);
}

pub trait WebmercatorToWgs84 {
    fn webmercator_to_wgs84(self) -> Self;
}

pub trait WebmercatorToWgs84InPlace {
    fn webmercator_to_wgs84_in_place(&mut self);
}

const EARTH_RADIUS_EQUATOR: f64 = 6378137.0;
const R2D: f64 = 180.0 / PI;
pub(crate) const CE: f64 = 2.0 * PI * EARTH_RADIUS_EQUATOR;
const HALF_SIZE: f64 = EARTH_RADIUS_EQUATOR * PI;
pub(crate) const EPSILON: f64 = 1.0e-14;

/// spherical mercator
pub const EXTEND_EPSG_3857: BoundingBox =
    BoundingBox([-HALF_SIZE, -HALF_SIZE, HALF_SIZE, HALF_SIZE]);

/// bounds of spherical mercator in WGS84 coordinates
pub const EXTEND_EPSG_4326: BoundingBox = BoundingBox([-180.0, -85.0, 180.0, 85.0]);

impl<T> Wgs84ToWebmercator for T
where
    T: MapCoords<f64, f64, Output = T>,
{
    fn wgs84_to_webmercator(self) -> Self {
        self.map_coords(coordinate_wgs84_to_webmercator)
    }
}

impl<T> WebmercatorToWgs84 for T
where
    T: MapCoords<f64, f64, Output = T>,
{
    fn webmercator_to_wgs84(self) -> Self {
        self.map_coords(coordinate_webmercator_to_wgs84)
    }
}

impl<T> Wgs84ToWebmercatorInPlace for T
where
    T: MapCoordsInPlace<f64>,
{
    fn wgs84_to_webmercator_in_place(&mut self) {
        self.map_coords_in_place(coordinate_wgs84_to_webmercator)
    }
}

impl<T> WebmercatorToWgs84InPlace for T
where
    T: MapCoordsInPlace<f64>,
{
    fn webmercator_to_wgs84_in_place(&mut self) {
        self.map_coords_in_place(coordinate_webmercator_to_wgs84)
    }
}

pub fn coordinate_wgs84_to_webmercator(c: Coordinate) -> Coordinate {
    let c = EXTEND_EPSG_4326.clamp_coordinate(c);
    Coordinate::from((
        EARTH_RADIUS_EQUATOR * c.x.to_radians(),
        EARTH_RADIUS_EQUATOR * PI.mul_add(0.25, 0.5 * c.y.to_radians()).tan().ln(),
    ))
}

pub fn coordinate_webmercator_to_wgs84(c: Coordinate) -> Coordinate {
    let ll_c = Coordinate::from((
        c.x * R2D / EARTH_RADIUS_EQUATOR,
        ((PI * 0.5) - 2.0 * (-1.0 * c.y / EARTH_RADIUS_EQUATOR).exp().atan()) * R2D,
    ));
    EXTEND_EPSG_4326.clamp_coordinate(ll_c)
}
