#!/bin/bash

source .env

docker run -it --rm \
  --name hyperbedcaller \
  --mount type=volume,src=hyperbedcaller-data,dst=/data \
  -e PHONE="$PHONE" \
  hyperbedcaller
