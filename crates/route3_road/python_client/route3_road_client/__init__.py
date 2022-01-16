__version__ = '0.2.1'

import typing

import grpc
import pyarrow as pa
import shapely.wkb
from shapely.geometry import Point
from shapely.geometry.base import BaseGeometry

from . import route3_road_pb2
from .route3_road_pb2 import GraphHandle, RouteWKB, RouteH3Indexes
from .route3_road_pb2_grpc import Route3RoadStub

DEFAULT_PORT = 7088
DEFAULT_HOST = "127.0.0.1"


class TableWithId:
    table: typing.Optional[pa.Table]
    id: str

    def __init__(self, id: str, table: typing.Optional[pa.Table]):
        self.id = id
        self.table = table


def cell_selection(cells: typing.Iterable[int], dataset_name: str = None) -> route3_road_pb2.CellSelection:
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


def build_h3_shortest_path_request(graph_handle: GraphHandle, origin_cells, destination_cells,
                                   num_destinations_to_reach: int = 3,
                                   num_gap_cells_to_graph: int = 1,
                                   smoothen_geometries: bool = False,
                                   ) -> route3_road_pb2.H3ShortestPathRequest:
    shortest_path_options = route3_road_pb2.ShortestPathOptions()
    shortest_path_options.num_destinations_to_reach = num_destinations_to_reach
    shortest_path_options.num_gap_cells_to_graph = num_gap_cells_to_graph

    request = route3_road_pb2.H3ShortestPathRequest()
    request.graph_handle.MergeFrom(graph_handle)
    request.options.MergeFrom(shortest_path_options)
    request.origins.MergeFrom(_to_cell_selection(origin_cells))
    request.destinations.MergeFrom(_to_cell_selection(destination_cells))
    request.smoothen_geometries = smoothen_geometries
    return request


def build_h3_within_threshold_request(graph_handle: GraphHandle, origin_cells,
                                      travel_duration_secs_threshold: float = 0.0
                                      ) -> route3_road_pb2.H3WithinThresholdRequest:
    request = route3_road_pb2.H3WithinThresholdRequest()
    request.graph_handle.MergeFrom(graph_handle)
    request.origins.MergeFrom(_to_cell_selection(origin_cells))
    request.travel_duration_secs_threshold = travel_duration_secs_threshold
    return request


def build_differential_shortest_path_request(graph_handle: GraphHandle, disturbance_geom: BaseGeometry,
                                             radius_meters: float,
                                             destination_points: typing.Iterable[Point],
                                             ref_dataset_name: str,
                                             num_destinations_to_reach: int = 3,
                                             num_gap_cells_to_graph: int = 1,
                                             downsampled_prerouting: bool = False,
                                             store_output: bool = True,
                                             ) -> route3_road_pb2.DifferentialShortestPathRequest:
    shortest_path_options = route3_road_pb2.ShortestPathOptions()
    shortest_path_options.num_destinations_to_reach = num_destinations_to_reach
    shortest_path_options.num_gap_cells_to_graph = num_gap_cells_to_graph

    request = route3_road_pb2.DifferentialShortestPathRequest()
    request.ref_dataset_name = ref_dataset_name
    request.graph_handle.MergeFrom(graph_handle)
    request.options.MergeFrom(shortest_path_options)
    request.disturbance_wkb_geometry = shapely.wkb.dumps(disturbance_geom)
    request.radius_meters = radius_meters
    request.downsampled_prerouting = downsampled_prerouting
    request.store_output = store_output

    for destination_point in destination_points:
        pt = request.destinations.add()
        pt.x = destination_point.x
        pt.y = destination_point.y
    return request


def build_differential_shortest_path_routes_request(object_id: str, cells: typing.Iterable[
    int], smoothen_geometries: bool = False) -> route3_road_pb2.DifferentialShortestPathRoutesRequest:
    request = route3_road_pb2.DifferentialShortestPathRoutesRequest()
    request.object_id = object_id
    request.smoothen_geometries = smoothen_geometries
    for cell in cells:
        request.cells.append(cell)
    return request


