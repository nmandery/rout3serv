__version__ = '0.2.0'

import typing
from typing import Optional, Iterable, Tuple

import geopandas as gpd
import grpc
import numpy as np
import pandas as pd
import pyarrow as pa
import shapely.wkb
from shapely.geometry import Point
from shapely.geometry.base import BaseGeometry

from . import route3_road_pb2
from .route3_road_pb2_grpc import Route3RoadStub
from .route3_road_pb2 import GraphHandle

DEFAULT_PORT = 7088


class DataFrameWithId:
    object_id: str
    population_within_disturbance: float
    dataframe: pd.DataFrame


class Server:
    channel = None
    stub = None

    def __init__(self, hostport: str = f"127.0.0.1:{DEFAULT_PORT}",
                 credentials: Optional[grpc.ChannelCredentials] = None):
        if credentials is not None:
            self.channel = grpc.secure_channel(hostport, credentials)
        else:
            self.channel = grpc.insecure_channel(hostport)
        self.stub = Route3RoadStub(self.channel)

    def version(self) -> route3_road_pb2.VersionResponse:
        return self.stub.Version(route3_road_pb2.Empty())

    def list_graphs(self) -> route3_road_pb2.ListGraphsResponse:
        return self.stub.ListGraphs(route3_road_pb2.Empty())

    def list_datasets(self) -> typing.List[str]:
        return self.stub.ListDatasets(route3_road_pb2.Empty()).dataset_name

    def differential_shortest_path(self, graph_handle: GraphHandle, disturbance_geom: BaseGeometry,
                                   radius_meters: float,
                                   destination_points: Iterable[Point],
                                   ref_dataset_name: str,
                                   num_destinations_to_reach: int = 3,
                                   num_gap_cells_to_graph: int = 1,
                                   downsampled_prerouting: bool = False,
                                   store_output: bool = True,
                                   ) -> Tuple[str, pd.DataFrame]:
        shortest_path_options = route3_road_pb2.ShortestPathOptions()
        shortest_path_options.num_destinations_to_reach = num_destinations_to_reach
        shortest_path_options.num_gap_cells_to_graph = num_gap_cells_to_graph

        req = route3_road_pb2.DifferentialShortestPathRequest()
        req.ref_dataset_name = ref_dataset_name
        req.graph_handle.MergeFrom(graph_handle)
        req.options.MergeFrom(shortest_path_options)
        req.disturbance_wkb_geometry = shapely.wkb.dumps(disturbance_geom)
        req.radius_meters = radius_meters
        req.downsampled_prerouting = downsampled_prerouting
        req.store_output = store_output

        for destination_point in destination_points:
            pt = req.destinations.add()
            pt.x = destination_point.x
            pt.y = destination_point.y

        return _arrowrecordbatch_to_dataframe(self.stub.DifferentialShortestPath(req))

    def get_differential_shortest_path(self, object_id: str) -> Tuple[str, pd.DataFrame]:
        req = route3_road_pb2.IdRef()
        req.object_id = object_id
        return _arrowrecordbatch_to_dataframe(self.stub.GetDifferentialShortestPath(req))

    def get_differential_shortest_path_routes(self, object_id: str, cells: Iterable[int]) -> gpd.GeoDataFrame:
        req = route3_road_pb2.DifferentialShortestPathRoutesRequest()
        req.object_id = object_id
        for cell in cells:
            req.cells.append(cell)

        response = self.stub.GetDifferentialShortestPathRoutes(req)

        geoms = []
        h3index_origin = []
        h3index_destination = []
        travel_duration_secs = []
        category_weight = []
        with_disturbance_list = []
        for stream_item in response:
            for with_disturbance, route_list in (
                    (1, stream_item.routes_with_disturbance), (0, stream_item.routes_without_disturbance)):
                for route in route_list:
                    h3index_origin.append(route.origin_cell)
                    h3index_destination.append(route.destination_cell)
                    with_disturbance_list.append(with_disturbance)
                    travel_duration_secs.append(route.travel_duration_secs)
                    category_weight.append(route.category_weight)
                    geoms.append(shapely.wkb.loads(route.wkb))

        gdf = gpd.GeoDataFrame({
            "geometry": geoms,
            "h3index_origin": np.asarray(h3index_origin, dtype=np.uint64),
            "h3index_destination": np.asarray(h3index_destination, dtype=np.uint64),
            "travel_duration_secs": np.asarray(travel_duration_secs, dtype=np.float64),
            "category_weight": np.asarray(category_weight, dtype=np.float64),
            "with_disturbance": np.asarray(with_disturbance_list, dtype=np.int),
        }, crs=4326)
        return gdf


def _arrowrecordbatch_to_dataframe(response: route3_road_pb2.ArrowRecordBatch) -> Tuple[str, pd.DataFrame]:
    object_id = None
    df = None
    batches = []
    for stream_item in response:
        if object_id is None:
            object_id = stream_item.object_id
        reader = pa.ipc.open_file(stream_item.data)
        for i in range(reader.num_record_batches):
            batches.append(reader.get_batch(i))
    if len(batches) > 0:
        df = pa.Table.from_batches(batches).to_pandas()
    return object_id, df


def make_graph_handle(name: str, h3_resolution: int) -> GraphHandle:
    gh = GraphHandle()
    gh.name = name
    gh.h3_resolution = h3_resolution
    return gh
