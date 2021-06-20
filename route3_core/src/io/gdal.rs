use std::ops::Add;

use eyre::Result;
use gdal::vector::{Defn, Feature, OGRwkbGeometryType, ToGdal};
use gdal::Driver;
use geo_types::LineString;
use h3ron::ToCoordinate;

use crate::graph::H3Graph;

pub trait OgrWrite {
    fn ogr_write<S: AsRef<str>>(&self, driver_name: S, output_name: S, layer_name: S)
        -> Result<()>;
}

impl<T> OgrWrite for H3Graph<T>
where
    T: PartialOrd + PartialEq + Add + Copy,
{
    fn ogr_write<S: AsRef<str>>(
        &self,
        driver_name: S,
        output_name: S,
        layer_name: S,
    ) -> Result<()> {
        let drv = Driver::get(driver_name.as_ref())?;
        let mut ds = drv.create_vector_only(output_name.as_ref())?;

        let lyr = ds.create_layer(layer_name.as_ref(), None, OGRwkbGeometryType::wkbLineString)?;

        //let weight_field_defn = FieldDefn::new("weight", OGRFieldType::OFTInteger64)?;
        //weight_field_defn.add_to_layer(&lyr)?;

        let defn = Defn::from_layer(&lyr);

        for (edge, _) in self.edges.iter() {
            let mut ft = Feature::new(&defn)?;
            let coords = vec![
                edge.origin_index_unchecked().to_coordinate(),
                edge.destination_index_unchecked().to_coordinate(),
            ];
            ft.set_geometry(LineString::from(coords).to_gdal()?)?;
            //ft.set_field_integer64("weight", edge.weight as i64)?;
            ft.create(&lyr)?;
        }
        Ok(())
    }
}
