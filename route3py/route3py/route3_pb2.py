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
  serialized_pb=b'\n\x0croute3.proto\x12\x0bgrpc.route3\"\x10\n\x0eVersionRequest\"\"\n\x0fVersionResponse\x12\x0f\n\x07version\x18\x01 \x01(\t\"1\n\x19\x41nalyzeDisturbanceRequest\x12\x14\n\x0cwkb_geometry\x18\x01 \x01(\x0c\"\x1c\n\x1a\x41nalyzeDisturbanceResponse2\xb9\x01\n\x06Route3\x12\x46\n\x07Version\x12\x1b.grpc.route3.VersionRequest\x1a\x1c.grpc.route3.VersionResponse\"\x00\x12g\n\x12\x41nalyzeDisturbance\x12&.grpc.route3.AnalyzeDisturbanceRequest\x1a\'.grpc.route3.AnalyzeDisturbanceResponse\"\x00\x62\x06proto3'
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
  serialized_end=132,
)


_ANALYZEDISTURBANCERESPONSE = _descriptor.Descriptor(
  name='AnalyzeDisturbanceResponse',
  full_name='grpc.route3.AnalyzeDisturbanceResponse',
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
  serialized_start=134,
  serialized_end=162,
)

DESCRIPTOR.message_types_by_name['VersionRequest'] = _VERSIONREQUEST
DESCRIPTOR.message_types_by_name['VersionResponse'] = _VERSIONRESPONSE
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
  serialized_start=165,
  serialized_end=350,
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
