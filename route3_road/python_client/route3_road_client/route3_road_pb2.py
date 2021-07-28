# -*- coding: utf-8 -*-
# Generated by the protocol buffer compiler.  DO NOT EDIT!
# source: route3_road.proto
"""Generated protocol buffer code."""
from google.protobuf.internal import enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from google.protobuf import reflection as _reflection
from google.protobuf import symbol_database as _symbol_database
# @@protoc_insertion_point(imports)

_sym_db = _symbol_database.Default()




DESCRIPTOR = _descriptor.FileDescriptor(
  name='route3_road.proto',
  package='route3.road',
  syntax='proto3',
  serialized_options=None,
  create_key=_descriptor._internal_create_key,
  serialized_pb=b'\n\x11route3_road.proto\x12\x0broute3.road\"\x07\n\x05\x45mpty\"S\n\x0fVersionResponse\x12\x0f\n\x07version\x18\x01 \x01(\t\x12\x16\n\x0egit_commit_sha\x18\x02 \x01(\t\x12\x17\n\x0f\x62uild_timestamp\x18\x03 \x01(\t\"\x1d\n\x05Point\x12\t\n\x01x\x18\x01 \x01(\x01\x12\t\n\x01y\x18\x02 \x01(\x01\"\xee\x01\n&DisturbanceOfPopulationMovementRequest\x12 \n\x18\x64isturbance_wkb_geometry\x18\x01 \x01(\x0c\x12\x15\n\rradius_meters\x18\x02 \x01(\x01\x12!\n\x19num_destinations_to_reach\x18\x03 \x01(\r\x12(\n\x0c\x64\x65stinations\x18\x04 \x03(\x0b\x32\x12.route3.road.Point\x12\x1e\n\x16num_gap_cells_to_graph\x18\x05 \x01(\r\x12\x1e\n\x16\x64ownsampled_prerouting\x18\x06 \x01(\x08\"\x13\n\x05IdRef\x12\n\n\x02id\x18\x01 \x01(\t\"N\n,DisturbanceOfPopulationMovementRoutesRequest\x12\x0f\n\x07\x64opm_id\x18\x01 \x01(\t\x12\r\n\x05\x63\x65lls\x18\x03 \x03(\x04\"T\n\x08RouteWKB\x12\x13\n\x0borigin_cell\x18\x01 \x01(\x04\x12\x18\n\x10\x64\x65stination_cell\x18\x02 \x01(\x04\x12\x0c\n\x04\x63ost\x18\x03 \x01(\x01\x12\x0b\n\x03wkb\x18\x04 \x01(\x0c\"3\n\x10\x41rrowRecordBatch\x12\x11\n\tobject_id\x18\x01 \x01(\t\x12\x0c\n\x04\x64\x61ta\x18\x02 \x01(\x0c\"\x9a\x01\n%DisturbanceOfPopulationMovementRoutes\x12\x39\n\x1aroutes_without_disturbance\x18\x02 \x03(\x0b\x32\x15.route3.road.RouteWKB\x12\x36\n\x17routes_with_disturbance\x18\x03 \x03(\x0b\x32\x15.route3.road.RouteWKB\"=\n\x11GraphInfoResponse\x12\x15\n\rh3_resolution\x18\x01 \x01(\r\x12\x11\n\tnum_edges\x18\x02 \x01(\x04*2\n\x13\x43\x65llInRoutePosition\x12\n\n\x06Origin\x10\x00\x12\x0f\n\x0b\x44\x65stination\x10\x01\x32\x8e\x04\n\nRoute3Road\x12=\n\x07Version\x12\x12.route3.road.Empty\x1a\x1c.route3.road.VersionResponse\"\x00\x12\x41\n\tGraphInfo\x12\x12.route3.road.Empty\x1a\x1e.route3.road.GraphInfoResponse\"\x00\x12\x80\x01\n&AnalyzeDisturbanceOfPopulationMovement\x12\x33.route3.road.DisturbanceOfPopulationMovementRequest\x1a\x1d.route3.road.ArrowRecordBatch\"\x00\x30\x01\x12[\n\"GetDisturbanceOfPopulationMovement\x12\x12.route3.road.IdRef\x1a\x1d.route3.road.ArrowRecordBatch\"\x00\x30\x01\x12\x9d\x01\n(GetDisturbanceOfPopulationMovementRoutes\x12\x39.route3.road.DisturbanceOfPopulationMovementRoutesRequest\x1a\x32.route3.road.DisturbanceOfPopulationMovementRoutes\"\x00\x30\x01\x62\x06proto3'
)

