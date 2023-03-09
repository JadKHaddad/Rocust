use prometheus_client::{
    encoding::{text::encode, EncodeLabelSet},
    metrics::{counter::Counter, family::Family, gauge::Gauge},
    registry::Registry,
};
use std::{fmt::Error as FmtError, sync::atomic::AtomicU64};

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, EncodeLabelSet)]
pub(crate) struct Label {
    pub endpoint_type: String,
    pub endpoint_name: String,
    pub user_id: u64,
    pub user_name: String,
}

pub(crate) struct PrometheusExporter {
    registry: Registry,
    request_counter: Family<Label, Counter<u64>>,
    failure_counter: Family<Label, Counter<u64>>,
    error_counter: Family<Label, Counter<u64>>,
    response_time_gauge: Family<Label, Gauge<f64, AtomicU64>>,
}

impl PrometheusExporter {
    pub(crate) fn new() -> Self {
        let mut registry = Registry::default();
        let request_counter = Family::<Label, Counter<u64>>::default();
        registry.register(
            "rocust_request",
            "Total number of requests",
            request_counter.clone(),
        );
        let failure_counter = Family::<Label, Counter<u64>>::default();
        registry.register(
            "rocust_failure",
            "Total number of failures",
            failure_counter.clone(),
        );
        let error_counter = Family::<Label, Counter<u64>>::default();
        registry.register(
            "rocust_error",
            "Total number of errors",
            error_counter.clone(),
        );
        let response_time_gauge = Family::<Label, Gauge<f64, AtomicU64>>::default();
        registry.register(
            "rocust_response_time",
            "Response time",
            response_time_gauge.clone(),
        );
        Self {
            registry,
            request_counter,
            failure_counter,
            error_counter,
            response_time_gauge,
        }
    }

    pub(crate) fn get_metrics(&self) -> Result<String, FmtError> {
        let mut buffer = String::new();
        encode(&mut buffer, &self.registry)?;
        Ok(buffer)
    }

    pub(crate) fn add_success(&self, label: Label, response_time: f64) {
        self.request_counter.get_or_create(&label).inc();
        self.response_time_gauge
            .get_or_create(&label)
            .set(response_time);
    }

    pub(crate) fn add_failure(&self, label: Label) {
        self.request_counter.get_or_create(&label).inc();
        self.failure_counter.get_or_create(&label).inc();
    }

    pub(crate) fn add_error(&self, label: Label) {
        self.request_counter.get_or_create(&label).inc();
        self.error_counter.get_or_create(&label).inc();
    }
}
