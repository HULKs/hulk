#!/usr/bin/env bash

cd "$(dirname "$0")"

protoc -I. --python_out=. feature.proto
protoc -I. --python_out=. example.proto

sed -i 's|import feature_pb2 as feature__pb2|import tfrecord.feature_pb2 as feature__pb2|' example_pb2.py