_CELLINROUTEPOSITION = _descriptor.EnumDescriptor(
  name='CellInRoutePosition',
  full_name='route3.road.CellInRoutePosition',
  filename=None,
  file=DESCRIPTOR,
  create_key=_descriptor._internal_create_key,
  values=[
    _descriptor.EnumValueDescriptor(
      name='Origin', index=0, number=0,
      serialized_options=None,
      type=None,
      create_key=_descriptor._internal_create_key),
    _descriptor.EnumValueDescriptor(
      name='Destination', index=1, number=1,
      serialized_options=None,
      type=None,
      create_key=_descriptor._internal_create_key),
  ],
  containing_type=None,
  serialized_options=None,
  serialized_start=860,
  serialized_end=910,
)
_sym_db.RegisterEnumDescriptor(_CELLINROUTEPOSITION)

CellInRoutePosition = enum_type_wrapper.EnumTypeWrapper(_CELLINROUTEPOSITION)
Origin = 0
Destination = 1



_EMPTY = _descriptor.Descriptor(
  name='Empty',
  full_name='route3.road.Empty',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
  ],
  extensions=[
  ],
  nested_types=[],
  enum_types=[
  ],
  serialized_options=None,
  is_extendable=False,
  syntax='proto3',
  extension_ranges=[],
  oneofs=[
  ],
  serialized_start=34,
  serialized_end=41,
)


_VERSIONRESPONSE = _descriptor.Descriptor(
  name='VersionResponse',
  full_name='route3.road.VersionResponse',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='version', full_name='route3.road.VersionResponse.version', index=0,
      number=1, type=9, cpp_type=9, label=1,
      has_default_value=False, default_value=b"".decode('utf-8'),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='git_commit_sha', full_name='route3.road.VersionResponse.git_commit_sha', index=1,
      number=2, type=9, cpp_type=9, label=1,
      has_default_value=False, default_value=b"".decode('utf-8'),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='build_timestamp', full_name='route3.road.VersionResponse.build_timestamp', index=2,
      number=3, type=9, cpp_type=9, label=1,
      has_default_value=False, default_value=b"".decode('utf-8'),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
  ],
  extensions=[
  ],
  nested_types=[],
  enum_types=[
  ],
  serialized_options=None,
  is_extendable=False,
  syntax='proto3',
  extension_ranges=[],
  oneofs=[
  ],
  serialized_start=43,
  serialized_end=126,
)


_POINT = _descriptor.Descriptor(
  name='Point',
  full_name='route3.road.Point',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='x', full_name='route3.road.Point.x', index=0,
      number=1, type=1, cpp_type=5, label=1,
      has_default_value=False, default_value=float(0),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='y', full_name='route3.road.Point.y', index=1,
      number=2, type=1, cpp_type=5, label=1,
      has_default_value=False, default_value=float(0),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
  ],
  extensions=[
  ],
  nested_types=[],
  enum_types=[
  ],
  serialized_options=None,
  is_extendable=False,
  syntax='proto3',
  extension_ranges=[],
  oneofs=[
  ],
  serialized_start=128,
  serialized_end=157,
)


_DISTURBANCEOFPOPULATIONMOVEMENTREQUEST = _descriptor.Descriptor(
  name='DisturbanceOfPopulationMovementRequest',
  full_name='route3.road.DisturbanceOfPopulationMovementRequest',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='disturbance_wkb_geometry', full_name='route3.road.DisturbanceOfPopulationMovementRequest.disturbance_wkb_geometry', index=0,
      number=1, type=12, cpp_type=9, label=1,
      has_default_value=False, default_value=b"",
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='radius_meters', full_name='route3.road.DisturbanceOfPopulationMovementRequest.radius_meters', index=1,
      number=2, type=1, cpp_type=5, label=1,
      has_default_value=False, default_value=float(0),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='num_destinations_to_reach', full_name='route3.road.DisturbanceOfPopulationMovementRequest.num_destinations_to_reach', index=2,
      number=3, type=13, cpp_type=3, label=1,
      has_default_value=False, default_value=0,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='destinations', full_name='route3.road.DisturbanceOfPopulationMovementRequest.destinations', index=3,
      number=4, type=11, cpp_type=10, label=3,
      has_default_value=False, default_value=[],
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='num_gap_cells_to_graph', full_name='route3.road.DisturbanceOfPopulationMovementRequest.num_gap_cells_to_graph', index=4,
      number=5, type=13, cpp_type=3, label=1,
      has_default_value=False, default_value=0,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='downsampled_prerouting', full_name='route3.road.DisturbanceOfPopulationMovementRequest.downsampled_prerouting', index=5,
      number=6, type=8, cpp_type=7, label=1,
      has_default_value=False, default_value=False,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
  ],
  extensions=[
  ],
  nested_types=[],
  enum_types=[
  ],
  serialized_options=None,
  is_extendable=False,
  syntax='proto3',
  extension_ranges=[],
  oneofs=[
  ],
  serialized_start=160,
  serialized_end=398,
)


