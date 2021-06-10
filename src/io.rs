use eyre::Result;
use gdal::vector::{Defn, Feature, FieldDefn, OGRFieldType, OGRwkbGeometryType, ToGdal};
use gdal::Driver;
use geo_types::LineString;
use h3ron::ToCoordinate;
use serde::Serialize;

use crate::graph::Graph;

pub trait OgrWrite {
    fn ogr_write<T: AsRef<str>>(&self, driver_name: T, output_name: T, layer_name: T)
        -> Result<()>;
}

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

#[derive(Serialize)]
pub struct GraphStats {
    pub h3_resolution: u8,
    pub input_graph: InputGraphStats,
    pub prepared_graph: PreparedGraphStats,
}

#[derive(Serialize)]
pub struct InputGraphStats {
    pub nodes: usize,
    pub edges: usize,
}

#[derive(Serialize)]
pub struct PreparedGraphStats {
    pub nodes: usize,
    pub in_edges: usize,
    pub out_edges: usize,
}

impl GraphStats {
    pub fn new(graph: &Graph) -> Self {
        Self {
            h3_resolution: graph.h3_resolution,
            input_graph: InputGraphStats {
                nodes: graph.input_graph.get_num_nodes(),
                edges: graph.input_graph.get_num_edges(),
            },
            prepared_graph: PreparedGraphStats {
                nodes: graph.graph.get_num_nodes(),
                in_edges: graph.graph.get_num_in_edges(),
                out_edges: graph.graph.get_num_out_edges(),
            },
        }
    }
}
