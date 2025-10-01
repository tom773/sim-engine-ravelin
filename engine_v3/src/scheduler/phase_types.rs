
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub success: bool,
    pub duration_ms: u64,
    pub telemetry: StepTelemetry,
    pub error: Option<String>,
}

impl StepResult {
    pub fn success(duration_ms: u64, telemetry: StepTelemetry) -> Self {
        Self { success: true, duration_ms, telemetry, error: None }
    }

    pub fn failure(duration_ms: u64, error: String) -> Self {
        Self { success: false, duration_ms, telemetry: StepTelemetry::default(), error: Some(error) }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepTelemetry {
    pub metrics: Vec<(String, TelemetryValue)>,
}

impl StepTelemetry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn single<T: Into<TelemetryValue>>(name: &str, value: T) -> Self {
        let mut telemetry = Self::new();
        telemetry.push_metric(name, value);
        telemetry
    }

    pub fn push_metric<T: Into<TelemetryValue>>(&mut self, name: impl Into<String>, value: T) {
        self.metrics.push((name.into(), value.into()));
    }

    pub fn with_metric<T: Into<TelemetryValue>>(mut self, name: impl Into<String>, value: T) -> Self {
        self.push_metric(name, value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TelemetryValue {
    String(String),
    Number(f64),
    Integer(i64),
    UInteger(u64),
    Bool(bool),
}

impl From<String> for TelemetryValue {
    fn from(v: String) -> Self {
        TelemetryValue::String(v)
    }
}

impl From<&str> for TelemetryValue {
    fn from(v: &str) -> Self {
        TelemetryValue::String(v.to_string())
    }
}

impl From<f64> for TelemetryValue {
    fn from(v: f64) -> Self {
        TelemetryValue::Number(v)
    }
}

impl From<i64> for TelemetryValue {
    fn from(v: i64) -> Self {
        TelemetryValue::Integer(v)
    }
}

impl From<u64> for TelemetryValue {
    fn from(v: u64) -> Self {
        TelemetryValue::UInteger(v)
    }
}

impl From<usize> for TelemetryValue {
    fn from(v: usize) -> Self {
        TelemetryValue::UInteger(v as u64)
    }
}

impl From<u32> for TelemetryValue {
    fn from(v: u32) -> Self {
        TelemetryValue::UInteger(v as u64)
    }
}

impl From<i32> for TelemetryValue {
    fn from(v: i32) -> Self {
        TelemetryValue::Integer(v as i64)
    }
}

impl From<bool> for TelemetryValue {
    fn from(v: bool) -> Self {
        TelemetryValue::Bool(v)
    }
}
