pub struct LandStriderConfig {
    server: String,
    port: u32,
}

impl LandStriderConfig {
    pub fn new(server: &str, port: u32) -> Self {
        Self {
            server: server.to_string(),
            port,
        }
    }

    pub fn get_base_url(&self) -> String {
        format!("http://{}:{}", self.server, self.port)
    }
}
