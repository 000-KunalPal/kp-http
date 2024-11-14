use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

// HTTP Response status lines
const HTTP_OK: &str = "HTTP/1.1 200 OK\r\n";
const HTTP_NOT_FOUND: &str = "HTTP/1.1 404 Not Found\r\n";
const HTTP_METHOD_NOT_ALLOWED: &str = "HTTP/1.1 405 Method Not Allowed\r\n";

// HTTP Request struct to parse incoming requests
#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpRequest {
    fn parse(raw_request: &[u8]) -> Option<HttpRequest> {
        let request_str = String::from_utf8_lossy(raw_request);
        let lines: Vec<&str> = request_str.split("\r\n").collect();
        
        if lines.is_empty() {
            return None;
        }

        // Parse request line
        let request_line: Vec<&str> = lines[0].split_whitespace().collect();
        if request_line.len() < 2 {
            return None;
        }

        let method = request_line[0].to_string();
        let path = request_line[1].to_string();

        // Parse headers
        let mut headers = Vec::new();
        let mut i = 1;
        while i < lines.len() && !lines[i].is_empty() {
            if let Some((key, value)) = lines[i].split_once(": ") {
                headers.push((key.to_string(), value.to_string()));
            }
            i += 1;
        }

        // Parse body (if any)
        let body = if i < lines.len() - 1 {
            lines[i + 1].as_bytes().to_vec()
        } else {
            Vec::new()
        };

        Some(HttpRequest {
            method,
            path,
            headers,
            body,
        })
    }
}

// HTTP Response builder
struct HttpResponse {
    status_line: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpResponse {
    fn new(status_line: &str) -> Self {
        HttpResponse {
            status_line: status_line.to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_string(), value.to_string()));
        self
    }

    fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    fn build(self) -> Vec<u8> {
        let mut response = Vec::new();
        
        // Add status line
        response.extend_from_slice(self.status_line.as_bytes());
        
        // Add headers
        for (key, value) in self.headers {
            response.extend_from_slice(format!("{}: {}\r\n", key, value).as_bytes());
        }
        
        // Add Content-Length header
        response.extend_from_slice(format!("Content-Length: {}\r\n", self.body.len()).as_bytes());
        
        // Add empty line to separate headers from body
        response.extend_from_slice(b"\r\n");
        
        // Add body
        response.extend_from_slice(&self.body);
        
        response
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    
    match stream.read(&mut buffer) {
        Ok(size) => {
            if let Some(request) = HttpRequest::parse(&buffer[..size]) {
                // Check for authentication header
                let is_authenticated = request.headers.iter()
                    .any(|(key, value)| key == "Authorization" && value == "Bearer secret-token");

                let response = match (request.method.as_str(), request.path.as_str()) {
                    ("GET", "/") => {
                        HttpResponse::new(HTTP_OK)
                            .with_header("Content-Type", "text/html")
                            .with_body(b"<h1>Welcome to Rust HTTP Server!</h1>".to_vec())
                    },
                    ("POST", "/echo") => {
                        if !is_authenticated {
                            HttpResponse::new("HTTP/1.1 401 Unauthorized\r\n")
                                .with_header("Content-Type", "text/plain")
                                .with_body(b"Unauthorized".to_vec())
                        } else {
                            // Echo back the request body
                            HttpResponse::new(HTTP_OK)
                                .with_header("Content-Type", "application/json")
                                .with_body(request.body)
                        }
                    },
                    ("GET", "/health") => {
                        HttpResponse::new(HTTP_OK)
                            .with_header("Content-Type", "application/json")
                            .with_body(b"{\"status\": \"healthy\"}".to_vec())
                    },
                    ("GET", _) => {
                        HttpResponse::new(HTTP_NOT_FOUND)
                            .with_header("Content-Type", "text/plain")
                            .with_body(b"404 - Not Found".to_vec())
                    },
                    (_, _) => {
                        HttpResponse::new(HTTP_METHOD_NOT_ALLOWED)
                            .with_header("Content-Type", "text/plain")
                            .with_body(b"405 - Method Not Allowed".to_vec())
                    }
                };

                let response_bytes = response.build();
                if let Err(e) = stream.write_all(&response_bytes) {
                    eprintln!("Failed to send response: {}", e);
                }
            }
        },
        Err(e) => eprintln!("Failed to read from connection: {}", e),
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Server listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // Set timeouts for the connection
                stream.set_read_timeout(Some(Duration::from_secs(5)))?;
                stream.set_write_timeout(Some(Duration::from_secs(5)))?;
                
                // Handle each connection in a new thread
                thread::spawn(|| {
                    handle_client(stream);
                });
            }
            Err(e) => {
                eprintln!("Failed to establish connection: {}", e);
            }
        }
    }

    Ok(())
}
