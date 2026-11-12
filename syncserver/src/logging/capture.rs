//! Captures formatted log output in memory, for tests.

use std::{
    io,
    sync::{Arc, Mutex},
};

use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
pub struct Capture(Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Capture {
    type Writer = Capture;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl Capture {
    pub fn lines(&self) -> Vec<String> {
        String::from_utf8(self.0.lock().unwrap().clone())
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// Lines with the timestamp replaced by 0, after checking it is an integer.
    pub fn normalized(&self) -> Vec<String> {
        self.lines()
            .iter()
            .map(|line| {
                let value: serde_json::Value = serde_json::from_str(line).unwrap();
                let timestamp = value["Timestamp"].as_i64().expect("an i64 Timestamp");
                line.replacen(
                    &format!("\"Timestamp\":{timestamp},"),
                    "\"Timestamp\":0,",
                    1,
                )
            })
            .collect()
    }

    pub fn json(&self) -> Vec<serde_json::Value> {
        self.lines()
            .iter()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}
