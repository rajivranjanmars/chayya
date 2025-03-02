# Chaya - URL Shortener with User Tracking

A simple URL shortener service built with Rust and Warp that tracks user interactions.

## Features

- Generate short URLs
- Track when users access shortened links
- Collect user information (name, email, mobile)
- Track device information using localStorage
- Visualize the database state

## Configuration

The application can be configured using environment variables:

- `SERVER_HOST`: Host address to bind to (default: "127.0.0.1")
- `SERVER_PORT`: Port to use (default: 3030)
- `BASE_URL`: Base URL for generated short links (default: "http://[SERVER_HOST]:[SERVER_PORT]")
- `TEMPLATES_PATH`: Path to the HTML templates directory (default: "/d:/New folder/chaya/templates")

## API Endpoints

### Create a shortened link
```
POST /shorten
Content-Type: application/json

{
  "url": "https://example.com"
}
```

### Access a shortened link
```
GET /{short_id}
```

### Submit user form
```
POST /user_form
Content-Type: application/x-www-form-urlencoded

short_id=abc123&device_id=dev456&name=John&email=john@example.com&mobile=1234567890
```

### Direct scan (for returning users)
```
POST /direct_scan
Content-Type: application/json

{
  "device_id": "dev456",
  "user_id": "usr789",
  "short_id": "abc123"
}
```

### Visualize database
```
GET /visualize_db
```

## Template Files

The application uses Handlebars templates stored in the `/templates` directory:

- `check_device.html`: Initial page that checks for device/user info in localStorage
- `user_form.html`: Form for collecting user information
- `new_device_form.html`: Form that includes script to store new device ID in localStorage
- `redirect.html`: Page that stores user info in localStorage and redirects to target URL

## Development

To run the application:

```
cargo run
```

To build for production:

```
cargo build --release
```
