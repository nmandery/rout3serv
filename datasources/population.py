"""
pulls data from facebooks Data for good Population density maps

https://dataforgood.fb.com/tools/population-density-maps/
"""
import os
import tempfile
from typing import List, Optional

import click
import geojson
import geopandas as gpd
import h3.api.numpy_int as h3
import h3ronpy.raster
import numpy as np
import pandas as pd
import pyarrow as pa
import pyproj
import rasterio
import rasterio.windows
import shapely.ops
from h3ronpy.util import dataframe_to_geodataframe
from h3ronpy.vector import geodataframe_to_h3
from pyarrow.fs import LocalFileSystem
from shapely.geometry import shape

# datasets are in WGS84, so no reprojection required

# mapping datasets to columns in the dataframe
DATASETS = (
    # list of datasets can be obtained by
    # aws s3 ls s3://dataforgood-fb-data/hrsl-cogs/ --no-sign-request
    ('hrsl_children_under_five', 'population_children_under_five'),
    ('hrsl_elderly_60_plus', 'population_elderly_60_plus'),
    ('hrsl_general', 'population'),
    ('hrsl_youth_15_24', 'population_youth_15_24'),
)


def fetch_dataset(dataset_name: str, bounds: List[float], conversion_h3_res: Optional[int] = None) -> (
        pd.DataFrame, int):
    filename = f"/vsis3/dataforgood-fb-data/hrsl-cogs/{dataset_name}/{dataset_name}-latest.vrt"
    print(f"Loading {dataset_name} dataset")
    with rasterio.open(filename) as ds:
        window = rasterio.windows.from_bounds(*bounds, transform=ds.transform)
        window_transform = rasterio.windows.transform(window, ds.transform)
        band = ds.read(1, window=window)

        if conversion_h3_res is None:
            # determinate the h3 resolution to use
            conversion_h3_res = h3ronpy.raster.nearest_h3_resolution(band.shape, window_transform)
            print(f"Using h3 resolution {conversion_h3_res} for raster -> h3 conversion")

        nodata_value = 0.0
        band[np.isnan(band)] = nodata_value
        df = h3ronpy.raster.raster_to_dataframe(band, window_transform, conversion_h3_res,
                                                nodata_value=nodata_value,
                                                compacted=False)
        return df, conversion_h3_res


def assemble_dataframe(bounds: List[float], target_h3_res: int) -> pd.DataFrame:
    df = pd.DataFrame({"h3index": []})
    with rasterio.Env(AWS_NO_SIGN_REQUEST="YES"):
        conversion_h3_res = None
        for dataset_name, column_name in DATASETS:
            col_df, conversion_h3_res = fetch_dataset(dataset_name, bounds, conversion_h3_res=conversion_h3_res)
            if col_df.empty:
                print(f"Dataset {dataset_name} was empty")
                col_df = pd.DataFrame(
                    {"h3index": np.array([], dtype=np.uint64), column_name: np.array([], dtype=np.float16)})
            else:
                col_df.rename(columns={"value": column_name}, inplace=True)

            if target_h3_res > conversion_h3_res:
                raise ValueError(f"only h3 resolutions up to {conversion_h3_res}. up-scaling is not implemented")
            elif target_h3_res < conversion_h3_res:
                # scale down
                col_df["h3index"] = col_df["h3index"].apply(lambda i: h3.h3_to_parent(i, target_h3_res))
                col_df = col_df.groupby(by=["h3index"]).sum().reset_index()

            if df.empty:
                df = col_df
            else:
                df = df.merge(col_df, how="outer", on="h3index", copy=False)
    return df


WGS84 = pyproj.CRS('EPSG:4326')
SPHERICAL_MERCATOR = pyproj.CRS('EPSG:3857')  # uses meters as units


def h3_buffer(geom, h3_resolution: int):
    """
    apply a slight buffering as the areas of h3 index hierarchies do not completely overlap
    """
    sm_geom = shapely.ops.transform(pyproj.Transformer.from_proj(
        WGS84, SPHERICAL_MERCATOR
    ).transform, geom)
    buffered = sm_geom.buffer(h3.edge_length(h3_resolution, unit="m"))
    return shapely.ops.transform(pyproj.Transformer.from_proj(
        SPHERICAL_MERCATOR, WGS84
    ).transform, buffered)


@click.command()
@click.argument("aoi_geojson_polygon_geometry_filename")
@click.argument("out_dir")
@click.option('--h3_res', default=10, help="H3 resolution the resulting data should have")
@click.option('--group_h3_res', default=6, help="the H3 resolution to use for grouping the data in files")
@click.option('--fgb', default=False, is_flag=True,
              help="Create flatgeobuf geo-datasets alongside the arrow files. fgbs can not be written when the data "
                   "is empty")
def cli(aoi_geojson_polygon_geometry_filename, out_dir, h3_res, group_h3_res, fgb):
    """Convert the COGs to arrow files using h3 resolution `target_h3_resolution`

    The AOI gets split into separate files according to `group_h3_res` to allow easy access of
    subsets of the data.

    Can also create additional FlatGeobuf files.

    Even when there is no data for a "bin", the corresponding file will be created. As pandas
    can not write empty FGB files, geodata files will not be created in this case.
    """
    aoi_geom = shape(geojson.loads(open(aoi_geojson_polygon_geometry_filename).read()))
    fs = LocalFileSystem()

    group_h3_res = min(h3_res, group_h3_res)
    # avoid too many small fetches
    fetch_h3_res = min(group_h3_res, 3)

    fetch_df = dataframe_to_geodataframe(
        geodataframe_to_h3(gpd.GeoDataFrame({}, geometry=[h3_buffer(aoi_geom, fetch_h3_res)], crs=4326), fetch_h3_res)
    )

    if fetch_df.empty:
        return

    out_path = f"{str(out_dir)}/{group_h3_res}/{h3_res}"
    fs.create_dir(out_path, recursive=True)

    for _, row in fetch_df.iterrows():
        df = assemble_dataframe(row["geometry"].bounds, h3_res)

        # remove over-fetched indexes
        df["fetch_h3index"] = df["h3index"] \
            .apply(lambda i: h3.h3_to_parent(i, fetch_h3_res))
        filtered_df = df[df["fetch_h3index"] == row["h3index"]] \
            .drop(["fetch_h3index"], axis=1)

        # create the groups
        filtered_df["group_h3index"] = filtered_df["h3index"] \
            .apply(lambda i: h3.h3_to_parent(i, group_h3_res))

        for group_h3index in h3.h3_to_children(row["h3index"], group_h3_res):
            out_name = f"{out_path}/{h3.h3_to_string(group_h3index)}"
            print(f"creating {out_name}")
            group_df = filtered_df[filtered_df["group_h3index"] == group_h3index] \
                .drop(["group_h3index"], axis=1)

            table = pa.Table.from_pandas(group_df)
            with fs.open_output_stream(f"{out_name}.arrow") as nativefile:
                with pa.RecordBatchFileWriter(nativefile, table.schema) as writer:
                    writer.write_table(table)

            if fgb and not group_df.empty:
                tf_name = tempfile.mktemp(suffix=".fgb")
                try:
                    geodf = dataframe_to_geodataframe(group_df, column_name="h3index")
                    geodf.to_file(tf_name, driver="FlatGeobuf", layer="population")

                    with fs.open_output_stream(f"{out_name}.fgb") as nativefile:
                        nativefile.write(open(tf_name, mode='rb').read())
                        nativefile.flush()
                finally:
                    os.remove(tf_name)


if __name__ == '__main__':
    cli()
