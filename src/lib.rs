//! PinWin (Rust Edition) — Core Library
//!
//! A complete, zero-dependency native Win32 Rust rewrite of Victor Zakharov's PinWin
//! (DeskPins alternative for Windows 10/11).

pub mod algorithms;
pub mod cli;
pub mod debounce;
pub mod keys;
pub mod overlay;
pub mod pin_manager;
pub mod relative_path;
pub mod settings;
pub mod ui;
pub mod winapi;
