#![allow(non_snake_case)]
#![recursion_limit = "512"]

pub mod app;
pub mod cli;
pub mod command;
pub mod config;
pub mod entities;
pub mod gpu_backend;
pub mod i18n;
pub(crate) mod input;
pub mod io;
pub mod mcp;
pub mod mcp_ops;
pub mod modules;
pub mod par;
pub mod perf;
pub mod rest;
pub mod scene;
pub mod snap;
pub mod sys;
pub mod sysinfo;
pub mod ui;
pub mod update_check;
