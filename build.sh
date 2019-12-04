#!/bin/bash

docker build -f Dockerfile.prod \
  --tag="hyperbedcaller" \
  .
