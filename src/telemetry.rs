use time::OffsetDateTime;

#[derive(Clone, Debug)]
pub struct Frame {
    pub timestamp: OffsetDateTime,
    pub data_points: Vec<DataPoint>,
}

impl Frame {
    pub fn new(timestamp: OffsetDateTime, data: &[(String, f32)]) -> Self {
        Self {
            timestamp,
            data_points: data
                .iter()
                .map(|(n, v)| DataPoint::new(n, timestamp, *v))
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DataPoint {
    pub name: String,
    pub timestamp: OffsetDateTime,
    pub data: f32,
}

impl DataPoint {
    pub fn new(name: &str, timestamp: OffsetDateTime, data: f32) -> Self {
        Self {
            name: name.to_string(),
            timestamp,
            data,
        }
    }
}
