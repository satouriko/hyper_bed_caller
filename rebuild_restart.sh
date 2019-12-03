#!/bin/bash

./build.sh
docker stop hyperbedcaller
docker rm hyperbedcaller
./start.sh
