# -*- coding: utf-8 -*-
# Generated by the protocol buffer compiler.  DO NOT EDIT!
# source: route3.proto
"""Generated protocol buffer code."""
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from google.protobuf import reflection as _reflection
from google.protobuf import symbol_database as _symbol_database
# @@protoc_insertion_point(imports)

_sym_db = _symbol_database.Default()




DESCRIPTOR = _descriptor.FileDescriptor(
  name='route3.proto',
  package='grpc.route3',
  syntax='proto3',
  serialized_options=None,
  create_key=_descriptor._internal_create_key,
  serialized_pb=b'\n\x0croute3.proto\x12\x0bgrpc.route3\"\x10\n\x0eVersionRequest\"\"\n\x0fVersionResponse\x12\x0f\n\x07version\x18\x01 \x01(\t\"\x1d\n\x05Point\x12\t\n\x01x\x18\x01 \x01(\x01\x12\t\n\x01y\x18\x02 \x01(\x01\"s\n\x19\x41nalyzeDisturbanceRequest\x12\x14\n\x0cwkb_geometry\x18\x01 \x01(\x0c\x12\x15\n\rradius_meters\x18\x02 \x01(\x01\x12)\n\rtarget_points\x18\x03 \x03(\x0b\x32\x12.grpc.route3.Point\"C\n\x1a\x41nalyzeDisturbanceResponse\x12%\n\x1dpopulation_within_disturbance\x18\x01 \x01(\x01\x32\xb9\x01\n\x06Route3\x12\x46\n\x07Version\x12\x1b.grpc.route3.VersionRequest\x1a\x1c.grpc.route3.VersionResponse\"\x00\x12g\n\x12\x41nalyzeDisturbance\x12&.grpc.route3.AnalyzeDisturbanceRequest\x1a\'.grpc.route3.AnalyzeDisturbanceResponse\"\x00\x62\x06proto3'
)




_VERSIONREQUEST = _descriptor.Descriptor(
  name='VersionRequest',
  full_name='grpc.route3.VersionRequest',
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
  serialized_start=29,
  serialized_end=45,
)


_VERSIONRESPONSE = _descriptor.Descriptor(
  name='VersionResponse',
  full_name='grpc.route3.VersionResponse',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='version', full_name='grpc.route3.VersionResponse.version', index=0,
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
  serialized_start=47,
  serialized_end=81,
)


_POINT = _descriptor.Descriptor(
  name='Point',
  full_name='grpc.route3.Point',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='x', full_name='grpc.route3.Point.x', index=0,
      number=1, type=1, cpp_type=5, label=1,
      has_default_value=False, default_value=float(0),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='y', full_name='grpc.route3.Point.y', index=1,
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
  serialized_start=83,
  serialized_end=112,
)


_ANALYZEDISTURBANCEREQUEST = _descriptor.Descriptor(
  name='AnalyzeDisturbanceRequest',
  full_name='grpc.route3.AnalyzeDisturbanceRequest',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='wkb_geometry', full_name='grpc.route3.AnalyzeDisturbanceRequest.wkb_geometry', index=0,
      number=1, type=12, cpp_type=9, label=1,
      has_default_value=False, default_value=b"",
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='radius_meters', full_name='grpc.route3.AnalyzeDisturbanceRequest.radius_meters', index=1,
      number=2, type=1, cpp_type=5, label=1,
      has_default_value=False, default_value=float(0),
      message_type=None, enum_type=None, containing_type=None,
      is_extension=False, extension_scope=None,
      serialized_options=None, file=DESCRIPTOR,  create_key=_descriptor._internal_create_key),
    _descriptor.FieldDescriptor(
      name='target_points', full_name='grpc.route3.AnalyzeDisturbanceRequest.target_points', index=2,
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
  serialized_start=114,
  serialized_end=229,
)


