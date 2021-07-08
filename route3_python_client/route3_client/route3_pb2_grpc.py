# Generated by the gRPC Python protocol compiler plugin. DO NOT EDIT!
"""Client and server classes corresponding to protobuf-defined services."""
import grpc

from . import route3_pb2 as route3__pb2


class Route3Stub(object):
    """Missing associated documentation comment in .proto file."""

    def __init__(self, channel):
        """Constructor.

        Args:
            channel: A grpc.Channel.
        """
        self.Version = channel.unary_unary(
                '/grpc.route3.Route3/Version',
                request_serializer=route3__pb2.VersionRequest.SerializeToString,
                response_deserializer=route3__pb2.VersionResponse.FromString,
                )
        self.AnalyzeDisturbanceOfPopulationMovement = channel.unary_unary(
                '/grpc.route3.Route3/AnalyzeDisturbanceOfPopulationMovement',
                request_serializer=route3__pb2.DisturbanceOfPopulationMovementRequest.SerializeToString,
                response_deserializer=route3__pb2.DisturbanceOfPopulationMovementResponse.FromString,
                )
        self.GetDisturbanceOfPopulationMovement = channel.unary_unary(
                '/grpc.route3.Route3/GetDisturbanceOfPopulationMovement',
                request_serializer=route3__pb2.GetDisturbanceOfPopulationMovementRequest.SerializeToString,
                response_deserializer=route3__pb2.DisturbanceOfPopulationMovementResponse.FromString,
                )
        self.GetDisturbanceOfPopulationMovementRoutes = channel.unary_stream(
                '/grpc.route3.Route3/GetDisturbanceOfPopulationMovementRoutes',
                request_serializer=route3__pb2.DisturbanceOfPopulationMovementRoutesRequest.SerializeToString,
                response_deserializer=route3__pb2.DisturbanceOfPopulationMovementRoutes.FromString,
                )


class Route3Servicer(object):
    """Missing associated documentation comment in .proto file."""

    def Version(self, request, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details('Method not implemented!')
        raise NotImplementedError('Method not implemented!')

    def AnalyzeDisturbanceOfPopulationMovement(self, request, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details('Method not implemented!')
        raise NotImplementedError('Method not implemented!')

    def GetDisturbanceOfPopulationMovement(self, request, context):
        """get an already computed analysis 
        """
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details('Method not implemented!')
        raise NotImplementedError('Method not implemented!')

    def GetDisturbanceOfPopulationMovementRoutes(self, request, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details('Method not implemented!')
        raise NotImplementedError('Method not implemented!')


def add_Route3Servicer_to_server(servicer, server):
    rpc_method_handlers = {
            'Version': grpc.unary_unary_rpc_method_handler(
                    servicer.Version,
                    request_deserializer=route3__pb2.VersionRequest.FromString,
                    response_serializer=route3__pb2.VersionResponse.SerializeToString,
            ),
            'AnalyzeDisturbanceOfPopulationMovement': grpc.unary_unary_rpc_method_handler(
                    servicer.AnalyzeDisturbanceOfPopulationMovement,
                    request_deserializer=route3__pb2.DisturbanceOfPopulationMovementRequest.FromString,
                    response_serializer=route3__pb2.DisturbanceOfPopulationMovementResponse.SerializeToString,
            ),
            'GetDisturbanceOfPopulationMovement': grpc.unary_unary_rpc_method_handler(
                    servicer.GetDisturbanceOfPopulationMovement,
                    request_deserializer=route3__pb2.GetDisturbanceOfPopulationMovementRequest.FromString,
                    response_serializer=route3__pb2.DisturbanceOfPopulationMovementResponse.SerializeToString,
            ),
            'GetDisturbanceOfPopulationMovementRoutes': grpc.unary_stream_rpc_method_handler(
                    servicer.GetDisturbanceOfPopulationMovementRoutes,
                    request_deserializer=route3__pb2.DisturbanceOfPopulationMovementRoutesRequest.FromString,
                    response_serializer=route3__pb2.DisturbanceOfPopulationMovementRoutes.SerializeToString,
            ),
    }
    generic_handler = grpc.method_handlers_generic_handler(
            'grpc.route3.Route3', rpc_method_handlers)
    server.add_generic_rpc_handlers((generic_handler,))


 # This class is part of an EXPERIMENTAL API.
class Route3(object):
    """Missing associated documentation comment in .proto file."""

    @staticmethod
    def Version(request,
            target,
            options=(),
            channel_credentials=None,
            call_credentials=None,
            insecure=False,
            compression=None,
            wait_for_ready=None,
            timeout=None,
            metadata=None):
        return grpc.experimental.unary_unary(request, target, '/grpc.route3.Route3/Version',
            route3__pb2.VersionRequest.SerializeToString,
            route3__pb2.VersionResponse.FromString,
            options, channel_credentials,
            insecure, call_credentials, compression, wait_for_ready, timeout, metadata)

    @staticmethod
    def AnalyzeDisturbanceOfPopulationMovement(request,
            target,
            options=(),
            channel_credentials=None,
            call_credentials=None,
            insecure=False,
            compression=None,
            wait_for_ready=None,
            timeout=None,
            metadata=None):
        return grpc.experimental.unary_unary(request, target, '/grpc.route3.Route3/AnalyzeDisturbanceOfPopulationMovement',
            route3__pb2.DisturbanceOfPopulationMovementRequest.SerializeToString,
            route3__pb2.DisturbanceOfPopulationMovementResponse.FromString,
            options, channel_credentials,
            insecure, call_credentials, compression, wait_for_ready, timeout, metadata)

    @staticmethod
    def GetDisturbanceOfPopulationMovement(request,
            target,
            options=(),
            channel_credentials=None,
            call_credentials=None,
            insecure=False,
            compression=None,
            wait_for_ready=None,
            timeout=None,
            metadata=None):
        return grpc.experimental.unary_unary(request, target, '/grpc.route3.Route3/GetDisturbanceOfPopulationMovement',
            route3__pb2.GetDisturbanceOfPopulationMovementRequest.SerializeToString,
            route3__pb2.DisturbanceOfPopulationMovementResponse.FromString,
            options, channel_credentials,
            insecure, call_credentials, compression, wait_for_ready, timeout, metadata)

    @staticmethod
    def GetDisturbanceOfPopulationMovementRoutes(request,
            target,
            options=(),
            channel_credentials=None,
            call_credentials=None,
            insecure=False,
            compression=None,
            wait_for_ready=None,
            timeout=None,
            metadata=None):
        return grpc.experimental.unary_stream(request, target, '/grpc.route3.Route3/GetDisturbanceOfPopulationMovementRoutes',
            route3__pb2.DisturbanceOfPopulationMovementRoutesRequest.SerializeToString,
            route3__pb2.DisturbanceOfPopulationMovementRoutes.FromString,
            options, channel_credentials,
            insecure, call_credentials, compression, wait_for_ready, timeout, metadata)