_IDREF = _descriptor.Descriptor(
  name='IdRef',
  full_name='route3.road.IdRef',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='id', full_name='route3.road.IdRef.id', index=0,
      number=1, type=9, cpp_type=9, label=1,
      has_default_value=False, default_value=b"".decode('utf-8'),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
  ],
  extensions=[
  ],
  nested_types=[],
  enum_types=[
  ],
  serialized_options=None,
  is_extendable=False,
  syntax='proto3',
  extension_ranges=[],
  oneofs=[
  ],
  serialized_start=400,
  serialized_end=419,
)


_DISTURBANCEOFPOPULATIONMOVEMENTROUTESREQUEST = _descriptor.Descriptor(
  name='DisturbanceOfPopulationMovementRoutesRequest',
  full_name='route3.road.DisturbanceOfPopulationMovementRoutesRequest',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='dopm_id', full_name='route3.road.DisturbanceOfPopulationMovementRoutesRequest.dopm_id', index=0,
      number=1, type=9, cpp_type=9, label=1,
      has_default_value=False, default_value=b"".decode('utf-8'),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='cells', full_name='route3.road.DisturbanceOfPopulationMovementRoutesRequest.cells', index=1,
      number=3, type=4, cpp_type=4, label=3,
      has_default_value=False, default_value=[],
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
  ],
  extensions=[
  ],
  nested_types=[],
  enum_types=[
  ],
  serialized_options=None,
  is_extendable=False,
  syntax='proto3',
  extension_ranges=[],
  oneofs=[
  ],
  serialized_start=421,
  serialized_end=499,
)


_ROUTEWKB = _descriptor.Descriptor(
  name='RouteWKB',
  full_name='route3.road.RouteWKB',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='origin_cell', full_name='route3.road.RouteWKB.origin_cell', index=0,
      number=1, type=4, cpp_type=4, label=1,
      has_default_value=False, default_value=0,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='destination_cell', full_name='route3.road.RouteWKB.destination_cell', index=1,
      number=2, type=4, cpp_type=4, label=1,
      has_default_value=False, default_value=0,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='cost', full_name='route3.road.RouteWKB.cost', index=2,
      number=3, type=1, cpp_type=5, label=1,
      has_default_value=False, default_value=float(0),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='wkb', full_name='route3.road.RouteWKB.wkb', index=3,
      number=4, type=12, cpp_type=9, label=1,
      has_default_value=False, default_value=b"",
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
  ],
  extensions=[
  ],
  nested_types=[],
  enum_types=[
  ],
  serialized_options=None,
  is_extendable=False,
  syntax='proto3',
  extension_ranges=[],
  oneofs=[
  ],
  serialized_start=501,
  serialized_end=585,
)


_ARROWRECORDBATCH = _descriptor.Descriptor(
  name='ArrowRecordBatch',
  full_name='route3.road.ArrowRecordBatch',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='object_id', full_name='route3.road.ArrowRecordBatch.object_id', index=0,
      number=1, type=9, cpp_type=9, label=1,
      has_default_value=False, default_value=b"".decode('utf-8'),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='data', full_name='route3.road.ArrowRecordBatch.data', index=1,
      number=2, type=12, cpp_type=9, label=1,
      has_default_value=False, default_value=b"",
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
  ],
  extensions=[
  ],
  nested_types=[],
  enum_types=[
  ],
  serialized_options=None,
  is_extendable=False,
  syntax='proto3',
  extension_ranges=[],
  oneofs=[
  ],
  serialized_start=587,
  serialized_end=638,
)


