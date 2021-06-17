__version__ = '0.1.0'

from typing import Optional

import grpc
from shapely.geometry.base import BaseGeometry
import shapely.wkb

from . import route3_pb2
from .route3_pb2_grpc import Route3Stub

DEFAULT_PORT = 7088


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

    def analyze_disturbance(self, geom: BaseGeometry):
        req = route3_pb2.AnalyzeDisturbanceRequest()
        req.wkb_geometry = shapely.wkb.dumps(geom)
        self.stub.AnalyzeDisturbance(req)
