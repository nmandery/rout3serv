use crate::geo::Error;
use geo_types::Geometry;
use geozero::wkb::{FromWkb, WkbDialect, WkbWriter};
use geozero::GeozeroGeometry;
use std::io::Cursor;

/// convert a geotypes `Geometry` to WKB
pub fn to_wkb(geom: &Geometry) -> Result<Vec<u8>, Error> {
    let mut wkb: Vec<u8> = Vec::with_capacity(20_000);
    let mut writer = WkbWriter::new(&mut wkb, WkbDialect::Wkb);
    geom.process_geom(&mut writer)?;
    Ok(wkb)
}

pub fn from_wkb(bytes: &[u8]) -> Result<Geometry, Error> {
    let mut cur = Cursor::new(bytes);
    Ok(Geometry::from_wkb(&mut cur, WkbDialect::Wkb)?)
}
