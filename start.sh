#!/bin/bash

source .env

docker run -d --restart=always \
  --name hyperbedcaller \
  --mount type=volume,src=hyperbedcaller-data,dst=/data \
  -e PHONE="$PHONE" \
  hyperbedcaller