_DISTURBANCEOFPOPULATIONMOVEMENTROUTES = _descriptor.Descriptor(
  name='DisturbanceOfPopulationMovementRoutes',
  full_name='route3.road.DisturbanceOfPopulationMovementRoutes',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='routes_without_disturbance', full_name='route3.road.DisturbanceOfPopulationMovementRoutes.routes_without_disturbance', index=0,
      number=2, type=11, cpp_type=10, label=3,
      has_default_value=False, default_value=[],
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='routes_with_disturbance', full_name='route3.road.DisturbanceOfPopulationMovementRoutes.routes_with_disturbance', index=1,
      number=3, type=11, cpp_type=10, label=3,
      has_default_value=False, default_value=[],
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
  ],
  extensions=[
  ],
  nested_types=[],
  enum_types=[
  ],
  serialized_options=None,
  is_extendable=False,
  syntax='proto3',
  extension_ranges=[],
  oneofs=[
  ],
  serialized_start=641,
  serialized_end=795,
)


_GRAPHINFORESPONSE = _descriptor.Descriptor(
  name='GraphInfoResponse',
  full_name='route3.road.GraphInfoResponse',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='h3_resolution', full_name='route3.road.GraphInfoResponse.h3_resolution', index=0,
      number=1, type=13, cpp_type=3, label=1,
      has_default_value=False, default_value=0,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='num_edges', full_name='route3.road.GraphInfoResponse.num_edges', index=1,
      number=2, type=4, cpp_type=4, label=1,
      has_default_value=False, default_value=0,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
  ],
  extensions=[
  ],
  nested_types=[],
  enum_types=[
  ],
  serialized_options=None,
  is_extendable=False,
  syntax='proto3',
  extension_ranges=[],
  oneofs=[
  ],
  serialized_start=797,
  serialized_end=858,
)

_DISTURBANCEOFPOPULATIONMOVEMENTREQUEST.fields_by_name['destinations'].message_type = _POINT
_DISTURBANCEOFPOPULATIONMOVEMENTROUTES.fields_by_name['routes_without_disturbance'].message_type = _ROUTEWKB
_DISTURBANCEOFPOPULATIONMOVEMENTROUTES.fields_by_name['routes_with_disturbance'].message_type = _ROUTEWKB
DESCRIPTOR.message_types_by_name['Empty'] = _EMPTY
DESCRIPTOR.message_types_by_name['VersionResponse'] = _VERSIONRESPONSE
DESCRIPTOR.message_types_by_name['Point'] = _POINT
DESCRIPTOR.message_types_by_name['DisturbanceOfPopulationMovementRequest'] = _DISTURBANCEOFPOPULATIONMOVEMENTREQUEST
DESCRIPTOR.message_types_by_name['IdRef'] = _IDREF
DESCRIPTOR.message_types_by_name['DisturbanceOfPopulationMovementRoutesRequest'] = _DISTURBANCEOFPOPULATIONMOVEMENTROUTESREQUEST
DESCRIPTOR.message_types_by_name['RouteWKB'] = _ROUTEWKB
DESCRIPTOR.message_types_by_name['ArrowRecordBatch'] = _ARROWRECORDBATCH
DESCRIPTOR.message_types_by_name['DisturbanceOfPopulationMovementRoutes'] = _DISTURBANCEOFPOPULATIONMOVEMENTROUTES
DESCRIPTOR.message_types_by_name['GraphInfoResponse'] = _GRAPHINFORESPONSE
DESCRIPTOR.enum_types_by_name['CellInRoutePosition'] = _CELLINROUTEPOSITION
_sym_db.RegisterFileDescriptor(DESCRIPTOR)

Empty = _reflection.GeneratedProtocolMessageType('Empty', (_message.Message,), {
  'DESCRIPTOR' : _EMPTY,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.Empty)
  })
_sym_db.RegisterMessage(Empty)

VersionResponse = _reflection.GeneratedProtocolMessageType('VersionResponse', (_message.Message,), {
  'DESCRIPTOR' : _VERSIONRESPONSE,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.VersionResponse)
  })
_sym_db.RegisterMessage(VersionResponse)

Point = _reflection.GeneratedProtocolMessageType('Point', (_message.Message,), {
  'DESCRIPTOR' : _POINT,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.Point)
  })
_sym_db.RegisterMessage(Point)

DisturbanceOfPopulationMovementRequest = _reflection.GeneratedProtocolMessageType('DisturbanceOfPopulationMovementRequest', (_message.Message,), {
  'DESCRIPTOR' : _DISTURBANCEOFPOPULATIONMOVEMENTREQUEST,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.DisturbanceOfPopulationMovementRequest)
  })
_sym_db.RegisterMessage(DisturbanceOfPopulationMovementRequest)

IdRef = _reflection.GeneratedProtocolMessageType('IdRef', (_message.Message,), {
  'DESCRIPTOR' : _IDREF,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.IdRef)
  })
