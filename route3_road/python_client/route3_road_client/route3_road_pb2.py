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
  serialized_pb=b'\n\x11route3_road.proto\x12\x0broute3.road\"\x07\n\x05\x45mpty\"S\n\x0fVersionResponse\x12\x0f\n\x07version\x18\x01 \x01(\t\x12\x16\n\x0egit_commit_sha\x18\x02 \x01(\t\x12\x17\n\x0f\x62uild_timestamp\x18\x03 \x01(\t\"\x1d\n\x05Point\x12\t\n\x01x\x18\x01 \x01(\x01\x12\t\n\x01y\x18\x02 \x01(\x01\"X\n\x13ShortestPathOptions\x12!\n\x19num_destinations_to_reach\x18\x04 \x01(\r\x12\x1e\n\x16num_gap_cells_to_graph\x18\x06 \x01(\r\"\xb7\x02\n\x1f\x44ifferentialShortestPathRequest\x12.\n\x0cgraph_handle\x18\x01 \x01(\x0b\x32\x18.route3.road.GraphHandle\x12 \n\x18\x64isturbance_wkb_geometry\x18\x02 \x01(\x0c\x12\x15\n\rradius_meters\x18\x03 \x01(\x01\x12\x31\n\x07options\x18\x04 \x01(\x0b\x32 .route3.road.ShortestPathOptions\x12(\n\x0c\x64\x65stinations\x18\x05 \x03(\x0b\x32\x12.route3.road.Point\x12\x1e\n\x16\x64ownsampled_prerouting\x18\x06 \x01(\x08\x12\x14\n\x0cstore_output\x18\x07 \x01(\x08\x12\x18\n\x10ref_dataset_name\x18\x08 \x01(\t\"\x1a\n\x05IdRef\x12\x11\n\tobject_id\x18\x01 \x01(\t\"I\n%DifferentialShortestPathRoutesRequest\x12\x11\n\tobject_id\x18\x01 \x01(\t\x12\r\n\x05\x63\x65lls\x18\x03 \x03(\x04\"T\n\x08RouteWKB\x12\x13\n\x0borigin_cell\x18\x01 \x01(\x04\x12\x18\n\x10\x64\x65stination_cell\x18\x02 \x01(\x04\x12\x0c\n\x04\x63ost\x18\x03 \x01(\x01\x12\x0b\n\x03wkb\x18\x04 \x01(\x0c\"3\n\x10\x41rrowRecordBatch\x12\x11\n\tobject_id\x18\x01 \x01(\t\x12\x0c\n\x04\x64\x61ta\x18\x02 \x01(\x0c\"\x93\x01\n\x1e\x44ifferentialShortestPathRoutes\x12\x39\n\x1aroutes_without_disturbance\x18\x02 \x03(\x0b\x32\x15.route3.road.RouteWKB\x12\x36\n\x17routes_with_disturbance\x18\x03 \x03(\x0b\x32\x15.route3.road.RouteWKB\"2\n\x0bGraphHandle\x12\x0c\n\x04name\x18\x01 \x01(\t\x12\x15\n\rh3_resolution\x18\x02 \x01(\r\"n\n\tGraphInfo\x12(\n\x06handle\x18\x01 \x01(\x0b\x32\x18.route3.road.GraphHandle\x12\x11\n\tis_cached\x18\x02 \x01(\x08\x12\x11\n\tnum_edges\x18\x03 \x01(\x04\x12\x11\n\tnum_nodes\x18\x04 \x01(\x04\"<\n\x12ListGraphsResponse\x12&\n\x06graphs\x18\x01 \x03(\x0b\x32\x16.route3.road.GraphInfo\",\n\x14ListDatasetsResponse\x12\x14\n\x0c\x64\x61taset_name\x18\x01 \x03(\t*2\n\x13\x43\x65llInRoutePosition\x12\n\n\x06Origin\x10\x00\x12\x0f\n\x0b\x44\x65stination\x10\x01\x32\xa7\x04\n\nRoute3Road\x12=\n\x07Version\x12\x12.route3.road.Empty\x1a\x1c.route3.road.VersionResponse\"\x00\x12\x43\n\nListGraphs\x12\x12.route3.road.Empty\x1a\x1f.route3.road.ListGraphsResponse\"\x00\x12G\n\x0cListDatasets\x12\x12.route3.road.Empty\x1a!.route3.road.ListDatasetsResponse\"\x00\x12k\n\x18\x44ifferentialShortestPath\x12,.route3.road.DifferentialShortestPathRequest\x1a\x1d.route3.road.ArrowRecordBatch\"\x00\x30\x01\x12T\n\x1bGetDifferentialShortestPath\x12\x12.route3.road.IdRef\x1a\x1d.route3.road.ArrowRecordBatch\"\x00\x30\x01\x12\x88\x01\n!GetDifferentialShortestPathRoutes\x12\x32.route3.road.DifferentialShortestPathRoutesRequest\x1a+.route3.road.DifferentialShortestPathRoutes\"\x00\x30\x01\x62\x06proto3'
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
  serialized_start=1227,
  serialized_end=1277,
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


