mod config;
mod db;
mod error;

use warp::Filter;
use std::sync::Arc;
use serde_json::json;
use nanoid::nanoid;
use chrono::Utc;
use handlebars::Handlebars;
// Remove unused imports
use std::path::PathBuf;

use config::Config;
use db::{Database, User, Device, Scan, UserForm, CreateShortenRequest, CreateShortenResponse};
use error::AppError;

#[tokio::main]
async fn main() {
    // Initialize configuration
    let config = Config::new();
    
    // Initialize template engine
    let mut handlebars = Handlebars::new();
    
    // Register templates from the templates directory using PathBuf for cross-platform paths
    let templates_path = PathBuf::from(&config.templates_path);
    
    println!("Loading templates from: {}", templates_path.display());
    
    handlebars.register_template_file("check_device", templates_path.join("check_device.html"))
        .expect("Failed to register check_device template");
    handlebars.register_template_file("user_form", templates_path.join("user_form.html"))
        .expect("Failed to register user_form template");
    handlebars.register_template_file("new_device_form", templates_path.join("new_device_form.html"))
        .expect("Failed to register new_device_form template");
    handlebars.register_template_file("redirect", templates_path.join("redirect.html"))
        .expect("Failed to register redirect template");
    
    let handlebars_arc = Arc::new(handlebars);

    // Initialize database
    let db = Database::new();
    
    // Clone config for filters - clone it before using in filters
    let config_arc = Arc::new(config);
    
    // Use a clone for the filter
    let with_config = {
        let config_arc = config_arc.clone();
        warp::any().map(move || config_arc.clone())
    };

    // Set up routes
    let with_hbs = warp::any().map(move || handlebars_arc.clone());
    
    let shorten = warp::path!("shorten")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_db(db.clone()))
        .and(with_config.clone())
        .and_then(handle_shorten);

    let redirect = warp::path!(String)
        .and(with_db(db.clone()))
        .and(with_hbs.clone())
        .and_then(handle_redirect);

    let user_form = warp::path!("user_form")
        .and(warp::post())
        .and(warp::body::form())
        .and(with_db(db.clone()))
        .and(with_hbs.clone())
        .and_then(handle_user_form);

    let check_device = warp::path!("check_device")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_db(db.clone()))
        .and(with_hbs.clone())
        .and_then(handle_check_device);

    let get_form = warp::path!("get_form" / String)
        .and(with_db(db.clone()))
        .and(with_hbs.clone())
        .and_then(handle_get_form);

    let direct_scan = warp::path!("direct_scan")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_db(db.clone()))
        .and_then(handle_direct_scan);

    let visualize_db = warp::path!("visualize_db")
        .and(with_db(db.clone()))
        .and_then(handle_visualize_db);

    let routes = shorten
        .or(redirect)
        .or(user_form)
        .or(check_device)
        .or(get_form)
        .or(direct_scan)
        .or(visualize_db);
    
    // Add CORS headers to all responses
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "OPTIONS"])
        .allow_headers(vec!["Content-Type", "Authorization"]);

    let routes = routes.with(cors);

    println!("Server starting at {}:{}", config_arc.server_host, config_arc.server_port);
    
    // Fix the parse method by specifying the type
    let host = config_arc.server_host.parse::<std::net::IpAddr>().unwrap();
    warp::serve(routes).run((host, config_arc.server_port)).await;
}

fn with_db(db: Database) -> impl Filter<Extract = (Database,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || db.clone())
}

async fn handle_shorten(
    body: CreateShortenRequest, 
    db: Database,
    config: Arc<Config>
) -> Result<impl warp::Reply, warp::Rejection> {
    let short_id = nanoid!(10); // 
    let short_url = format!("{}/{}", config.base_url, short_id);
    let timestamp = Utc::now();

    if let Ok(mut links) = db.shortened_links.lock() {
        links.insert(short_id.clone(), (body.url, timestamp));
    } else {
        return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        )));
    }

    let response = CreateShortenResponse { short_url, timestamp };
    Ok(warp::reply::json(&response))
}

