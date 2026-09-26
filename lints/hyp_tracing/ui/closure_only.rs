//! A closure is not a function a consumer can annotate. This one sits in a
//! static, outside any function, so nothing else here could be reported.

pub static INCREMENT: fn(u32) -> u32 = |n| n + 1;
