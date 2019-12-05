FROM rust:1.39
RUN apt-get update && apt-get install -y build-essential libssl-dev zlib1g-dev gperf cmake git
RUN git clone -b 'v1.5.0' --single-branch --depth 1 https://github.com/tdlib/td.git /td
RUN cd /td && mkdir build && cd build \
    && cmake -DCMAKE_INSTALL_PREFIX="/usr" -DCMAKE_BUILD_TYPE=Release .. && cmake --build . -- -j 2 \
    && cmake --build . --target install
ARG API_ID=0
ENV API_ID=${API_ID}
ARG API_HASH=""
ENV API_HASH=${API_HASH}
ARG COMMIT_SHA=""
ENV COMMIT_SHA=${COMMIT_SHA}
ENV DATA_PATH="/data"
COPY . /hyper_bed_caller
RUN cd /hyper_bed_caller \
    && cargo build --release
WORKDIR /hyper_bed_caller/target/release
CMD /hyper_bed_caller/target/release/hyper_bed_caller
VOLUME [ "/data" ]
