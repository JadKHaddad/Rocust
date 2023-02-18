use prettytable::{row, Cell, Row, Table};
use serde::Serialize;
use std::{collections::HashMap, time::Duration};

const HEADERS: [&'static str; 11] = [
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
const AGR_TYPE_NAME: [&'static str; 2] = ["", "AGR"];

#[derive(Debug, Default, Clone, Serialize)]
pub struct Results {
    pub total_requests: u32,
    pub total_failed_requests: u32,
    pub total_errors: u32,
    pub total_response_time: f64,
    pub average_response_time: f64,
    pub min_response_time: f64,
    pub median_response_time: f64,
    pub max_response_time: f64,
    pub requests_per_second: f64,
    pub failed_requests_per_second: f64,
}

impl Results {
    fn add_success(&mut self, response_time: f64) {
        self.total_response_time += response_time;
        self.total_requests += 1;
        self.average_response_time = self.total_response_time / self.total_requests as f64;
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

    fn calculate_per_second(&mut self, elapsed: &Duration) {
        self.calculate_requests_per_second(elapsed);
        self.calculate_failed_requests_per_second(elapsed);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize)]
pub struct EndpointTypeName(pub String, pub String);

#[derive(Debug, Default, Clone, Serialize)]
pub struct AllResults {
    aggrigated_results: Results,
    endpoint_results: HashMap<EndpointTypeName, Results>,
}

impl AllResults {
    pub fn add_success(&mut self, endpoint_type_name: EndpointTypeName, response_time: f64) {
        self.aggrigated_results.add_success(response_time);
        if let Some(endpoint_results) = self.endpoint_results.get_mut(&endpoint_type_name) {
            endpoint_results.add_success(response_time);
        } else {
            let mut endpoint_results = Results::default();
            endpoint_results.add_success(response_time);
            self.endpoint_results
                .insert(endpoint_type_name, endpoint_results);
        }
    }

    pub fn add_failure(&mut self, endpoint_type_name: EndpointTypeName) {
        self.aggrigated_results.add_failure();
        if let Some(endpoint_results) = self.endpoint_results.get_mut(&endpoint_type_name) {
            endpoint_results.add_failure();
        } else {
            let mut endpoint_results = Results::default();
            endpoint_results.add_failure();
            self.endpoint_results
                .insert(endpoint_type_name, endpoint_results);
        }
    }

    pub fn add_error(&mut self, endpoint_type_name: EndpointTypeName, _error: String) {
        self.aggrigated_results.add_error();
        if let Some(endpoint_results) = self.endpoint_results.get_mut(&endpoint_type_name) {
            endpoint_results.add_error();
        } else {
            let mut endpoint_results = Results::default();
            endpoint_results.add_error();
            self.endpoint_results
                .insert(endpoint_type_name, endpoint_results);
        }
    }

    pub fn calculate_per_second(&mut self, elapsed: &Duration) {
        self.aggrigated_results.calculate_per_second(elapsed);
        for (_, endpoint_results) in self.endpoint_results.iter_mut() {
            endpoint_results.calculate_per_second(elapsed);
        }
    }

    pub fn csv_string(&self) -> String {
        let mut wtr = csv::Writer::from_writer(vec![]);
        let _ = wtr.write_record(&HEADERS);
        for (endpoint_type_name, results) in &self.endpoint_results {
            let _ = wtr.write_record(&[
                &endpoint_type_name.0,
                &endpoint_type_name.1,
                &results.total_requests.to_string(),
                &results.total_failed_requests.to_string(),
                &results.total_errors.to_string(),
                &results.requests_per_second.to_string(),
                &results.failed_requests_per_second.to_string(),
                &results.total_response_time.to_string(),
                &results.average_response_time.to_string(),
                &results.min_response_time.to_string(),
                &results.max_response_time.to_string(),
            ]);
        }
        let _ = wtr.write_record(&[
            AGR_TYPE_NAME[0],
            AGR_TYPE_NAME[1],
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
        ]);
        let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
        data
    }

    pub fn table_string(&self) -> String {
        let mut table = Table::new();
        table.add_row(Row::new(HEADERS.iter().map(|s| Cell::new(s)).collect()));
        for (endpoint_type_name, results) in &self.endpoint_results {
            table.add_row(row![
                endpoint_type_name.0,
                endpoint_type_name.1,
                results.total_requests,
                results.total_failed_requests,
                results.total_errors,
                results.requests_per_second,
                results.failed_requests_per_second,
                results.total_response_time,
                results.average_response_time,
                results.min_response_time,
                results.max_response_time,
            ]);
        }
        table.add_row(row![
            AGR_TYPE_NAME[0],
            AGR_TYPE_NAME[1],
            self.aggrigated_results.total_requests,
            self.aggrigated_results.total_failed_requests,
            self.aggrigated_results.total_errors,
            self.aggrigated_results.requests_per_second,
            self.aggrigated_results.failed_requests_per_second,
            self.aggrigated_results.total_response_time,
            self.aggrigated_results.average_response_time,
            self.aggrigated_results.min_response_time,
            self.aggrigated_results.max_response_time,
        ]);
        table.to_string()
    }

    pub fn get_by_type(&self, r#type: &str) -> Vec<&Results> {
        let mut results = Vec::new();
        for (endpoint_type_name, result) in &self.endpoint_results {
            if endpoint_type_name.0 == r#type {
                results.push(result);
            }
        }
        results
    }

    pub fn get_by_name(&self, name: &str) -> Vec<&Results> {
        let mut results = Vec::new();
        for (endpoint_type_name, result) in &self.endpoint_results {
            if endpoint_type_name.1 == name {
                results.push(result);
            }
        }
        results
    }

    pub fn get_by_type_and_name(&self, r#type: &str, name: &str) -> Option<&Results> {
        self.endpoint_results
            .get(&EndpointTypeName(r#type.to_string(), name.to_string()))
    }

    pub fn get_aggrigated(&self) -> &Results {
        &self.aggrigated_results
    }
}
