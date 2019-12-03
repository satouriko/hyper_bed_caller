#!/bin/bash

source .env

docker run -it \
  --name hyperbedcaller \
  --mount type=volume,src=hyperbedcaller-data,dst=/data \
  -e PHONE="$PHONE" \
  hyperbedcaller
