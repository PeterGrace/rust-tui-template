mod app;
mod consts;
mod theme;
mod cli;
mod tabs;
mod tui;
mod structs;
mod util;

#[macro_use] extern crate tracing;

use crate::app::App;
use clap::Parser;
use tokio::io;
use console_subscriber as tokio_console_subscriber;
use lazy_static::lazy_static;
use log::LevelFilter;
use tokio::sync::RwLock;
use tracing_subscriber::{EnvFilter, Registry, prelude::*};
use tracing_subscriber::fmt::format::FmtSpan;
use tui_logger::TuiTracingSubscriberLayer;
use cli::CliArgs;
use ratatui::prelude::*;
use crate::structs::Preferences;

lazy_static! {
    static ref PREFERENCES: RwLock<Preferences> = RwLock::new(Preferences::default());
    static ref PAGE_SIZE: RwLock<u16> = RwLock::new(0_u16);
    static ref FIFTY_FIFTY: Vec<Constraint> =
        vec![Constraint::Percentage(50), Constraint::Percentage(50)];
}


#[tokio::main]
async fn main() -> io::Result<()> {

    let _ = dotenv::dotenv();
    
    // Initialize tui-logger
    tui_logger::init_logger(log::LevelFilter::Trace).unwrap();
    tui_logger::set_default_level(log::LevelFilter::Trace);
    
    let console_layer = tokio_console_subscriber::spawn();
    let collector = tracing_subscriber::registry()
        .with(console_layer)
        .with(EnvFilter::from_default_env())
        .with(TuiTracingSubscriberLayer);
    tracing::subscriber::set_global_default(collector).expect("Could not initialize logging.");
    let cli = CliArgs::parse();
    let mut app = App::default();
    {
        let mut prefs = PREFERENCES.write().await;
        // setting this to a nonzero length String to help indicate we're a bona-fide
        // preferences struct and not a ::default() generated one.
        prefs.initialized = "Yes".to_owned();
    }
    assert!(!PREFERENCES.read().await.initialized.is_empty());
    let _ = app.run().await;

    Ok(())
}