_SHORTESTPATHOPTIONS = _descriptor.Descriptor(
  name='ShortestPathOptions',
  full_name='route3.road.ShortestPathOptions',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='num_destinations_to_reach', full_name='route3.road.ShortestPathOptions.num_destinations_to_reach', index=0,
      number=4, type=13, cpp_type=3, label=1,
      has_default_value=False, default_value=0,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='num_gap_cells_to_graph', full_name='route3.road.ShortestPathOptions.num_gap_cells_to_graph', index=1,
      number=6, type=13, cpp_type=3, label=1,
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
  serialized_start=159,
  serialized_end=247,
)


_DIFFERENTIALSHORTESTPATHREQUEST = _descriptor.Descriptor(
  name='DifferentialShortestPathRequest',
  full_name='route3.road.DifferentialShortestPathRequest',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='graph_handle', full_name='route3.road.DifferentialShortestPathRequest.graph_handle', index=0,
      number=1, type=11, cpp_type=10, label=1,
      has_default_value=False, default_value=None,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='disturbance_wkb_geometry', full_name='route3.road.DifferentialShortestPathRequest.disturbance_wkb_geometry', index=1,
      number=2, type=12, cpp_type=9, label=1,
      has_default_value=False, default_value=b"",
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='radius_meters', full_name='route3.road.DifferentialShortestPathRequest.radius_meters', index=2,
      number=3, type=1, cpp_type=5, label=1,
      has_default_value=False, default_value=float(0),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='options', full_name='route3.road.DifferentialShortestPathRequest.options', index=3,
      number=4, type=11, cpp_type=10, label=1,
      has_default_value=False, default_value=None,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='destinations', full_name='route3.road.DifferentialShortestPathRequest.destinations', index=4,
      number=5, type=11, cpp_type=10, label=3,
      has_default_value=False, default_value=[],
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='downsampled_prerouting', full_name='route3.road.DifferentialShortestPathRequest.downsampled_prerouting', index=5,
      number=6, type=8, cpp_type=7, label=1,
      has_default_value=False, default_value=False,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='store_output', full_name='route3.road.DifferentialShortestPathRequest.store_output', index=6,
      number=7, type=8, cpp_type=7, label=1,
      has_default_value=False, default_value=False,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='ref_dataset_name', full_name='route3.road.DifferentialShortestPathRequest.ref_dataset_name', index=7,
      number=8, type=9, cpp_type=9, label=1,
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
  serialized_start=250,
  serialized_end=561,
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
      name='object_id', full_name='route3.road.IdRef.object_id', index=0,
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
  serialized_start=563,
  serialized_end=589,
)


_DIFFERENTIALSHORTESTPATHROUTESREQUEST = _descriptor.Descriptor(
  name='DifferentialShortestPathRoutesRequest',
  full_name='route3.road.DifferentialShortestPathRoutesRequest',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='object_id', full_name='route3.road.DifferentialShortestPathRoutesRequest.object_id', index=0,
      number=1, type=9, cpp_type=9, label=1,
      has_default_value=False, default_value=b"".decode('utf-8'),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='cells', full_name='route3.road.DifferentialShortestPathRoutesRequest.cells', index=1,
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
  serialized_start=591,
  serialized_end=664,
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
  serialized_start=666,
  serialized_end=750,
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
  serialized_start=752,
  serialized_end=803,
)


_DIFFERENTIALSHORTESTPATHROUTES = _descriptor.Descriptor(
  name='DifferentialShortestPathRoutes',
  full_name='route3.road.DifferentialShortestPathRoutes',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='routes_without_disturbance', full_name='route3.road.DifferentialShortestPathRoutes.routes_without_disturbance', index=0,
      number=2, type=11, cpp_type=10, label=3,
      has_default_value=False, default_value=[],
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='routes_with_disturbance', full_name='route3.road.DifferentialShortestPathRoutes.routes_with_disturbance', index=1,
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
  serialized_start=806,
  serialized_end=953,
)


_GRAPHHANDLE = _descriptor.Descriptor(
  name='GraphHandle',
  full_name='route3.road.GraphHandle',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='name', full_name='route3.road.GraphHandle.name', index=0,
      number=1, type=9, cpp_type=9, label=1,
      has_default_value=False, default_value=b"".decode('utf-8'),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='h3_resolution', full_name='route3.road.GraphHandle.h3_resolution', index=1,
      number=2, type=13, cpp_type=3, label=1,
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
  serialized_start=955,
  serialized_end=1005,
)


