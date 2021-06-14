__version__ = '0.1.0'

import grpc

from . import route3_pb2
from .route3_pb2_grpc import Route3Stub


class Server:
    channel = None
    stub = None

    def __init__(self, hostport: str = "0.0.0.0:7000"):
        self.channel = grpc.insecure_channel(hostport)
        self.stub = Route3Stub(self.channel)

    def version(self) -> str:
        return self.stub.Version(route3_pb2.VersionRequest()).version
