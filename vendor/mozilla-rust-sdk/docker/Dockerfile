FROM rust:1.36.0

RUN apt-get update -qq && apt-get install -y clang

# Install Gcloud CLI
RUN curl https://sdk.cloud.google.com | bash
ENV PATH $PATH:/root/google-cloud-sdk/bin/

# Install CMake, required by Rust crates
RUN wget -qO- "https://cmake.org/files/v3.15/cmake-3.15.1-Linux-x86_64.tar.gz" | tar --strip-components=1 -xz -C /usr/local

# Install Protobuf, gmock folder is created to make autogen.sh script work
RUN mkdir /protobuf-2.7.0
WORKDIR /protobuf-2.7.0
RUN curl -OL https://chromium.googlesource.com/external/github.com/google/protobuf/+archive/2.7.0.tar.gz \
    && tar -xvf 2.7.0.tar.gz \
    && mkdir -p gmock \
    && ./autogen.sh \
    && ./configure \
    && make install

# Install Go
RUN curl -s https://dl.google.com/go/go1.12.7.linux-amd64.tar.gz | tar -v -C /usr/local -xz
ENV PATH $PATH:/usr/local/go/bin

# copy Google credentials configuration and set env var
COPY files/credentials.json /root/credentials.json
ENV GOOGLE_APPLICATION_CREDENTIALS /root/credentials.json

WORKDIR /googleapis-rs
