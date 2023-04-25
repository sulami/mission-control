use time::OffsetDateTime;

#[derive(Clone, Debug)]
pub struct DataPoint {
    pub name: String,
    pub timestamp: OffsetDateTime,
    pub data: f32,
}

impl DataPoint {
    pub fn new(name: &str, timestamp: &OffsetDateTime, data: f32) -> Self {
        Self {
            name: name.to_string(),
            timestamp: *timestamp,
            data,
        }
    }
}
