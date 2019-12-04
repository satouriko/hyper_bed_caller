#!/bin/bash

docker stop hyperbedcaller
docker rm hyperbedcaller
docker rmi hyperbedcaller
./build.sh
./start.sh
