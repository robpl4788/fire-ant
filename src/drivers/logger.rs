use core::fmt::Write;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use heapless::{String, Vec};

const MAX_SERIES: usize = 10;

#[derive(Clone, Copy, Debug)]
struct Series {
    min: f32,
    max: f32,
    mean: f32,
    value_count: u32,
    key: &'static str,
}

impl Series {
    fn new(key: &'static str, value: f32) -> Self {
        Series {
            key,
            min: value,
            max: value,
            mean: value,
            value_count: 0,
        }
    }

    fn add_value(&mut self, value: f32) {
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
        self.mean = (self.mean * self.value_count as f32 + value) / (self.value_count as f32 + 1.0);
        self.value_count += 1;
    }

    fn get_min(&self) -> f32 {
        self.min
    }

    fn get_max(&self) -> f32 {
        self.max
    }

    fn get_mean(&self) -> f32 {
        self.mean
    }

    fn get_key(&self) -> &'static str {
        self.key
    }

    fn get_value_count(&self) -> u32 {
        self.value_count
    }
}

pub(crate) struct Logger {
    all_series: Vec<Series, MAX_SERIES>,
}

impl Logger {
    pub const fn new() -> Self {
        Logger {
            all_series: Vec::new(),
        }
    }

    pub fn log_value(&mut self, key: &'static str, value: f32) {
        for series in &mut self.all_series {
            if series.get_key() == key {
                series.add_value(value);
                return;
            }
        }

        self.all_series.push(Series::new(key, value)).unwrap();
    }

    pub fn get_data(&mut self) -> [u8; 256] {
        let mut data = String::<256>::new();

        write!(data, "{{\"all_series\":[").unwrap();

        for (i, series) in self.all_series.iter().enumerate() {
            if i != 0 {
                write!(data, ",").unwrap();
            }
            write!(
                data,
                "{{\"key\":\"{}\",\"min\":{:.3},\"max\":{:.3},\"mean\":{:.3},\"value_count\":{} }}",
                series.key, series.min, series.max, series.mean, series.value_count,
            )
            .unwrap();
        }

        write!(data, "]}}\n").unwrap();
        // info!("{}", data.as_str());

        self.all_series.clear();

        let mut buf = [0u8; 256];
        let bytes = data.as_bytes();

        let len = bytes.len().min(256);
        buf[..len].copy_from_slice(&bytes[..len]);

        buf
    }
}

pub static LOGGER: Mutex<CriticalSectionRawMutex, Logger> = Mutex::new(Logger::new());
