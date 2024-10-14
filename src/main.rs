mod lib;
use lib::ThreadPool;
use std::fs;
use std::io::{prelude::*, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::str;

fn main() {
    let listener = TcpListener::bind("0.0.0.0:7878").unwrap();  // Changed address
    let pool = ThreadPool::new(4); // Create a thread pool with 4 workers

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                println!("Connection failed: {}", e);
                continue; // Skip this iteration on connection error
            }
        };

        // Assign the connection handling to a thread in the pool
        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down."); // Notify that the server is shutting down
}

fn handle_connection(mut stream: TcpStream) {
    let mut buf_reader = BufReader::new(&mut stream); // Create a buffered reader for the stream
    let mut request_line = String::new();

    // Read the request line from the client
    if buf_reader.read_line(&mut request_line).is_err() {
        println!("Failed to read request line from the client.");
        return; // Exit if the request line cannot be read
    }

    // Match the request line to determine the response
    let (status_line, filename) = match &request_line[..] {
        "GET / HTTP/1.1\r\n" => ("HTTP/1.1 200 OK", "website/hello.html"),
        "GET /style.css HTTP/1.1\r\n" => ("HTTP/1.1 200 OK", "website/style.css"),
        "GET /script.js HTTP/1.1\r\n" => ("HTTP/1.1 200 OK", "upload.js"),
        "GET /list-files HTTP/1.1\r\n" => {
            // Get the list of uploaded files and send as JSON
            let file_list_json = list_files();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                file_list_json.len(),
                file_list_json
            );
            stream.write_all(response.as_bytes()).unwrap();
            return; // Exit after sending the response
        },
        _ if request_line.starts_with("POST /upload HTTP/1.1") => {
            // Handle file uploads
            let content_length = get_content_length(&mut buf_reader);
            let mut body = vec![0; content_length];
            stream.read_exact(&mut body).expect("Failed to read the request body");
            handle_file_upload(body, &mut stream);
            return; // Exit after handling the file upload
        },
        _ if request_line.starts_with("DELETE /delete/") => {
            // Handle file deletion requests
            let file_to_delete = request_line
                .split_whitespace()
                .nth(1)
                .unwrap()
                .trim_start_matches("/delete/");

            match fs::remove_file(format!("uploads/{}", file_to_delete)) {
                Ok(_) => {
                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
                    stream.write_all(response.as_bytes()).unwrap();
                    return; // Exit after deleting the file
                },
                Err(_) => {
                    println!("File not found: {}", file_to_delete);
                },
            }

            ("HTTP/1.1 404 NOT FOUND", "website/404.html") // Prepare response for a missing file
        },
        _ if request_line.starts_with("GET /download/") => {
            // Handle file download requests
            let file_to_download = request_line
                .split_whitespace()
                .nth(1)
                .unwrap()
                .trim_start_matches("/download/");

            match fs::read(format!("uploads/{}", file_to_download)) {
                Ok(contents) => {
                    let length = contents.len();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nContent-Disposition: attachment; filename=\"{file_to_download}\"\r\n\r\n"
                    );
                    stream.write_all(response.as_bytes()).unwrap();
                    stream.write_all(&contents).unwrap();
                    return; // Exit after sending the file
                },
                Err(_) => {
                    println!("File not found: {}", file_to_download);
                },
            }

            ("HTTP/1.1 404 NOT FOUND", "website/404.html") // Prepare response for a missing file
        },
        _ => ("HTTP/1.1 404 NOT FOUND", "website/404.html"), // Default 404 response
    };

    // Read the contents of the file to send in the response
    let contents = match fs::read_to_string(filename) {
        Ok(contents) => contents,
        Err(e) => {
            println!("Failed to read file {}: {}", filename, e);
            return; // Exit if file reading fails
        }
    };
    let length = contents.len();

    // Prepare and send the HTTP response
    let response = format!(
        "{status_line}\r\nContent-Length: {length}\r\nCache-Control: no-cache\r\nExpires: Thu, 01 Jan 1970 00:00:00 GMT\r\n\r\n{contents}"
    );

    if let Err(e) = stream.write_all(response.as_bytes()) {
        println!("Failed to send response to the client: {}", e);
        return; // Exit if response sending fails
    }
    stream.flush().unwrap(); // Ensure all data is sent
}

fn list_files() -> String {
    // List files in the uploads directory and return as a JSON string
    let entries = fs::read_dir("uploads").unwrap();
    let mut files = Vec::new();

    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(name) = entry.file_name().into_string() {
                files.push(name);
            }
        }
    }

    let json_files = serde_json::to_string(&files).unwrap(); // Convert file list to JSON format
    json_files
}

fn get_content_length(reader: &mut BufReader<&mut TcpStream>) -> usize {
    // Retrieve the Content-Length header from the request
    let mut content_length = 0;
    for line in reader.lines() {
        let line = line.unwrap();
        if line.starts_with("Content-Length:") {
            content_length = line.split(':').nth(1).unwrap().trim().parse().unwrap();
            break; // Exit after finding the Content-Length
        }
    }
    content_length
}

fn handle_file_upload(body: Vec<u8>, stream: &mut TcpStream) {
    // Handle file uploads by extracting file data from the request body
    let boundary = b"\r\n\r\n";
    let body_len = body.len();
    if let Some(pos) = body.windows(boundary.len()).position(|window| window == boundary) {
        let file_data = &body[pos + boundary.len()..body_len - 4]; // Extract file data
        
        // Extract the filename from the headers
        let headers = str::from_utf8(&body[..pos]).unwrap();
        let filename = headers.lines()
            .find(|line| line.contains("filename="))
            .and_then(|line| line.split("filename=").nth(1))
            .map(|name| name.trim_matches('"'))
            .unwrap_or("uploads/uploaded_file"); // Default filename if none provided

        fs::write(format!("uploads/{}", filename), file_data).expect("Unable to write file"); // Save the uploaded file

        // Send response indicating success
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).expect("Failed to send response");
    } else {
        // Send a bad request response if the upload format is incorrect
        let response = "HTTP/1.1 400 BAD REQUEST\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).expect("Failed to send response");
    }
}