_ANALYZEDISTURBANCERESPONSE = _descriptor.Descriptor(
  name='AnalyzeDisturbanceResponse',
  full_name='grpc.route3.AnalyzeDisturbanceResponse',
  filename=None,
  file=DESCRIPTOR,
  containing_type=None,
  create_key=_descriptor._internal_create_key,
  fields=[
    _descriptor.FieldDescriptor(
      name='population_within_disturbance', full_name='grpc.route3.AnalyzeDisturbanceResponse.population_within_disturbance', index=0,
      number=1, type=1, cpp_type=5, label=1,
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
  serialized_start=231,
  serialized_end=298,
)

_ANALYZEDISTURBANCEREQUEST.fields_by_name['target_points'].message_type = _POINT
DESCRIPTOR.message_types_by_name['VersionRequest'] = _VERSIONREQUEST
DESCRIPTOR.message_types_by_name['VersionResponse'] = _VERSIONRESPONSE
DESCRIPTOR.message_types_by_name['Point'] = _POINT
DESCRIPTOR.message_types_by_name['AnalyzeDisturbanceRequest'] = _ANALYZEDISTURBANCEREQUEST
DESCRIPTOR.message_types_by_name['AnalyzeDisturbanceResponse'] = _ANALYZEDISTURBANCERESPONSE
_sym_db.RegisterFileDescriptor(DESCRIPTOR)

VersionRequest = _reflection.GeneratedProtocolMessageType('VersionRequest', (_message.Message,), {
  'DESCRIPTOR' : _VERSIONREQUEST,
  '__module__' : 'route3_pb2'
  # @@protoc_insertion_point(class_scope:grpc.route3.VersionRequest)
  })
_sym_db.RegisterMessage(VersionRequest)

VersionResponse = _reflection.GeneratedProtocolMessageType('VersionResponse', (_message.Message,), {
  'DESCRIPTOR' : _VERSIONRESPONSE,
  '__module__' : 'route3_pb2'
  # @@protoc_insertion_point(class_scope:grpc.route3.VersionResponse)
  })
_sym_db.RegisterMessage(VersionResponse)

Point = _reflection.GeneratedProtocolMessageType('Point', (_message.Message,), {
  'DESCRIPTOR' : _POINT,
  '__module__' : 'route3_pb2'
  # @@protoc_insertion_point(class_scope:grpc.route3.Point)
  })
_sym_db.RegisterMessage(Point)

AnalyzeDisturbanceRequest = _reflection.GeneratedProtocolMessageType('AnalyzeDisturbanceRequest', (_message.Message,), {
  'DESCRIPTOR' : _ANALYZEDISTURBANCEREQUEST,
  '__module__' : 'route3_pb2'
  # @@protoc_insertion_point(class_scope:grpc.route3.AnalyzeDisturbanceRequest)
  })
_sym_db.RegisterMessage(AnalyzeDisturbanceRequest)

AnalyzeDisturbanceResponse = _reflection.GeneratedProtocolMessageType('AnalyzeDisturbanceResponse', (_message.Message,), {
  'DESCRIPTOR' : _ANALYZEDISTURBANCERESPONSE,
  '__module__' : 'route3_pb2'
  # @@protoc_insertion_point(class_scope:grpc.route3.AnalyzeDisturbanceResponse)
  })
_sym_db.RegisterMessage(AnalyzeDisturbanceResponse)



_ROUTE3 = _descriptor.ServiceDescriptor(
  name='Route3',
  full_name='grpc.route3.Route3',
  file=DESCRIPTOR,
  index=0,
  serialized_options=None,
  create_key=_descriptor._internal_create_key,
  serialized_start=301,
  serialized_end=486,
  methods=[
  _descriptor.MethodDescriptor(
    name='Version',
    full_name='grpc.route3.Route3.Version',
    index=0,
    containing_service=None,
    input_type=_VERSIONREQUEST,
    output_type=_VERSIONRESPONSE,
    serialized_options=None,
    create_key=_descriptor._internal_create_key,
  ),
  _descriptor.MethodDescriptor(
    name='AnalyzeDisturbance',
    full_name='grpc.route3.Route3.AnalyzeDisturbance',
    index=1,
    containing_service=None,
    input_type=_ANALYZEDISTURBANCEREQUEST,
    output_type=_ANALYZEDISTURBANCERESPONSE,
    serialized_options=None,
    create_key=_descriptor._internal_create_key,
  ),
])
_sym_db.RegisterServiceDescriptor(_ROUTE3)

DESCRIPTOR.services_by_name['Route3'] = _ROUTE3

# @@protoc_insertion_point(module_scope)
