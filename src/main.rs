mod lib;
use lib::ThreadPool;
use std::{
    fs,
    io::{prelude::*, BufReader, Write},
    net::{TcpListener, TcpStream},
};

fn main() {
    let listener = TcpListener::bind("0.0.0.0:7878").unwrap(); // Привязка к IP Zerotier
    let pool = ThreadPool::new(4); // Пул потоков с 4 рабочими

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                println!("Connection failed: {}", e);
                continue;
            }
        };

        let peer_addr = stream.peer_addr().unwrap();
        println!("New connection from: {}", peer_addr);

        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down.");
}

fn handle_connection(mut stream: TcpStream) {
    let mut buf_reader = BufReader::new(&mut stream);
    let mut request_line = String::new();

    // Чтение строки запроса
    if buf_reader.read_line(&mut request_line).is_err() {
        println!("Failed to read request line from the client.");
        return;
    }

    let (status_line, filename) = match &request_line[..] {
        "GET / HTTP/1.1\r\n" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /style.css HTTP/1.1\r\n" => ("HTTP/1.1 200 OK", "style.css"),
        "GET /script.js HTTP/1.1\r\n" => ("HTTP/1.1 200 OK", "upload.js"), // Добавляем обработку для JavaScript
        "GET /list-files HTTP/1.1\r\n" => {
            let file_list_json = list_files();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                file_list_json.len(),
                file_list_json
            );
            stream.write_all(response.as_bytes()).unwrap();
            return; // Завершение функции после успешной отправки ответа
        },
        _ if request_line.starts_with("POST /upload HTTP/1.1") => {
            let content_length = get_content_length(&mut buf_reader);
            let mut body = vec![0; content_length];
            stream.read_exact(&mut body).expect("Failed to read the request body");
            handle_file_upload(body, &mut stream); // Передаем stream в handle_file_upload
            return; // Завершение функции после успешной обработки
        },
        _ if request_line.starts_with("GET /download/") => {
            let file_to_download = request_line.split_whitespace().nth(1).unwrap().trim_start_matches("/download/");
            match fs::read_to_string(file_to_download) {
                Ok(contents) => {
                    let length = contents.len();
                    let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\n\r\n{contents}");
                    stream.write_all(response.as_bytes()).unwrap();
                    return; // Завершение функции после успешной отправки ответа
                },
                Err(_) => {
                    println!("File not found: {}", file_to_download);
                },
            }
            ("HTTP/1.1 404 NOT FOUND", "404.html") // Если файл не найден, обрабатываем это ниже
        },
        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };

    // Чтение содержимого файла
    let contents = match fs::read_to_string(filename) {
        Ok(contents) => contents,
        Err(e) => {
            println!("Failed to read file {}: {}", filename, e);
            return;
        }
    };
    let length = contents.len();

    // Формирование HTTP-ответа с правильными заголовками
    let response = format!(
        "{status_line}\r\nContent-Length: {length}\r\nCache-Control: no-cache\r\nExpires: Thu, 01 Jan 1970 00:00:00 GMT\r\n\r\n{contents}"
    );

    // Отправка ответа клиенту
    if let Err(e) = stream.write_all(response.as_bytes()) {
        println!("Failed to send response to the client: {}", e);
        return;
    }
    stream.flush().unwrap();

    // Логирование завершения обработки запроса
    println!("Processed request: {} -> {}", request_line, status_line);
}

fn list_files() -> String {
    let entries = fs::read_dir(".").unwrap(); // Чтение текущего каталога
    let mut files = Vec::new();

    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(name) = entry.file_name().into_string() {
                files.push(name);
            }
        }
    }

    // Преобразование в JSON
    let json_files = serde_json::to_string(&files).unwrap();
    println!("Files found: {:?}", files); // Логирование найденных файлов
    json_files // Возвращаем JSON-строку
}

fn get_content_length(reader: &mut BufReader<&mut TcpStream>) -> usize {
    let mut content_length = 0;
    for line in reader.lines() {
        let line = line.unwrap();
        if line.starts_with("Content-Length:") {
            content_length = line.split(':').nth(1).unwrap().trim().parse().unwrap();
            break;
        }
    }
    content_length
}

fn handle_file_upload(body: Vec<u8>, stream: &mut TcpStream) { // Изменяем сигнатуру функции
    // Извлечение данных файла из тела запроса и сохранение его
    let boundary = b"\r\n\r\n";
    let body_len = body.len();
    if let Some(pos) = body.windows(boundary.len()).position(|window| window == boundary) {
        let file_data = &body[pos + boundary.len()..body_len - 4]; // Пропустить границу и конечные \r\n
        let filename = "uploaded_file"; // Укажите имя файла, с которым хотите сохранить

        fs::write(filename, file_data).expect("Unable to write file");

        // Логирование размера загружаемого файла
        println!("Uploaded file size: {}", file_data.len());

        // Отправка успешного ответа клиенту
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).expect("Failed to send response");
    } else {
        println!("Failed to find file data in upload request.");
        let response = "HTTP/1.1 400 BAD REQUEST\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(response.as_bytes()).expect("Failed to send response");
    }
}
