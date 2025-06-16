use sonic_rs::Serialize;

#[derive(Serialize)]
pub struct SerializedLabel {
    pub name: String,
    pub address: usize,
}

/// Main struct representing the compiled program
/// TODO add more fields for debugging symbols and metadata.
#[derive(Serialize)]
pub struct SerializedProgram {
    pub compiler_version: String,
    pub data: Vec<Vec<String>>,
    pub labels: Vec<SerializedLabel>,
}

impl SerializedProgram {
    pub fn new(data: Vec<Vec<String>>, labels: Vec<SerializedLabel>) -> Self {
        Self {
            data,
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            labels,
        }
    }

    pub fn to_json(&self) -> String {
        sonic_rs::to_string_pretty(self).unwrap()
    }
}
