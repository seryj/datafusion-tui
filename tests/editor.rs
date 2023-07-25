
use std::error::Error;

use clap::Parser;
use datafusion_tui::app::core::App;
use datafusion_tui::cli::args::Args;
use datafusion_tui::run_app;
use log::LevelFilter;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::test]
async fn test_switch_tabs() {
    // let args = Args::parse();
    // let mut app = App::new(args).await;
    // let res = run_app(&mut app).await;

    // res.
}