_GRAPHINFO = _descriptor.Descriptor(
  name='GraphInfo',
  full_name='route3.road.GraphInfo',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='handle', full_name='route3.road.GraphInfo.handle', index=0,
      number=1, type=11, cpp_type=10, label=1,
      has_default_value=False, default_value=None,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='is_cached', full_name='route3.road.GraphInfo.is_cached', index=1,
      number=2, type=8, cpp_type=7, label=1,
      has_default_value=False, default_value=False,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='num_edges', full_name='route3.road.GraphInfo.num_edges', index=2,
      number=3, type=4, cpp_type=4, label=1,
      has_default_value=False, default_value=0,
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='num_nodes', full_name='route3.road.GraphInfo.num_nodes', index=3,
      number=4, type=4, cpp_type=4, label=1,
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
  serialized_start=1007,
  serialized_end=1117,
)


_LISTGRAPHSRESPONSE = _descriptor.Descriptor(
  name='ListGraphsResponse',
  full_name='route3.road.ListGraphsResponse',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='graphs', full_name='route3.road.ListGraphsResponse.graphs', index=0,
      number=1, type=11, cpp_type=10, label=3,
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
  serialized_start=1119,
  serialized_end=1179,
)


_LISTDATASETSRESPONSE = _descriptor.Descriptor(
  name='ListDatasetsResponse',
  full_name='route3.road.ListDatasetsResponse',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='dataset_name', full_name='route3.road.ListDatasetsResponse.dataset_name', index=0,
      number=1, type=9, cpp_type=9, label=3,
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
  serialized_start=1181,
  serialized_end=1225,
)

_DIFFERENTIALSHORTESTPATHREQUEST.fields_by_name['graph_handle'].message_type = _GRAPHHANDLE
_DIFFERENTIALSHORTESTPATHREQUEST.fields_by_name['options'].message_type = _SHORTESTPATHOPTIONS
_DIFFERENTIALSHORTESTPATHREQUEST.fields_by_name['destinations'].message_type = _POINT
_DIFFERENTIALSHORTESTPATHROUTES.fields_by_name['routes_without_disturbance'].message_type = _ROUTEWKB
_DIFFERENTIALSHORTESTPATHROUTES.fields_by_name['routes_with_disturbance'].message_type = _ROUTEWKB
_GRAPHINFO.fields_by_name['handle'].message_type = _GRAPHHANDLE
_LISTGRAPHSRESPONSE.fields_by_name['graphs'].message_type = _GRAPHINFO
DESCRIPTOR.message_types_by_name['Empty'] = _EMPTY
DESCRIPTOR.message_types_by_name['VersionResponse'] = _VERSIONRESPONSE
DESCRIPTOR.message_types_by_name['Point'] = _POINT
DESCRIPTOR.message_types_by_name['ShortestPathOptions'] = _SHORTESTPATHOPTIONS
DESCRIPTOR.message_types_by_name['DifferentialShortestPathRequest'] = _DIFFERENTIALSHORTESTPATHREQUEST
DESCRIPTOR.message_types_by_name['IdRef'] = _IDREF
DESCRIPTOR.message_types_by_name['DifferentialShortestPathRoutesRequest'] = _DIFFERENTIALSHORTESTPATHROUTESREQUEST
DESCRIPTOR.message_types_by_name['RouteWKB'] = _ROUTEWKB
DESCRIPTOR.message_types_by_name['ArrowRecordBatch'] = _ARROWRECORDBATCH
DESCRIPTOR.message_types_by_name['DifferentialShortestPathRoutes'] = _DIFFERENTIALSHORTESTPATHROUTES
DESCRIPTOR.message_types_by_name['GraphHandle'] = _GRAPHHANDLE
DESCRIPTOR.message_types_by_name['GraphInfo'] = _GRAPHINFO
DESCRIPTOR.message_types_by_name['ListGraphsResponse'] = _LISTGRAPHSRESPONSE
DESCRIPTOR.message_types_by_name['ListDatasetsResponse'] = _LISTDATASETSRESPONSE
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

ShortestPathOptions = _reflection.GeneratedProtocolMessageType('ShortestPathOptions', (_message.Message,), {
  'DESCRIPTOR' : _SHORTESTPATHOPTIONS,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.ShortestPathOptions)
  })
_sym_db.RegisterMessage(ShortestPathOptions)

DifferentialShortestPathRequest = _reflection.GeneratedProtocolMessageType('DifferentialShortestPathRequest', (_message.Message,), {
  'DESCRIPTOR' : _DIFFERENTIALSHORTESTPATHREQUEST,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.DifferentialShortestPathRequest)
  })
