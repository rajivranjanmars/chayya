use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct User {
    pub user_id: String,
    pub device_id: String,
    pub name: String,
    pub email: String,
    pub mobile: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Clone)]
pub struct Device {
    pub device_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Clone)]
pub struct Scan {
    pub scan_id: String,
    pub short_url: String,
    pub user_id: String,
    pub device_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone)]
pub struct Database {
    pub shortened_links: Arc<Mutex<HashMap<String, (String, DateTime<Utc>)>>>,
    pub users: Arc<Mutex<HashMap<String, User>>>,
    pub devices: Arc<Mutex<HashMap<String, Device>>>,
    pub scans: Arc<Mutex<HashMap<String, Scan>>>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            shortened_links: Arc::new(Mutex::new(HashMap::new())),
            users: Arc::new(Mutex::new(HashMap::new())),
            devices: Arc::new(Mutex::new(HashMap::new())),
            scans: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct UserForm {
    pub short_id: String,
    pub device_id: String,
    pub name: String,
    pub email: String,
    pub mobile: String,
}

#[derive(Deserialize)]
pub struct CreateShortenRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct CreateShortenResponse {
    pub short_url: String,
    pub timestamp: DateTime<Utc>,
}