_sym_db.RegisterMessage(IdRef)

DisturbanceOfPopulationMovementRoutesRequest = _reflection.GeneratedProtocolMessageType('DisturbanceOfPopulationMovementRoutesRequest', (_message.Message,), {
  'DESCRIPTOR' : _DISTURBANCEOFPOPULATIONMOVEMENTROUTESREQUEST,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.DisturbanceOfPopulationMovementRoutesRequest)
  })
_sym_db.RegisterMessage(DisturbanceOfPopulationMovementRoutesRequest)

RouteWKB = _reflection.GeneratedProtocolMessageType('RouteWKB', (_message.Message,), {
  'DESCRIPTOR' : _ROUTEWKB,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.RouteWKB)
  })
_sym_db.RegisterMessage(RouteWKB)

ArrowRecordBatch = _reflection.GeneratedProtocolMessageType('ArrowRecordBatch', (_message.Message,), {
  'DESCRIPTOR' : _ARROWRECORDBATCH,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.ArrowRecordBatch)
  })
_sym_db.RegisterMessage(ArrowRecordBatch)

DisturbanceOfPopulationMovementRoutes = _reflection.GeneratedProtocolMessageType('DisturbanceOfPopulationMovementRoutes', (_message.Message,), {
  'DESCRIPTOR' : _DISTURBANCEOFPOPULATIONMOVEMENTROUTES,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.DisturbanceOfPopulationMovementRoutes)
  })
_sym_db.RegisterMessage(DisturbanceOfPopulationMovementRoutes)

GraphInfoResponse = _reflection.GeneratedProtocolMessageType('GraphInfoResponse', (_message.Message,), {
  'DESCRIPTOR' : _GRAPHINFORESPONSE,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.GraphInfoResponse)
  })
_sym_db.RegisterMessage(GraphInfoResponse)



_ROUTE3ROAD = _descriptor.ServiceDescriptor(
  name='Route3Road',
  full_name='route3.road.Route3Road',
  file=DESCRIPTOR,
  index=0,
  serialized_options=None,
  create_key=_descriptor._internal_create_key,
  serialized_start=913,
  serialized_end=1439,
  methods=[
  _descriptor.MethodDescriptor(
    name='Version',
    full_name='route3.road.Route3Road.Version',
    index=0,
    containing_service=None,
    input_type=_EMPTY,
    output_type=_VERSIONRESPONSE,
    serialized_options=None,
    create_key=_descriptor._internal_create_key,
  ),
  _descriptor.MethodDescriptor(
    name='GraphInfo',
    full_name='route3.road.Route3Road.GraphInfo',
    index=1,
    containing_service=None,
    input_type=_EMPTY,
    output_type=_GRAPHINFORESPONSE,
    serialized_options=None,
    create_key=_descriptor._internal_create_key,
  ),
  _descriptor.MethodDescriptor(
    name='AnalyzeDisturbanceOfPopulationMovement',
    full_name='route3.road.Route3Road.AnalyzeDisturbanceOfPopulationMovement',
    index=2,
    containing_service=None,
    input_type=_DISTURBANCEOFPOPULATIONMOVEMENTREQUEST,
    output_type=_ARROWRECORDBATCH,
    serialized_options=None,
    create_key=_descriptor._internal_create_key,
  ),
  _descriptor.MethodDescriptor(
    name='GetDisturbanceOfPopulationMovement',
    full_name='route3.road.Route3Road.GetDisturbanceOfPopulationMovement',
    index=3,
    containing_service=None,
    input_type=_IDREF,
    output_type=_ARROWRECORDBATCH,
    serialized_options=None,
    create_key=_descriptor._internal_create_key,
  ),
  _descriptor.MethodDescriptor(
    name='GetDisturbanceOfPopulationMovementRoutes',
    full_name='route3.road.Route3Road.GetDisturbanceOfPopulationMovementRoutes',
    index=4,
    containing_service=None,
    input_type=_DISTURBANCEOFPOPULATIONMOVEMENTROUTESREQUEST,
    output_type=_DISTURBANCEOFPOPULATIONMOVEMENTROUTES,
    serialized_options=None,
    create_key=_descriptor._internal_create_key,
  ),
])
_sym_db.RegisterServiceDescriptor(_ROUTE3ROAD)

DESCRIPTOR.services_by_name['Route3Road'] = _ROUTE3ROAD

# @@protoc_insertion_point(module_scope)
