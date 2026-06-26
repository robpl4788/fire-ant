use core::fmt::Write;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use heapless::{String, Vec};

const MAX_SERIES: usize = 10;
const JSON_BUFFER_SIZE: usize = 256;

/// Statistics for a logged data series
#[derive(Clone, Copy, Debug)]
struct Series {
    key: &'static str,
    min: f32,
    max: f32,
    mean: f32,
    value_count: u32,
}

impl Series {
    /// Create a new series with an initial value
    fn new(key: &'static str, value: f32) -> Self {
        Series {
            key,
            min: value,
            max: value,
            mean: value,
            value_count: 1,
        }
    }

    /// Add a new value to the series
    fn add_value(&mut self, value: f32) {
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
        // Update mean incrementally
        self.mean = (self.mean * self.value_count as f32 + value) / (self.value_count as f32 + 1.0);
        self.value_count += 1;
    }
}

/// Data logger that collects statistics for multiple time-series
pub(crate) struct Logger {
    series_list: Vec<Series, MAX_SERIES>,
}

impl Logger {
    pub const fn new() -> Self {
        Logger {
            series_list: Vec::new(),
        }
    }

    /// Log a value for a given key
    pub fn log_value(&mut self, key: &'static str, value: f32) {
        // Update existing series or create new one
        for series in &mut self.series_list {
            if series.key == key {
                series.add_value(value);
                return;
            }
        }

        let _ = self.series_list.push(Series::new(key, value));
    }

    /// Get logged data as JSON-formatted bytes
    pub fn get_data(&mut self) -> [u8; JSON_BUFFER_SIZE] {
        let mut json = String::<JSON_BUFFER_SIZE>::new();

        let _ = write!(json, "{{\"all_series\":[");

        for (i, series) in self.series_list.iter().enumerate() {
            if i != 0 {
                let _ = write!(json, ",");
            }
            let _ = write!(
                json,
                "{{\"key\":\"{}\",\"min\":{:.3},\"max\":{:.3},\"mean\":{:.3},\"value_count\":{}}}",
                series.key, series.min, series.max, series.mean, series.value_count,
            );
        }

        let _ = write!(json, "]}}\n");
        self.series_list.clear();

        // Convert to fixed-size buffer
        let mut buf = [0u8; JSON_BUFFER_SIZE];
        let bytes = json.as_bytes();
        let len = bytes.len().min(JSON_BUFFER_SIZE);
        buf[..len].copy_from_slice(&bytes[..len]);

        buf
    }
}

pub type LoggerMutex = Mutex<CriticalSectionRawMutex, Logger>;
pub static LOGGER: LoggerMutex = Mutex::new(Logger::new());
