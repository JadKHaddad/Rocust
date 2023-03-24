use crate::{
    fs::{timestamped_writer::TimeStapmedWriter, writer::Writer},
    prometheus_exporter::PrometheusExporter,
    results::AllResults,
    utils, TestConfig,
};

#[derive(Clone)]
pub(crate) struct Writers {
    current_results_writer: Option<Writer>,
    results_history_writer: Option<Writer>,
    summary_writer: Option<Writer>,
    prometheus_current_metrics_writer: Option<Writer>,
    prometheus_metrics_history_writer: Option<TimeStapmedWriter>,
}

impl Writers {
    pub async fn new(test_config: &TestConfig) -> Self {
        let current_results_writer =
            if let Some(current_results_file) = &test_config.current_results_file {
                match Writer::from_str(current_results_file).await {
                    Ok(writer) => Some(writer),
                    Err(error) => {
                        tracing::error!(%error, "Failed to create writer for current results file");
                        None
                    }
                }
            } else {
                None
            };
        let results_history_writer =
            if let Some(results_history_file) = &test_config.results_history_file {
                match Writer::from_str(results_history_file).await {
                    Ok(writer) => {
                        // write header
                        let header = AllResults::history_header_csv_string();
                        match header {
                            Ok(header) => match writer.write_all(header.as_bytes()).await {
                                Ok(_) => Some(writer),
                                Err(error) => {
                                    tracing::error!(
                                        %error,
                                        "Failed to write header to results history file"
                                    );
                                    None
                                }
                            },
                            Err(error) => {
                                tracing::error!(
                                    %error,
                                    "Failed to create header for results history file",
                                );
                                None
                            }
                        }
                    }
                    Err(error) => {
                        tracing::error!(%error, "Failed to create writer for results history file");
                        None
                    }
                }
            } else {
                None
            };
        let summary_writer = if let Some(summary_file) = &test_config.summary_file {
            match Writer::from_str(summary_file).await {
                Ok(writer) => Some(writer),
                Err(error) => {
                    tracing::error!(%error, "Failed to create writer for summary file");
                    None
                }
            }
        } else {
            None
        };
        let prometheus_current_metrics_writer = if let Some(prometheus_current_metrics_file) =
            &test_config.prometheus_current_metrics_file
        {
            match Writer::from_str(prometheus_current_metrics_file).await {
                Ok(writer) => Some(writer),
                Err(error) => {
                    tracing::error!(
                        %error,
                        "Failed to create writer for prometheus current metrics file"
                    );
                    None
                }
            }
        } else {
            None
        };
        let prometheus_metrics_history_writer = if let Some(prometheus_metrics_history_folder) =
            &test_config.prometheus_metrics_history_folder
        {
            match TimeStapmedWriter::from_str(
                prometheus_metrics_history_folder,
                String::from("metrics.prom"),
            )
            .await
            {
                Ok(writer) => Some(writer),
                Err(error) => {
                    tracing::error!(
                        %error,
                        "Failed to create writer for prometheus history metrics"
                    );
                    None
                }
            }
        } else {
            None
        };
        Self {
            current_results_writer,
            results_history_writer,
            summary_writer,
            prometheus_current_metrics_writer,
            prometheus_metrics_history_writer,
        }
    }

    async fn write_current_results(&self, all_results: &AllResults) {
        if let Some(writer) = &self.current_results_writer {
            let csv_string = all_results.current_results_csv_string();
            match csv_string {
                Ok(csv_string) => {
                    if let Err(error) = writer.write_all(csv_string.as_bytes()).await {
                        tracing::error!(%error, "Error writing to csv");
                    }
                }
                Err(error) => {
                    tracing::error!(%error, "Error getting csv string");
                }
            }
        }
    }

    async fn write_results_history(&self, all_results: &AllResults) {
        if let Some(writer) = &self.results_history_writer {
            match utils::get_timestamp_as_millis_as_string() {
                Ok(timestamp) => {
                    let csv_string = all_results
                        .current_aggrigated_results_with_timestamp_csv_string(&timestamp);
                    match csv_string {
                        Ok(csv_string) => {
                            if let Err(error) = writer.append_all(csv_string.as_bytes()).await {
                                tracing::error!(%error, "Error writing to csv");
                            }
                        }
                        Err(error) => {
                            tracing::error!(%error, "Error getting csv string");
                        }
                    }
                }
                Err(error) => {
                    tracing::error!(%error, "Error getting timestamp");
                }
            }
        }
    }

    async fn write_prometheus_current_metrics(&self, prometheus_exporter: &PrometheusExporter) {
        if let Some(writer) = &self.prometheus_current_metrics_writer {
            let prometheus_metrics_string = prometheus_exporter.get_metrics();
            match prometheus_metrics_string {
                Ok(prometheus_metrics_string) => {
                    if let Err(error) = writer.write_all(prometheus_metrics_string.as_bytes()).await
                    {
                        tracing::error!(%error, "Error writing prometheus current metrics");
                    }
                }
                Err(error) => {
                    tracing::error!(%error, "Error getting prometheus string");
                }
            }
        }
    }

    async fn write_prometheus_metrics_history(&self, prometheus_exporter: &PrometheusExporter) {
        if let Some(writer) = &self.prometheus_metrics_history_writer {
            let prometheus_metrics_string = prometheus_exporter.get_metrics();
            match prometheus_metrics_string {
                Ok(prometheus_metrics_string) => {
                    if let Err(error) = writer.write_all(prometheus_metrics_string.as_bytes()).await
                    {
                        tracing::error!(%error, "Error writing prometheus metrics history");
                    }
                }
                Err(error) => {
                    tracing::error!(%error, "Error getting prometheus string");
                }
            }
        }
    }

    pub(crate) async fn write_on_update_interval(
        &self,
        all_results: &AllResults,
        prometheus_exporter: &PrometheusExporter,
    ) {
        self.write_current_results(all_results).await;
        self.write_results_history(all_results).await;
        self.write_prometheus_current_metrics(prometheus_exporter)
            .await;
        self.write_prometheus_metrics_history(prometheus_exporter)
            .await;
    }

    pub(crate) fn get_summary_writer(&self) -> &Option<Writer> {
        &self.summary_writer
    }
}
