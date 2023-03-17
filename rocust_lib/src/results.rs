use csv::{Error as CsvError, IntoInnerError as CsvIntoInnerError, Writer as CsvWriter};
use prettytable::{row, Cell, Row, Table};
use serde::{ser::SerializeStruct, Serialize};
use std::{collections::HashMap, string::FromUtf8Error, time::Duration};
use thiserror::Error as ThisError;

const CONSOLE_HEADERS: [&str; 11] = [
    "TYPE",
    "NAME",
    "TOTAL REQ",
    "FAILED REQ",
    "TOTAL ERR",
    "REQ/S",
    "FAILED REQ/S",
    "TOTAL RES TIME",
    "AVG RES TIME",
    "MIN RES TIME",
    "MAX RES TIME",
];

const CONSOLE_AGR_TYPE_NAME: [&str; 2] = ["", "AGR"];

const FILE_HEADERS: [&str; 11] = [
    "type",
    "name",
    "total_requests",
    "total_failed_requests",
    "total_errors",
    "requests_per_second",
    "failed_requests_per_second",
    "total_response_time",
    "average_response_time",
    "min_response_time",
    "max_response_time",
];

const FILE_AGR_TYPE_NAME: [&str; 2] = ["", "aggregated"];

#[derive(Debug, Default, Clone, Serialize)]
pub struct Results {
    total_requests: u32,
    total_failed_requests: u32,
    total_errors: u32,
    total_response_time: f64,
    average_response_time: f64,
    min_response_time: f64,
    median_response_time: f64,
    max_response_time: f64,
    requests_per_second: f64,
    failed_requests_per_second: f64,
}

impl Results {
    fn add_success(&mut self, response_time: f64) {
        self.total_response_time += response_time;
        self.total_requests += 1;
        if self.min_response_time == 0.0 || response_time < self.min_response_time {
            self.min_response_time = response_time;
        }
        if response_time > self.max_response_time {
            self.max_response_time = response_time;
        }
    }

    fn add_failure(&mut self) {
        self.total_requests += 1;
        self.total_failed_requests += 1;
    }

    fn add_error(&mut self) {
        self.total_errors += 1;
    }

    fn calculate_average_response_time(&mut self) {
        // (total_requests - total_failed_requests) = total_successful_requests
        let total_successful_requests = self.total_requests - self.total_failed_requests;
        self.average_response_time = self.total_response_time / total_successful_requests as f64;
    }
    fn calculate_requests_per_second(&mut self, elapsed: &Duration) {
        let total_requests = self.total_requests;
        let requests_per_second = total_requests as f64 / elapsed.as_secs_f64();
        self.requests_per_second = requests_per_second;
    }

    fn calculate_failed_requests_per_second(&mut self, elapsed: &Duration) {
        let total_failed_requests = self.total_failed_requests;
        let failed_requests_per_second = total_failed_requests as f64 / elapsed.as_secs_f64();
        self.failed_requests_per_second = failed_requests_per_second;
    }

    fn calculate_on_update_interval(&mut self, elapsed: &Duration) {
        self.calculate_average_response_time();
        self.calculate_requests_per_second(elapsed);
        self.calculate_failed_requests_per_second(elapsed);
    }

    pub fn get_total_requests(&self) -> u32 {
        self.total_requests
    }

    pub fn get_total_failed_requests(&self) -> u32 {
        self.total_failed_requests
    }

    pub fn get_total_errors(&self) -> u32 {
        self.total_errors
    }

    pub fn get_total_response_time(&self) -> f64 {
        self.total_response_time
    }

    pub fn get_average_response_time(&self) -> f64 {
        self.average_response_time
    }

    pub fn get_min_response_time(&self) -> f64 {
        self.min_response_time
    }

    pub fn get_median_response_time(&self) -> f64 {
        self.median_response_time
    }

    pub fn get_max_response_time(&self) -> f64 {
        self.max_response_time
    }

    pub fn get_requests_per_second(&self) -> f64 {
        self.requests_per_second
    }

    pub fn get_failed_requests_per_second(&self) -> f64 {
        self.failed_requests_per_second
    }

