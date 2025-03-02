use std::env;
use std::path::PathBuf;

pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub base_url: String,
    pub templates_path: String,
}

impl Default for Config {
    fn default() -> Self {
        let host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("SERVER_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3030);
        
        let base_url = env::var("BASE_URL")
            .unwrap_or_else(|_| format!("http://{}:{}", host, port));
        
        // Use a relative path that works on any OS, or get from environment
        let templates_path = env::var("TEMPLATES_PATH").unwrap_or_else(|_| {
            let mut path = PathBuf::from(env::current_dir().unwrap_or_default());
            path.push("templates");
            path.to_string_lossy().into_owned()
        });
        
        Self {
            server_host: host,
            server_port: port,
            base_url,
            templates_path,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn get_base_url(&self) -> &str {
        &self.base_url
    }
}
