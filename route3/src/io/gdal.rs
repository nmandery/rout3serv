use eyre::Result;
use gdal::vector::{Defn, Feature, FieldDefn, OGRFieldType, OGRwkbGeometryType, ToGdal};
use gdal::Driver;
use geo_types::LineString;
use h3ron::ToCoordinate;

use crate::graph::Graph;

#[cfg(feature = "gdal")]
pub trait OgrWrite {
    fn ogr_write<T: AsRef<str>>(&self, driver_name: T, output_name: T, layer_name: T)
        -> Result<()>;
}

#[cfg(feature = "gdal")]
impl OgrWrite for Graph {
    fn ogr_write<T: AsRef<str>>(
        &self,
        driver_name: T,
        output_name: T,
        layer_name: T,
    ) -> Result<()> {
        let drv = Driver::get(driver_name.as_ref())?;
        let mut ds = drv.create_vector_only(output_name.as_ref())?;

        let lyr = ds.create_layer(layer_name.as_ref(), None, OGRwkbGeometryType::wkbLineString)?;

        let weight_field_defn = FieldDefn::new("weight", OGRFieldType::OFTInteger64)?;
        weight_field_defn.add_to_layer(&lyr)?;

        let defn = Defn::from_layer(&lyr);

        for edge in self.input_graph.get_edges() {
            let mut ft = Feature::new(&defn)?;
            let coords = vec![
                self.h3cell_by_nodeid(edge.from)?.to_coordinate(),
                self.h3cell_by_nodeid(edge.to)?.to_coordinate(),
            ];
            ft.set_geometry(LineString::from(coords).to_gdal()?)?;
            ft.set_field_integer64("weight", edge.weight as i64)?;
            ft.create(&lyr)?;
        }
        Ok(())
    }
}