    fn into_ser_results(self, endpoint_type_name: EndpointTypeName) -> SerResults {
        SerResults {
            endpoint_type_name,
            results: self,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SerResults {
    endpoint_type_name: EndpointTypeName,
    results: Results,
}

impl Serialize for SerResults {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("SerResults", 3)?;
        state.serialize_field("type", &self.endpoint_type_name.r#type)?;
        state.serialize_field("name", &self.endpoint_type_name.name)?;
        state.serialize_field("results", &self.results)?;
        state.end()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct EndpointTypeName {
    pub r#type: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SerAllResults {
    aggrigated_results: Results,
    endpoint_results: Vec<SerResults>,
}

#[derive(Debug, Default, Clone)]
pub struct AllResults {
    aggrigated_results: Results,
    endpoint_results: HashMap<EndpointTypeName, Results>,
}

#[derive(Debug, ThisError)]
pub enum CSVError {
    #[error("FromUtf8Error error: {0}")]
    FromUtf8Error(#[from] FromUtf8Error),

    #[error("CsvError error: {0}")]
    CsvError(#[from] CsvError),

    #[error("IntoInnerError error: {0}")]
    IntoInnerError(Box<CsvIntoInnerError<CsvWriter<Vec<u8>>>>), // a very big type
}

impl From<CsvIntoInnerError<CsvWriter<Vec<u8>>>> for CSVError {
    fn from(err: CsvIntoInnerError<CsvWriter<Vec<u8>>>) -> Self {
        CSVError::IntoInnerError(Box::new(err))
    }
}

// TODO: refactor repeated code
impl AllResults {
    pub(crate) fn add_success(
        &mut self,
        endpoint_type_name: &EndpointTypeName,
        response_time: f64,
    ) {
        self.aggrigated_results.add_success(response_time);
        if let Some(endpoint_results) = self.endpoint_results.get_mut(endpoint_type_name) {
            endpoint_results.add_success(response_time);
        } else {
            let mut endpoint_results = Results::default();
            endpoint_results.add_success(response_time);
            self.endpoint_results
                .insert(endpoint_type_name.clone(), endpoint_results);
        }
    }

    pub(crate) fn add_failure(&mut self, endpoint_type_name: &EndpointTypeName) {
        self.aggrigated_results.add_failure();
        if let Some(endpoint_results) = self.endpoint_results.get_mut(endpoint_type_name) {
            endpoint_results.add_failure();
        } else {
            let mut endpoint_results = Results::default();
            endpoint_results.add_failure();
            self.endpoint_results
                .insert(endpoint_type_name.clone(), endpoint_results);
        }
    }

    pub(crate) fn add_error(&mut self, endpoint_type_name: &EndpointTypeName, _error: &str) {
        self.aggrigated_results.add_error();
        if let Some(endpoint_results) = self.endpoint_results.get_mut(endpoint_type_name) {
            endpoint_results.add_error();
        } else {
            let mut endpoint_results = Results::default();
            endpoint_results.add_error();
            self.endpoint_results
                .insert(endpoint_type_name.clone(), endpoint_results);
        }
    }

    pub(crate) fn calculate_on_update_interval(&mut self, elapsed: &Duration) {
        self.aggrigated_results
            .calculate_on_update_interval(elapsed);
        for (_, endpoint_results) in self.endpoint_results.iter_mut() {
            endpoint_results.calculate_on_update_interval(elapsed);
        }
    }

    pub(crate) fn history_header_csv_string() -> Result<String, CSVError> {
        let mut wtr = CsvWriter::from_writer(vec![]);
        let headers_with_timestamp = [&["timestamp"], &FILE_HEADERS[..]].concat();
        wtr.write_record(&headers_with_timestamp)?;
        let data = String::from_utf8(wtr.into_inner()?)?;
        Ok(data)
    }

    pub(crate) fn current_aggrigated_results_with_timestamp_csv_string(
        &self,
        timestamp: &str,
    ) -> Result<String, CSVError> {
        let mut wtr = CsvWriter::from_writer(vec![]);
        wtr.write_record([
            timestamp,
            FILE_AGR_TYPE_NAME[0],
            FILE_AGR_TYPE_NAME[1],
            &self.aggrigated_results.total_requests.to_string(),
            &self.aggrigated_results.total_failed_requests.to_string(),
            &self.aggrigated_results.total_errors.to_string(),
            &self.aggrigated_results.requests_per_second.to_string(),
            &self
                .aggrigated_results
                .failed_requests_per_second
                .to_string(),
            &self.aggrigated_results.total_response_time.to_string(),
            &self.aggrigated_results.average_response_time.to_string(),
            &self.aggrigated_results.min_response_time.to_string(),
            &self.aggrigated_results.median_response_time.to_string(),
            &self.aggrigated_results.max_response_time.to_string(),
        ])?;
        let data = String::from_utf8(wtr.into_inner()?)?;
        Ok(data)
    }

    pub(crate) fn current_results_csv_string(&self) -> Result<String, CSVError> {
        let mut wtr = CsvWriter::from_writer(vec![]);
        wtr.write_record(FILE_HEADERS)?;
        for (endpoint_type_name, results) in &self.endpoint_results {
            wtr.write_record([
                &endpoint_type_name.r#type,
                &endpoint_type_name.name,
                &results.total_requests.to_string(),
                &results.total_failed_requests.to_string(),
                &results.total_errors.to_string(),
                &results.requests_per_second.to_string(),
                &results.failed_requests_per_second.to_string(),
                &results.total_response_time.to_string(),
                &results.average_response_time.to_string(),
                &results.min_response_time.to_string(),
                &results.max_response_time.to_string(),
            ])?;
        }
        wtr.write_record([
            FILE_AGR_TYPE_NAME[0],
            FILE_AGR_TYPE_NAME[1],
            &self.aggrigated_results.total_requests.to_string(),
            &self.aggrigated_results.total_failed_requests.to_string(),
            &self.aggrigated_results.total_errors.to_string(),
            &self.aggrigated_results.requests_per_second.to_string(),
            &self
                .aggrigated_results
                .failed_requests_per_second
                .to_string(),
            &self.aggrigated_results.total_response_time.to_string(),
            &self.aggrigated_results.average_response_time.to_string(),
            &self.aggrigated_results.min_response_time.to_string(),
            &self.aggrigated_results.max_response_time.to_string(),
        ])?;
        let data = String::from_utf8(wtr.into_inner()?)?;
        Ok(data)
    }

    pub(crate) fn table_string(&self, precision: usize) -> String {
        let mut table = Table::new();
        table.add_row(Row::new(
            CONSOLE_HEADERS.iter().map(|s| Cell::new(s)).collect(),
        ));
        for (endpoint_type_name, results) in &self.endpoint_results {
            table.add_row(row![
                endpoint_type_name.r#type,
                endpoint_type_name.name,
                format!("{:.1$}", results.total_requests, precision),
                format!("{:.1$}", results.total_failed_requests, precision),
                format!("{:.1$}", results.total_errors, precision),
                format!("{:.1$}", results.requests_per_second, precision),
                format!("{:.1$}", results.failed_requests_per_second, precision),
                format!("{:.1$}", results.total_response_time, precision),
                format!("{:.1$}", results.average_response_time, precision),
                format!("{:.1$}", results.min_response_time, precision),
                format!("{:.1$}", results.max_response_time, precision),
            ]);
        }
        table.add_row(row![
            CONSOLE_AGR_TYPE_NAME[0],
            CONSOLE_AGR_TYPE_NAME[1],
            format!("{:.1$}", self.aggrigated_results.total_requests, precision),
            format!(
                "{:.1$}",
                self.aggrigated_results.total_failed_requests, precision
            ),
            format!("{:.1$}", self.aggrigated_results.total_errors, precision),
            format!(
                "{:.1$}",
                self.aggrigated_results.requests_per_second, precision
            ),
            format!(
                "{:.1$}",
                self.aggrigated_results.failed_requests_per_second, precision
            ),
            format!(
                "{:.1$}",
                self.aggrigated_results.total_response_time, precision
            ),
            format!(
                "{:.1$}",
                self.aggrigated_results.average_response_time, precision
            ),
            format!(
                "{:.1$}",
                self.aggrigated_results.min_response_time, precision
            ),
            format!(
                "{:.1$}",
                self.aggrigated_results.max_response_time, precision
            ),
        ]);
        table.to_string()
    }

    pub fn get_by_type(&self, r#type: &str) -> Vec<&Results> {
        let mut results = Vec::new();
        for (endpoint_type_name, result) in &self.endpoint_results {
            if endpoint_type_name.r#type == r#type {
                results.push(result);
            }
        }
        results
    }

    pub fn get_by_name(&self, name: &str) -> Vec<&Results> {
        let mut results = Vec::new();
        for (endpoint_type_name, result) in &self.endpoint_results {
            if endpoint_type_name.name == name {
                results.push(result);
            }
        }
        results
    }

    pub fn get_by_type_and_name(&self, r#type: &str, name: &str) -> Option<&Results> {
        self.endpoint_results.get(&EndpointTypeName {
            r#type: r#type.to_string(),
            name: name.to_string(),
        })
    }

    pub fn get_aggrigated_results(&self) -> &Results {
        &self.aggrigated_results
    }

    pub fn get_endpoint_results(&self) -> &HashMap<EndpointTypeName, Results> {
        &self.endpoint_results
    }
}

impl From<AllResults> for SerAllResults {
    fn from(all_results: AllResults) -> SerAllResults {
        let aggrigated_results = all_results.aggrigated_results;
        let endpoint_results = all_results
            .endpoint_results
            .into_iter()
            .map(|(endpoint_type_name, results)| results.into_ser_results(endpoint_type_name))
            .collect();
        SerAllResults {
            aggrigated_results,
            endpoint_results,
        }
    }
}
