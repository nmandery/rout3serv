use geo_types::Coord;
use std::f64::consts::PI;

const EARTH_RADIUS_EQUATOR: f64 = 6378137.0;
const R2D: f64 = 180.0 / PI;
//pub(crate) const CE: f64 = 2.0 * PI * EARTH_RADIUS_EQUATOR;
const HALF_SIZE: f64 = EARTH_RADIUS_EQUATOR * PI;
//pub(crate) const EPSILON: f64 = 1.0e-14;

#[allow(dead_code)]
/// spherical mercator
pub const EXTEND_EPSG_3857: BoundingBox =
    BoundingBox([-HALF_SIZE, -HALF_SIZE, HALF_SIZE, HALF_SIZE]);

/// bounds of spherical mercator in WGS84 coordinates
pub const EXTEND_EPSG_4326: BoundingBox = BoundingBox([-180.0, -85.0, 180.0, 85.0]);

pub struct BoundingBox(pub [f64; 4]);

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

    pub fn clamp_coordinate(&self, coord: Coord) -> Coord {
        Coord::from((
            coord.x.clamp(self.min_x(), self.max_x()),
            coord.y.clamp(self.min_y(), self.max_y()),
        ))
    }
}

pub fn wgs84_to_webmercator(c: Coord) -> Coord {
    let c = EXTEND_EPSG_4326.clamp_coordinate(c);
    Coord::from((
        EARTH_RADIUS_EQUATOR * c.x.to_radians(),
        EARTH_RADIUS_EQUATOR * PI.mul_add(0.25, 0.5 * c.y.to_radians()).tan().ln(),
    ))
}

pub fn webmercator_to_wgs84(c: Coord) -> Coord {
    let ll_c = Coord::from((
        c.x * R2D / EARTH_RADIUS_EQUATOR,
        ((PI * 0.5) - 2.0 * (-1.0 * c.y / EARTH_RADIUS_EQUATOR).exp().atan()) * R2D,
    ));
    EXTEND_EPSG_4326.clamp_coordinate(ll_c)
}
