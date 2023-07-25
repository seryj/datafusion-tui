// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Context (remote or local)

use arrow::record_batch::RecordBatch;
use datafusion::dataframe::DataFrame;
use datafusion::error::{DataFusionError, Result};
use datafusion::execution::context::{SessionConfig, SessionContext};

use log::{debug, error, info};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Arc;

use crate::app::ui::Scroll;

#[derive(Clone)]
pub struct QueryResultsMeta {
    pub query: String,
    pub succeeded: bool,
    pub error: Option<String>,
    pub rows: usize,
    pub query_duration: f64,
}

pub struct QueryResults {
    pub batches: Vec<RecordBatch>,
    pub pretty_batches: String,
    pub meta: QueryResultsMeta,
    pub scroll: Scroll,
}

impl QueryResults {
    pub fn format_timing_info(&self) -> String {
        format!(
            "[ {} {} in set. Query took {:.3} seconds ] ",
            self.meta.rows,
            if self.meta.rows == 1 { "row" } else { "rows" },
            self.meta.query_duration
        )
    }
}

/// The CLI supports using a local DataFusion context or a distributed BallistaContext
pub enum Context {
    /// In-process execution with DataFusion
    Local(SessionContext),
    // /// Distributed execution with Ballista (if available)
    // Remote(BallistaContext),
}

impl Context {
    /// create a new remote context with given host and port
    // pub fn new_remote(host: &str, port: u16) -> Result<Context> {
    //     debug!("Created BallistaContext @ {:?}:{:?}", host, port);
    //     Ok(Context::Remote(BallistaContext::try_new(host, port)?))
    // }

    /// create a local context using the given config
    pub async fn new_local(config: &SessionConfig) -> Context {
        debug!("Created ExecutionContext");
        let ctx = SessionContext::with_config(config.clone());

        #[cfg(feature = "s3")]
        use crate::app::datafusion::object_stores::register_s3;
        #[cfg(feature = "s3")]
        let ctx = register_s3(ctx).await;

        #[cfg(feature = "bigtable")]
        use crate::app::datafusion::table_providers::register_bigtable;
        #[cfg(feature = "bigtable")]
        let ctx = register_bigtable(ctx).await;

        Context::Local(ctx)
    }

    /// execute an SQL statement against the context
    pub async fn sql(&mut self, sql: &str) -> Result<DataFrame> {
        info!("Executing SQL: {:?}", sql);
        match self {
            Context::Local(datafusion) => datafusion.sql(sql).await,
            // Context::Remote(ballista) => ballista.sql(sql).await,
        }
    }

    pub async fn exec_files(&mut self, files: Vec<String>) {
        let files = files
            .into_iter()
            .map(|file_path| File::open(file_path).unwrap())
            .collect::<Vec<_>>();
        for file in files {
            let mut reader = BufReader::new(file);
            exec_from_lines(self, &mut reader).await;
        }
    }

    pub fn format_execution_config(&self) -> Option<Vec<String>> {
        // match self {
        //     Context::Local(ctx) => {
        //         let mut config = Vec::new();
        //         let cfg = ctx.state.lock().config.clone();
        //         debug!("Extracting ExecutionConfig attributes");
        //         config.push(format!("Target Partitions: {}", cfg.target_partitions));
        //         config.push(format!("Repartition Joins: {}", cfg.repartition_joins));
        //         config.push(format!(
        //             "Repartition Aggregations: {}",
        //             cfg.repartition_aggregations
        //         ));
        //         config.push(format!("Repartition Windows: {}", cfg.repartition_windows));
        //         Some(config)
        //     }
        //     Context::Remote(_) => None,
        // }
        None
    }

    pub fn format_physical_optimizers(&self) -> Option<Vec<String>> {
        // match self {
        //     Context::Local(ctx) => {
        //         let physical_opts = ctx.state().config().physical_optimizers.clone();
        //         debug!("Extracting physical optimizer rules");
        //         let opts = physical_opts
        //             .iter()
        //             .map(|opt| opt.name().to_string())
        //             .collect();
        //         Some(opts)
        //     }
        //     Context::Remote(_) => None,
        // }
        None
    }
}

async fn exec_from_lines(ctx: &mut Context, reader: &mut BufReader<File>) {
    let mut query = "".to_owned();

    for line in reader.lines() {
        match line {
            Ok(line) if line.starts_with("--") => {
                continue;
            }
            Ok(line) => {
                let line = line.trim_end();
                query.push_str(line);
                if line.ends_with(';') {
                    match exec_and_print(ctx, query).await {
                        Ok(_) => {}
                        Err(err) => error!("{:?}", err),
                    }
                    query = "".to_owned();
                } else {
                    query.push('\n');
                }
            }
            _ => {
                break;
            }
        }
    }

    // run the left over query if the last statement doesn't contain ‘;’
    if !query.is_empty() {
        match exec_and_print(ctx, query).await {
            Ok(_) => {}
            Err(err) => error!("{:?}", err),
        }
    }
}

async fn exec_and_print(ctx: &mut Context, sql: String) -> Result<()> {
    let _df = ctx.sql(&sql).await?;
    Ok(())
}

// implement wrappers around the BallistaContext to support running without ballista

// Feature added but not tested as cant install from crates
#[cfg(feature = "ballista")]
use ballista;
#[cfg(feature = "ballista")]
pub struct BallistaContext(ballista::context::BallistaContext);
#[cfg(feature = "ballista")]
impl BallistaContext {
    pub fn try_new(host: &str, port: u16) -> Result<Self> {
        use ballista::context::BallistaContext;
        use ballista::prelude::BallistaConfig;
        let config: BallistaConfig =
            BallistaConfig::new().map_err(|e| DataFusionError::Execution(format!("{:?}", e)))?;
        Ok(Self(BallistaContext::remote(host, port, &config)))
    }
    pub async fn sql(&mut self, sql: &str) -> Result<Arc<dyn DataFrame>> {
        self.0.sql(sql).await
    }
}

// Feature added but not tested as cant install from crates
#[cfg(not(feature = "ballista"))]
pub struct BallistaContext();
#[cfg(not(feature = "ballista"))]
impl BallistaContext {
    pub fn try_new(_host: &str, _port: u16) -> Result<Self> {
        Err(DataFusionError::NotImplemented(
            "Remote execution not supported. Compile with feature 'ballista' to enable".to_string(),
        ))
    }
    pub async fn sql(&mut self, _sql: &str) -> Result<Arc<DataFrame>> {
        unreachable!()
    }
}