async fn handle_redirect(
    short_id: String, 
    db: Database,
    hbs: Arc<Handlebars<'_>> // Add lifetime parameter
) -> Result<impl warp::Reply, warp::Rejection> {
    // Verify shortened link exists
    if let Ok(links) = db.shortened_links.lock() {
        if !links.contains_key(&short_id) {
            return Err(warp::reject::custom(AppError::NotFound(
                format!("Short URL not found: {}", short_id)
            )));
        }
    } else {
        return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        )));
    }
    
    // Render the check_device template
    let data = json!({
        "short_id": short_id
    });
    
    let html = hbs.render("check_device", &data)
        .map_err(|e| warp::reject::custom(AppError::TemplateError(e.to_string())))?;
    
    Ok(warp::reply::html(html))
}

async fn handle_check_device(
    params: serde_json::Value,
    db: Database,
    hbs: Arc<Handlebars<'_>> // Add lifetime parameter
) -> Result<impl warp::Reply, warp::Rejection> {
    // Validate request parameters
    let device_id = params["device_id"].as_str()
        .ok_or_else(|| warp::reject::custom(AppError::ValidationError("Missing device_id".to_string())))?;
        
    let short_id = params["short_id"].as_str()
        .ok_or_else(|| warp::reject::custom(AppError::ValidationError("Missing short_id".to_string())))?;
    
    // Check if device exists in our database
    let device_exists = if let Ok(devices) = db.devices.lock() {
        devices.contains_key(device_id)
    } else {
        return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        )));
    };
    
    // If device doesn't exist in our database, store it
    if !device_exists {
        if let Ok(mut devices) = db.devices.lock() {
            devices.insert(device_id.to_string(), Device {
                device_id: device_id.to_string(),
                created_at: Utc::now(),
            });
        } else {
            return Err(warp::reject::custom(AppError::DatabaseError(
                "Failed to acquire database lock".to_string()
            )));
        }
    }

    // Render the user form template
    let data = json!({
        "short_id": short_id,
        "device_id": device_id
    });
    
    let html = hbs.render("user_form", &data)
        .map_err(|e| warp::reject::custom(AppError::TemplateError(e.to_string())))?;
    
    Ok(warp::reply::html(html))
}

async fn handle_get_form(
    short_id: String,
    db: Database,
    hbs: Arc<Handlebars<'_>> // Add lifetime parameter
) -> Result<impl warp::Reply, warp::Rejection> {
    // Generate a new device ID
    let device_id = nanoid!(10);
    
    // Store the device in the database
    if let Ok(mut devices) = db.devices.lock() {
        devices.insert(device_id.clone(), Device {
            device_id: device_id.clone(),
            created_at: Utc::now(),
        });
    } else {
        return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        )));
    }

    // Render the new device form template
    let data = json!({
        "short_id": short_id,
        "device_id": device_id
    });
    
    let html = hbs.render("new_device_form", &data)
        .map_err(|e| warp::reject::custom(AppError::TemplateError(e.to_string())))?;
    
    Ok(warp::reply::html(html))
}

