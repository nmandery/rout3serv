__version__ = '0.2.0'

import typing
from typing import Optional, Iterable, Tuple

import grpc
import numpy as np
import pyarrow as pa
import shapely.wkb
from shapely.geometry import Point
from shapely.geometry.base import BaseGeometry

from . import route3_road_pb2
from .route3_road_pb2 import GraphHandle
from .route3_road_pb2_grpc import Route3RoadStub

DEFAULT_PORT = 7088


def cell_selection(cells: Iterable[int], dataset_name: str = None) -> route3_road_pb2.CellSelection:
    cs = route3_road_pb2.CellSelection()
    if dataset_name is not None:
        cs.dataset_name = dataset_name
    for cell in cells:
        cs.cells.append(cell)
    return cs


def _to_cell_selection(arg, **kwargs) -> route3_road_pb2.CellSelection:
    if isinstance(arg, route3_road_pb2.CellSelection):
        return arg
    dataset_name = kwargs.get("dataset_name")
    if hasattr(arg, '__iter__'):
        return cell_selection(arg, dataset_name=dataset_name)
    raise Exception("unsupported type for cell_selection")


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

    def h3_shortest_path(self, graph_handle: GraphHandle, origin_cells, destination_cells,
                         num_destinations_to_reach: int = 3,
                         num_gap_cells_to_graph: int = 1,
                         ) -> Tuple[str, Optional[pa.Table]]:
        shortest_path_options = route3_road_pb2.ShortestPathOptions()
        shortest_path_options.num_destinations_to_reach = num_destinations_to_reach
        shortest_path_options.num_gap_cells_to_graph = num_gap_cells_to_graph

        req = route3_road_pb2.H3ShortestPathRequest()
        req.graph_handle.MergeFrom(graph_handle)
        req.options.MergeFrom(shortest_path_options)
        req.origins.MergeFrom(_to_cell_selection(origin_cells))
        req.destinations.MergeFrom(_to_cell_selection(destination_cells))
        return _arrowrecordbatch_to_table(self.stub.H3ShortestPath(req))

    def differential_shortest_path(self, graph_handle: GraphHandle, disturbance_geom: BaseGeometry,
                                   radius_meters: float,
                                   destination_points: Iterable[Point],
                                   ref_dataset_name: str,
                                   num_destinations_to_reach: int = 3,
                                   num_gap_cells_to_graph: int = 1,
                                   downsampled_prerouting: bool = False,
                                   store_output: bool = True,
                                   ) -> Tuple[str, Optional[pa.Table]]:
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

        return _arrowrecordbatch_to_table(self.stub.DifferentialShortestPath(req))

    def get_differential_shortest_path(self, object_id: str) -> Tuple[str, Optional[pa.Table]]:
        req = route3_road_pb2.IdRef()
        req.object_id = object_id
        return _arrowrecordbatch_to_table(self.stub.GetDifferentialShortestPath(req))

    def get_differential_shortest_path_routes(self, object_id: str, cells: Iterable[int]):
        """returns a `GeoDataframe` containing the linestrings of the routes originating from the given cells.

        requires geopandas
        """
        import geopandas as gpd

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


def _arrowrecordbatch_to_table(response: route3_road_pb2.ArrowRecordBatch) -> Tuple[str, Optional[pa.Table]]:
    """convert a streamed ArrowRecordBatch response to a pyarrow.Table"""
    object_id = None
    table = None
    batches = []
    for stream_item in response:
        if object_id is None:
            object_id = stream_item.object_id
        reader = pa.ipc.open_file(stream_item.data)
        for i in range(reader.num_record_batches):
            batches.append(reader.get_batch(i))
    if len(batches) > 0:
        table = pa.Table.from_batches(batches)
    return object_id, table


def make_graph_handle(name: str, h3_resolution: int) -> GraphHandle:
    gh = GraphHandle()
    gh.name = name
    gh.h3_resolution = h3_resolution
    return gh
