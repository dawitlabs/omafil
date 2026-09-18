//! Omafil's toolkit-independent core: filesystem listing and operations,
//! search, previews, archives, drives, desktop integration and Omarchy
//! theming. Nothing here knows how the result will be drawn.

pub mod archive;
pub mod clipboard;
pub mod desktop_integration;
pub mod desktop_requests;
pub mod diagnostics;
pub mod dictation;
pub mod display;
pub mod drives;
pub mod error;
pub mod file_icons;
pub mod file_manager_service;
pub mod icon_theme;
pub mod inspect;
pub mod launch;
pub mod listing;
pub mod omarchy;
pub mod openers;
pub mod operation_io;
pub mod operations;
pub mod paths;
pub mod preview;
pub mod recent;
pub mod recycle;
pub mod search;
pub mod store;
pub mod thumbnail;
pub mod user_dirs;
pub mod watcher;
