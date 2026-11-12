//! PDF Catcher: tangkap file PDF, lalu gabung, pisah, atau ubah ke Word.
//!
//! Crate library ini berisi semua logika yang tidak bergantung pada UI,
//! sehingga bisa diuji tanpa membuka jendela.

pub mod core;
pub mod error;
pub mod settings;
pub mod single_instance;
pub mod worker;

pub use error::AppError;