_sym_db.RegisterMessage(DifferentialShortestPathRequest)

IdRef = _reflection.GeneratedProtocolMessageType('IdRef', (_message.Message,), {
  'DESCRIPTOR' : _IDREF,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.IdRef)
  })
_sym_db.RegisterMessage(IdRef)

DifferentialShortestPathRoutesRequest = _reflection.GeneratedProtocolMessageType('DifferentialShortestPathRoutesRequest', (_message.Message,), {
  'DESCRIPTOR' : _DIFFERENTIALSHORTESTPATHROUTESREQUEST,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.DifferentialShortestPathRoutesRequest)
  })
_sym_db.RegisterMessage(DifferentialShortestPathRoutesRequest)

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

DifferentialShortestPathRoutes = _reflection.GeneratedProtocolMessageType('DifferentialShortestPathRoutes', (_message.Message,), {
  'DESCRIPTOR' : _DIFFERENTIALSHORTESTPATHROUTES,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.DifferentialShortestPathRoutes)
  })
_sym_db.RegisterMessage(DifferentialShortestPathRoutes)

GraphHandle = _reflection.GeneratedProtocolMessageType('GraphHandle', (_message.Message,), {
  'DESCRIPTOR' : _GRAPHHANDLE,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.GraphHandle)
  })
_sym_db.RegisterMessage(GraphHandle)

GraphInfo = _reflection.GeneratedProtocolMessageType('GraphInfo', (_message.Message,), {
  'DESCRIPTOR' : _GRAPHINFO,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.GraphInfo)
  })
_sym_db.RegisterMessage(GraphInfo)

ListGraphsResponse = _reflection.GeneratedProtocolMessageType('ListGraphsResponse', (_message.Message,), {
  'DESCRIPTOR' : _LISTGRAPHSRESPONSE,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.ListGraphsResponse)
  })
_sym_db.RegisterMessage(ListGraphsResponse)

ListDatasetsResponse = _reflection.GeneratedProtocolMessageType('ListDatasetsResponse', (_message.Message,), {
  'DESCRIPTOR' : _LISTDATASETSRESPONSE,
  '__module__' : 'route3_road_pb2'
  # @@protoc_insertion_point(class_scope:route3.road.ListDatasetsResponse)
  })
_sym_db.RegisterMessage(ListDatasetsResponse)



_ROUTE3ROAD = _descriptor.ServiceDescriptor(
  name='Route3Road',
  full_name='route3.road.Route3Road',
  file=DESCRIPTOR,
  index=0,
  serialized_options=None,
  create_key=_descriptor._internal_create_key,
  serialized_start=1280,
  serialized_end=1831,
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
    name='ListGraphs',
    full_name='route3.road.Route3Road.ListGraphs',
    index=1,
    containing_service=None,
    input_type=_EMPTY,
    output_type=_LISTGRAPHSRESPONSE,
    serialized_options=None,
    create_key=_descriptor._internal_create_key,
  ),
  _descriptor.MethodDescriptor(
    name='ListDatasets',
    full_name='route3.road.Route3Road.ListDatasets',
    index=2,
    containing_service=None,
    input_type=_EMPTY,
    output_type=_LISTDATASETSRESPONSE,
    serialized_options=None,
    create_key=_descriptor._internal_create_key,
  ),
  _descriptor.MethodDescriptor(
    name='DifferentialShortestPath',
    full_name='route3.road.Route3Road.DifferentialShortestPath',
    index=3,
    containing_service=None,
    input_type=_DIFFERENTIALSHORTESTPATHREQUEST,
    output_type=_ARROWRECORDBATCH,
    serialized_options=None,
    create_key=_descriptor._internal_create_key,
  ),
  _descriptor.MethodDescriptor(
    name='GetDifferentialShortestPath',
    full_name='route3.road.Route3Road.GetDifferentialShortestPath',
    index=4,
    containing_service=None,
    input_type=_IDREF,
    output_type=_ARROWRECORDBATCH,
    serialized_options=None,
    create_key=_descriptor._internal_create_key,
  ),
  _descriptor.MethodDescriptor(
    name='GetDifferentialShortestPathRoutes',
    full_name='route3.road.Route3Road.GetDifferentialShortestPathRoutes',
    index=5,
    containing_service=None,
    input_type=_DIFFERENTIALSHORTESTPATHROUTESREQUEST,
    output_type=_DIFFERENTIALSHORTESTPATHROUTES,
    serialized_options=None,
    create_key=_descriptor._internal_create_key,
  ),
])
_sym_db.RegisterServiceDescriptor(_ROUTE3ROAD)

DESCRIPTOR.services_by_name['Route3Road'] = _ROUTE3ROAD

# @@protoc_insertion_point(module_scope)