async fn handle_user_form(
    form: UserForm,
    db: Database,
    hbs: Arc<Handlebars<'_>> // Add lifetime parameter
) -> Result<impl warp::Reply, warp::Rejection> {
    // Validate form data
    if form.name.trim().is_empty() || form.email.trim().is_empty() || form.mobile.trim().is_empty() {
        return Err(warp::reject::custom(AppError::ValidationError(
            "All fields are required".to_string()
        )));
    }
    
    // Generate a user ID
    let user_id = nanoid!(10);
    
    // Create user record
    let user = User {
        user_id: user_id.clone(),
        device_id: form.device_id.clone(),
        name: form.name.clone(),
        email: form.email.clone(),
        mobile: form.mobile.clone(),
        created_at: Utc::now(),
    };

    // Store device in devices table if not already there
    if let Ok(mut devices) = db.devices.lock() {
        if !devices.contains_key(&form.device_id) {
            devices.insert(form.device_id.clone(), Device {
                device_id: form.device_id.clone(),
                created_at: Utc::now(),
            });
        }
    } else {
        return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        )));
    }

    // Store user in users table
    if let Ok(mut users) = db.users.lock() {
        users.insert(user_id.clone(), user);
    } else {
        return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        )));
    }

    // Get the target URL
    let url = match db.shortened_links.lock() {
        Ok(links) => match links.get(&form.short_id) {
            Some((url, _timestamp)) => url.clone(),
            None => return Err(warp::reject::custom(AppError::NotFound(
                format!("Short URL not found: {}", form.short_id)
            ))),
        },
        Err(_) => return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        ))),
    }; // Added semicolon here

    // Create a scan record
    let scan_id = nanoid!(10);
    let timestamp = Utc::now();
    let scan = Scan {
        scan_id: scan_id.clone(),
        short_url: form.short_id.clone(),
        user_id: user_id.clone(),
        device_id: form.device_id.clone(),
        timestamp,
    };

    // Store scan in scans table
    if let Ok(mut scans) = db.scans.lock() {
        scans.insert(scan_id.clone(), scan);
    } else {
        return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        )));
    }

    // Render the redirect template
    let data = json!({
        "device_id": form.device_id,
        "user_id": user_id,
        "name": form.name,
        "email": form.email,
        "mobile": form.mobile,
        "scan_id": scan_id,
        "short_url": form.short_id,
        "timestamp": timestamp.to_rfc3339(),
        "url": url
    });
    
    let html = hbs.render("redirect", &data)
        .map_err(|e| warp::reject::custom(AppError::TemplateError(e.to_string())))?;
    
    Ok(warp::reply::html(html))
}

async fn handle_direct_scan(
    params: serde_json::Value,
    db: Database,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Validate request parameters
    let device_id = params["device_id"].as_str()
        .ok_or_else(|| warp::reject::custom(AppError::ValidationError("Missing device_id".to_string())))?;
        
    let user_id = params["user_id"].as_str()
        .ok_or_else(|| warp::reject::custom(AppError::ValidationError("Missing user_id".to_string())))?;
        
    let short_id = params["short_id"].as_str()
        .ok_or_else(|| warp::reject::custom(AppError::ValidationError("Missing short_id".to_string())))?;

    // Get the target URL
    let url = match db.shortened_links.lock() {
        Ok(links) => match links.get(short_id) {
            Some((url, _timestamp)) => url.clone(),
            None => return Err(warp::reject::custom(AppError::NotFound(
                format!("Short URL not found: {}", short_id)
            ))),
        },
        Err(_) => return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        ))),
    }; // Added semicolon here

    // Create a scan record
    let scan_id = nanoid!(10);
    let timestamp = Utc::now();
    let scan = Scan {
        scan_id: scan_id.clone(),
        short_url: short_id.to_string(),
        user_id: user_id.to_string(),
        device_id: device_id.to_string(),
        timestamp,
    };

    // Store scan in scans table
    if let Ok(mut scans) = db.scans.lock() {
        scans.insert(scan_id.clone(), scan);
    } else {
        return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        )));
    }

    // Return JSON with scan info and target URL
    let response = json!({
        "scan_id": scan_id,
        "url": url,
        "timestamp": timestamp.to_rfc3339()
    });
    
    Ok(warp::reply::json(&response))
}

async fn handle_visualize_db(db: Database) -> Result<impl warp::Reply, warp::Rejection> {
    let shortened_links = match db.shortened_links.lock() {
        Ok(links) => links.clone(),
        Err(_) => return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        ))),
    };
    
    let users = match db.users.lock() {
        Ok(users) => users.clone(),
        Err(_) => return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        ))),
    };
    
    let devices = match db.devices.lock() {
        Ok(devices) => devices.clone(),
        Err(_) => return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        ))),
    };
    
    let scans = match db.scans.lock() {
        Ok(scans) => scans.clone(),
        Err(_) => return Err(warp::reject::custom(AppError::DatabaseError(
            "Failed to acquire database lock".to_string()
        ))),
    };
    
    let db_state = json!({
        "shortened_links": shortened_links,
        "users": users,
        "devices": devices,
        "scans": scans,
    });
    
    Ok(warp::reply::json(&db_state))
}
