__version__ = '0.1.0'

from typing import Optional, Iterable

import geopandas as gpd
import grpc
import pandas as pd
import pyarrow as pa
import numpy as np
import shapely.wkb
from shapely.geometry import Point
from shapely.geometry.base import BaseGeometry

from . import route3_pb2
from .route3_pb2_grpc import Route3Stub

DEFAULT_PORT = 7088


class DisturbanceOfPopulationMovementStats:
    dopm_id: str
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
        self.stub = Route3Stub(self.channel)

    def server_version(self) -> str:
        return self.stub.Version(route3_pb2.VersionRequest()).version

    def _return_stats(self, response: route3_pb2.DisturbanceOfPopulationMovementResponse) -> DisturbanceOfPopulationMovementStats:
        stats = DisturbanceOfPopulationMovementStats()
        stats.dopm_id = response.dopm_id
        stats.population_within_disturbance = response.stats.population_within_disturbance
        stats.dataframe = pa.ipc.open_file(response.stats.recordbatch).read_pandas()
        return stats

    def analyze_disturbance_of_population_movement(self, disturbance_geom: BaseGeometry, radius_meters: float,
                                                   destination_points: Iterable[Point],
                                                   num_destinations_to_reach: int = 3) -> DisturbanceOfPopulationMovementStats:
        req = route3_pb2.DisturbanceOfPopulationMovementRequest()
        req.disturbance_wkb_geometry = shapely.wkb.dumps(disturbance_geom)
        req.radius_meters = radius_meters
        req.num_destinations_to_reach = num_destinations_to_reach

        for destination_point in destination_points:
            pt = req.destinations.add()
            pt.x = destination_point.x
            pt.y = destination_point.y

        response = self.stub.AnalyzeDisturbanceOfPopulationMovement(req)
        return self._return_stats(response)

    def get_disturbance_of_population_movement(self, dopm_id: str) -> route3_pb2.DisturbanceOfPopulationMovementStats:
        req = route3_pb2.GetDisturbanceOfPopulationMovementRequest()
        req.dopm_id = dopm_id
        response = self.stub.GetDisturbanceOfPopulationMovement(req)
        return self._return_stats(response)

    def get_disturbance_of_population_movement_routes(self, dopm_id: str, cells: Iterable[int]) -> gpd.GeoDataFrame:
        req = route3_pb2.DisturbanceOfPopulationMovementRoutesRequest()
        req.dopm_id = dopm_id
        for cell in cells:
            req.cells.append(cell)

        response = self.stub.GetDisturbanceOfPopulationMovementRoutes(req)

        geoms = []
        h3index_origin = []
        h3index_destination = []
        cost = []
        with_disturbance_list = []
        for stream_item in response:
            for with_disturbance, route_list in (
            (1, stream_item.routes_with_disturbance), (0, stream_item.routes_without_disturbance)):
                for route in route_list:
                    h3index_origin.append(route.origin_cell)
                    h3index_destination.append(route.destination_cell)
                    with_disturbance_list.append(with_disturbance)
                    cost.append(route.cost)
                    geoms.append(shapely.wkb.loads(route.wkb))

        gdf = gpd.GeoDataFrame({
            "geometry": geoms,
            "h3index_origin": np.asarray(h3index_origin, dtype=np.uint64),
            "h3index_destination": np.asarray(h3index_destination, dtype=np.uint64),
            "cost": np.asarray(cost, dtype=np.float64),
            "with_disturbance": np.asarray(with_disturbance_list, dtype=np.int),
        }, crs=4326)
        return gdf
