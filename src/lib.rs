//! Offline surface contracts. No device, network, audio or lighting-output API.
pub mod adapter;
pub mod codec;
pub mod controller;
pub mod lux_control;
#[cfg(target_os = "linux")]
pub mod lux_operator;
pub mod model;
pub mod operator;
pub mod render;
pub mod simulator;
pub mod surface;

#[cfg(target_os = "linux")]
pub mod frontend;
#[cfg(all(feature = "native", target_os = "linux"))]
pub mod native;

#[cfg(target_os = "linux")]
pub mod native_actions;

#[cfg(target_os = "linux")]
pub mod role_client;
