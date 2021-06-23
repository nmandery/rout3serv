use eyre::Result;
use gdal::spatial_ref::SpatialRef;
use gdal::vector::Geometry;

/// buffer a geometry in meters
///
/// This function creates some distortion as the geometry is transformed
/// between WGS84 and Spherical Mercator
pub fn buffer_meters(geom: &Geometry, meters: f64) -> Result<Geometry> {
    let srs_wgs84 = SpatialRef::from_epsg(4326)?;
    let srs_spherical_mercator = SpatialRef::from_epsg(3857)?;
    let mut geom_sm_buffered = {
        let mut geom_cloned = geom.clone();
        geom_cloned.set_spatial_ref(srs_wgs84.clone());
        geom_cloned
            .transform_to(&srs_spherical_mercator)?
            .buffer(meters, 4)?
    };
    geom_sm_buffered.set_spatial_ref(srs_spherical_mercator);
    Ok(geom_sm_buffered.transform_to(&srs_wgs84)?)
}
