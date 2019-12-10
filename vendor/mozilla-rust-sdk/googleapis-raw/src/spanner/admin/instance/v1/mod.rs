pub mod spanner_instance_admin;
pub mod spanner_instance_admin_grpc;

pub(crate) use crate::{
    empty,
    iam::v1::{iam_policy, policy},
    longrunning::operations,
};
