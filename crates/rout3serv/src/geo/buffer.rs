use crate::geo::webmercator::{webmercator_to_wgs84, wgs84_to_webmercator};
use crate::geo::Error;
use geo::{MapCoords, MapCoordsInPlace};
use geo_types::Geometry;
use geos::Geom;
use geozero::{ToGeo, ToGeos};
use uom::si::f64::Length;
use uom::si::length::meter;

/// buffer a geometry in meters
///
/// This function creates some distortion as the geometry is transformed
/// between WGS84 and Spherical Mercator
pub fn buffer(geom: &Geometry, distance: Length) -> Result<Geometry, Error> {
    let geom = geom.map_coords(wgs84_to_webmercator);
    let ggeom = geom.to_geos()?;
    let buffered = ggeom.buffer(distance.get::<meter>(), 2)?;
    let mut o: Geometry = buffered.to_geo()?;
    o.map_coords_in_place(webmercator_to_wgs84);
    Ok(o)
}
