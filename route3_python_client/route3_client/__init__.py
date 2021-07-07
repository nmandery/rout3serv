__version__ = '0.1.0'

from typing import Optional, Iterable

import grpc
from shapely.geometry.base import BaseGeometry
from shapely.geometry import Point
import pyarrow as pa
import pandas as pd
import shapely.wkb

from . import route3_pb2
from .route3_pb2 import DisturbanceOfPopulationMovementResponse
from .route3_pb2_grpc import Route3Stub

DEFAULT_PORT = 7088


class DisturbanceOfPopulationMovementStats:
    id: str
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

    def _return_stats(self, response: DisturbanceOfPopulationMovementResponse) -> DisturbanceOfPopulationMovementStats:
        stats = DisturbanceOfPopulationMovementStats()
        stats.id = response.id
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

    def get_disturbance_of_population_movement(self, id: str) -> DisturbanceOfPopulationMovementStats:
        req = route3_pb2.GetDisturbanceOfPopulationMovementRequest()
        req.id = id
        response = self.stub.GetDisturbanceOfPopulationMovement(req)
        return self._return_stats(response)