class Server:
    channel = None
    stub = None

    def __init__(self, hostport: str = f"{DEFAULT_HOST}:{DEFAULT_PORT}",
                 credentials: typing.Optional[grpc.ChannelCredentials] = None,
                 grpc_options: typing.Any = None):
        compression = grpc.Compression.Gzip
        if credentials is not None:
            self.channel = grpc.secure_channel(hostport, credentials, compression=compression, options=grpc_options)
        else:
            self.channel = grpc.insecure_channel(hostport, compression=compression, options=grpc_options)
        self.stub = Route3RoadStub(self.channel)

    def version(self) -> route3_road_pb2.VersionResponse:
        return self.stub.Version(route3_road_pb2.Empty())

    def list_graphs(self) -> route3_road_pb2.ListGraphsResponse:
        return self.stub.ListGraphs(route3_road_pb2.Empty())

    def list_datasets(self) -> typing.List[str]:
        return self.stub.ListDatasets(route3_road_pb2.Empty()).dataset_name

    def h3_shortest_path(self, request: route3_road_pb2.H3ShortestPathRequest) -> TableWithId:
        return _arrowrecordbatch_to_table(self.stub.H3ShortestPath(request))

    def h3_shortest_path_routes(self, request: route3_road_pb2.H3ShortestPathRequest) -> typing.Generator[
        RouteWKB, None, None]:
        """generator to yield the calculated routes as RouteWKB objects"""
        for route in self.stub.H3ShortestPathRoutes(request):
            yield route

    def h3_shortest_path_cells(self, request: route3_road_pb2.H3ShortestPathRequest) -> typing.Generator[
        RouteH3Indexes, None, None]:
        """generator to yield the calculated routes as cells in RouteH3Indexes objects"""
        for route in self.stub.H3ShortestPathCells(request):
            yield route

    def h3_shortest_path_edges(self, request: route3_road_pb2.H3ShortestPathRequest) -> typing.Generator[
        RouteH3Indexes, None, None]:
        """generator to yield the calculated routes as edges in RouteH3Indexes objects"""
        for route in self.stub.H3ShortestPathEdges(request):
            yield route

    def h3_shortest_path_linestrings(self, request: route3_road_pb2.H3ShortestPathRequest) -> "GeoDataFrame":
        """returns a geodataframe of the calculates routes. Routes are returned as
        linestring geometries."""
        return _h3_shortest_path_linestrings_gdf(self.h3_shortest_path_routes(request))

    def h3_cells_within_threshold(self, request: route3_road_pb2.H3WithinThresholdRequest) -> TableWithId:
        """graph cells with in a certain threshold of origin cells"""
        return _arrowrecordbatch_to_table(self.stub.H3CellsWithinThreshold(request))

    def differential_shortest_path(self, request: route3_road_pb2.DifferentialShortestPathRequest) -> TableWithId:
        return _arrowrecordbatch_to_table(self.stub.DifferentialShortestPath(request))

    def get_differential_shortest_path(self, object_id: str) -> TableWithId:
        req = route3_road_pb2.IdRef()
        req.object_id = object_id
        return _arrowrecordbatch_to_table(self.stub.GetDifferentialShortestPath(req))

    def get_differential_shortest_path_routes(self, object_id: str, cells: typing.Iterable[int],
                                              smoothen_geometries: bool = False) -> "GeoDataFrame":
        """returns a `GeoDataframe` containing the linestrings of the routes originating from the given cells.

        requires geopandas
        """
        response = self.stub.GetDifferentialShortestPathRoutes(
            build_differential_shortest_path_routes_request(object_id, cells, smoothen_geometries=smoothen_geometries))
        return _get_differential_shortest_path_routes_gdf(response)


def _arrowrecordbatch_to_table(response: route3_road_pb2.ArrowRecordBatch) -> TableWithId:
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
    return TableWithId(object_id, table)


def build_graph_handle(name: str, h3_resolution: int) -> GraphHandle:
    gh = GraphHandle()
    gh.name = name
    gh.h3_resolution = h3_resolution
    return gh


def _h3_shortest_path_linestrings_gdf(gen: typing.Generator[RouteWKB, None, None]) -> "GeoDataFrame":
    from geopandas import GeoDataFrame
    import numpy as np

    geoms = []
    h3index_origin = []
    h3index_destination = []
    travel_duration_secs = []
    category_weight = []
    path_length_m = []

    for route in gen:
        h3index_origin.append(route.origin_cell)
        h3index_destination.append(route.destination_cell)
        travel_duration_secs.append(route.travel_duration_secs)
        category_weight.append(route.category_weight)
        path_length_m.append(route.path_length_m)
        geoms.append(shapely.wkb.loads(route.wkb))

    gdf = GeoDataFrame({
        "geometry": geoms,
        "h3index_origin": np.asarray(h3index_origin, dtype=np.uint64),
        "h3index_destination": np.asarray(h3index_destination, dtype=np.uint64),
        "travel_duration_secs": np.asarray(travel_duration_secs, dtype=np.float64),
        "category_weight": np.asarray(category_weight, dtype=np.float64),
        "path_length_m": np.asarray(path_length_m, dtype=np.float64),
    }, crs=4326)
    return gdf


def _get_differential_shortest_path_routes_gdf(
        response: route3_road_pb2.DifferentialShortestPathRoutes) -> "GeoDataFrame":
    from geopandas import GeoDataFrame
    import numpy as np

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

    gdf = GeoDataFrame({
        "geometry": geoms,
        "h3index_origin": np.asarray(h3index_origin, dtype=np.uint64),
        "h3index_destination": np.asarray(h3index_destination, dtype=np.uint64),
        "travel_duration_secs": np.asarray(travel_duration_secs, dtype=np.float64),
        "category_weight": np.asarray(category_weight, dtype=np.float64),
        "with_disturbance": np.asarray(with_disturbance_list, dtype=np.int),
    }, crs=4326)
    return gdf
