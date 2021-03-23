// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(bare_trait_objects)]

// This appears as a comment in each generated file. Add it once here
// to save a bit of time and effort.

const _PROTOBUF_VERSION_CHECK: () = ::protobuf::VERSION_2_22_0;

pub mod empty;
pub(crate) mod iam;
pub mod longrunning;
pub(crate) mod rpc;
pub(crate) mod r#type;

pub mod bigtable;
pub mod pubsub;
pub mod spanner;
pub mod storage;
