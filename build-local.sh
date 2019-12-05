#!/bin/bash

source .env

git clone -b 'v1.5.0' --single-branch --depth 1 https://github.com/tdlib/td.git td

echo $API_ID
echo $API_HASH

docker build -f Dockerfile.local \
  --build-arg COMMIT_SHA="$(git rev-parse HEAD)" \
  --build-arg API_ID="${API_ID}" \
  --build-arg API_HASH="${API_HASH}" \
  --tag="hyperbedcaller" \
  .

docker create --name hyperbedcaller hyperbedcaller
docker cp hyperbedcaller:/usr/lib/libtdjson.so.1.5.0 .
docker cp hyperbedcaller:/hyper_bed_caller/target/release/hyper_bed_caller .
docker rm hyperbedcaller
