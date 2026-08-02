// This module is complete and unit-tested on its own, but nothing calls
// into it yet -- the scheduler that assembles device snapshots from it is
// a later M3 work unit.
#![allow(dead_code, unused_imports)]

pub mod docker;
pub mod key_value;
pub mod metrics;
