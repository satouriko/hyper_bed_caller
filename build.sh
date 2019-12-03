#!/bin/bash

source .env

docker build -f Dockerfile.prod \
  --build-arg COMMIT_SHA="$(git rev-parse HEAD)" \
  --build-arg API_ID="${API_ID}" \
  --build-arg API_HASH="${API_HASH}" \
  --tag="hyperbedcaller" \
  .
