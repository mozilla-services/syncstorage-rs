# Copyright 2020 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

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
