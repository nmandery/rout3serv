use gdal::errors::Result;
use gdal::vector::{Defn, Feature, FieldDefn, Layer, OGRFieldType, OGRwkbGeometryType, ToGdal};
use gdal::Driver;
use geo_types::LineString;
use h3ron::ToCoordinate;

use crate::graph::H3Graph;

pub trait OgrWrite {
    fn ogr_write<S: AsRef<str>>(&self, driver_name: S, output_name: S, layer_name: S)
        -> Result<()>;
}

pub trait WeightFeatureField {
    fn register_weight_fields(layer: &Layer) -> Result<()>;
    fn fill_weight_feature_fields<'a>(&self, feature: &'a mut Feature) -> Result<()>;
}

pub const WEIGHT_FIELD_NAME: &'static str = "weight";

impl WeightFeatureField for u64 {
    fn register_weight_fields(layer: &Layer) -> Result<()> {
        let weight_field_defn = FieldDefn::new(WEIGHT_FIELD_NAME, OGRFieldType::OFTInteger64)?;
        weight_field_defn.add_to_layer(&layer)
    }

    fn fill_weight_feature_fields<'a>(&self, feature: &mut Feature<'a>) -> Result<()> {
        feature.set_field_integer64(WEIGHT_FIELD_NAME, *self as i64)
    }
}

impl WeightFeatureField for i64 {
    fn register_weight_fields(layer: &Layer) -> Result<()> {
        let weight_field_defn = FieldDefn::new(WEIGHT_FIELD_NAME, OGRFieldType::OFTInteger64)?;
        weight_field_defn.add_to_layer(&layer)
    }

    fn fill_weight_feature_fields<'a>(&self, feature: &mut Feature<'a>) -> Result<()> {
        feature.set_field_integer64(WEIGHT_FIELD_NAME, *self)
    }
}

impl WeightFeatureField for f64 {
    fn register_weight_fields(layer: &Layer) -> Result<()> {
        let weight_field_defn = FieldDefn::new(WEIGHT_FIELD_NAME, OGRFieldType::OFTReal)?;
        weight_field_defn.add_to_layer(&layer)
    }

    fn fill_weight_feature_fields<'a>(&self, feature: &mut Feature<'a>) -> Result<()> {
        feature.set_field_double(WEIGHT_FIELD_NAME, *self)
    }
}

impl WeightFeatureField for f32 {
    fn register_weight_fields(layer: &Layer) -> Result<()> {
        let weight_field_defn = FieldDefn::new(WEIGHT_FIELD_NAME, OGRFieldType::OFTReal)?;
        weight_field_defn.add_to_layer(&layer)
    }

    fn fill_weight_feature_fields<'a>(&self, feature: &mut Feature<'a>) -> Result<()> {
        feature.set_field_double(WEIGHT_FIELD_NAME, *self as f64)
    }
}

impl WeightFeatureField for i32 {
    fn register_weight_fields(layer: &Layer) -> Result<()> {
        let weight_field_defn = FieldDefn::new(WEIGHT_FIELD_NAME, OGRFieldType::OFTInteger)?;
        weight_field_defn.add_to_layer(&layer)
    }

    fn fill_weight_feature_fields<'a>(&self, feature: &mut Feature<'a>) -> Result<()> {
        feature.set_field_integer(WEIGHT_FIELD_NAME, *self as i32)
    }
}

impl WeightFeatureField for u32 {
    fn register_weight_fields(layer: &Layer) -> Result<()> {
        let weight_field_defn = FieldDefn::new(WEIGHT_FIELD_NAME, OGRFieldType::OFTInteger)?;
        weight_field_defn.add_to_layer(&layer)
    }

    fn fill_weight_feature_fields<'a>(&self, feature: &mut Feature<'a>) -> Result<()> {
        feature.set_field_integer(WEIGHT_FIELD_NAME, *self as i32)
    }
}

impl<T> OgrWrite for H3Graph<T>
where
    T: WeightFeatureField,
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
        T::register_weight_fields(&lyr)?;

        let defn = Defn::from_layer(&lyr);

        for (edge, weight) in self.edges.iter() {
            let mut ft = Feature::new(&defn)?;
            let coords = vec![
                edge.origin_index_unchecked().to_coordinate(),
                edge.destination_index_unchecked().to_coordinate(),
            ];
            ft.set_geometry(LineString::from(coords).to_gdal()?)?;
            weight.fill_weight_feature_fields(&mut ft)?;
            ft.create(&lyr)?;
        }
        Ok(())
    }
}
