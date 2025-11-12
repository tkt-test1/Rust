// src/http.rs
//
// 【処理概要】
// HTTPプロトコルの低レベル処理を実装。
// リクエストのパースとレスポンスの生成を行う。
//
// 【主な機能】
// - HTTPリクエストの解析（メソッド、パス、ヘッダー、ボディ）
// - HTTPレスポンスの生成（ステータスコード、ヘッダー、ボディ）
// - 生のバイト列とHTTP構造体の相互変換
//
// 【実装内容】
// 1. リクエスト行のパース（例: "GET /path HTTP/1.1"）
// 2. ヘッダーのパース（例: "Content-Type: application/json"）
// 3. ボディの読み取り（Content-Lengthに基づく）
// 4. レスポンスのバイト列生成（ステータス行 + ヘッダー + ボディ）

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read};
use std::net::TcpStream;

/// HTTPリクエストを表す構造体
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    /// TcpStreamからHTTPリクエストをパースする
    /// 
    /// パース手順:
    /// 1. リクエスト行を読み取り（例: GET /path HTTP/1.1）
    /// 2. ヘッダー行を全て読み取り（空行まで）
    /// 3. Content-Lengthがあればボディを読み取り
    pub fn parse(stream: &mut TcpStream) -> io::Result<Self> {
        let mut reader = BufReader::new(stream);
        let mut lines = Vec::new();

        // ヘッダー部分を読み取り（空行まで）
        loop {
            let mut line = String::new();
            let bytes_read = reader.read_line(&mut line)?;
            
            if bytes_read == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Connection closed before receiving complete request",
                ));
            }

            // 改行を削除
            let line = line.trim_end().to_string();
            
            // 空行はヘッダーの終わりを示す
            if line.is_empty() {
                break;
            }
            
            lines.push(line);
        }

        if lines.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Empty HTTP request",
            ));
        }

        // リクエスト行をパース（例: "GET /path HTTP/1.1"）
        let request_line = &lines[0];
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        
        if parts.len() != 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid request line: {}", request_line),
            ));
        }

        let method = parts[0].to_string();
        let path = parts[1].to_string();
        let version = parts[2].to_string();

        // ヘッダーをパース（例: "Content-Type: application/json"）
        let mut headers = HashMap::new();
        for line in &lines[1..] {
            if let Some((key, value)) = line.split_once(':') {
                headers.insert(
                    key.trim().to_lowercase(),
                    value.trim().to_string(),
                );
            }
        }

        // ボディの読み取り（Content-Lengthがある場合）
        let mut body = Vec::new();
        if let Some(length_str) = headers.get("content-length") {
            if let Ok(length) = length_str.parse::<usize>() {
                body = vec![0; length];
                reader.read_exact(&mut body)?;
            }
        }

        Ok(HttpRequest {
            method,
            path,
            version,
            headers,
            body,
        })
    }
}

/// HTTPレスポンスを表す構造体
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// 新しいレスポンスを作成
    pub fn new(status_code: u16, status_text: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Server".to_string(), "RustHTTP/1.0".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        
        HttpResponse {
            status_code,
            status_text: status_text.to_string(),
            headers,
            body: Vec::new(),
        }
    }

    /// ボディを設定
    pub fn with_body(mut self, body: &str) -> Self {
        self.body = body.as_bytes().to_vec();
        self.headers.insert(
            "Content-Length".to_string(),
            self.body.len().to_string(),
        );
        self
    }

    /// HTTPレスポンスをバイト列に変換
    /// 
    /// フォーマット:
    /// HTTP/1.1 200 OK\r\n
    /// Header1: Value1\r\n
    /// Header2: Value2\r\n
    /// \r\n
    /// body content
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut response = Vec::new();

        // ステータス行
        let status_line = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status_code, self.status_text
        );
        response.extend_from_slice(status_line.as_bytes());

        // ヘッダー
        for (key, value) in &self.headers {
            let header_line = format!("{}: {}\r\n", key, value);
            response.extend_from_slice(header_line.as_bytes());
        }

        // 空行（ヘッダーとボディの区切り）
        response.extend_from_slice(b"\r\n");

        // ボディ
        response.extend_from_slice(&self.body);

        response
    }
}

// ===== 便利メソッド =====

impl HttpResponse {
    /// 200 OK レスポンス
    pub fn ok(body: &str) -> Self {
        Self::new(200, "OK").with_body(body)
    }

    /// 201 Created レスポンス
    pub fn created(body: &str) -> Self {
        Self::new(201, "Created").with_body(body)
    }

    /// 400 Bad Request レスポンス
    pub fn bad_request(body: &str) -> Self {
        Self::new(400, "Bad Request").with_body(body)
    }

    /// 401 Unauthorized レスポンス
    pub fn unauthorized(body: &str) -> Self {
        Self::new(401, "Unauthorized").with_body(body)
    }

    /// 404 Not Found レスポンス
    pub fn not_found(body: &str) -> Self {
        Self::new(404, "Not Found").with_body(body)
    }

    /// 500 Internal Server Error レスポンス
    pub fn internal_error(body: &str) -> Self {
        Self::new(500, "Internal Server Error").with_body(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_to_bytes() {
        let response = HttpResponse::ok(r#"{"status": "success"}"#);
        let bytes = response.to_bytes();
        let text = String::from_utf8_lossy(&bytes);
        
        assert!(text.contains("HTTP/1.1 200 OK"));
        assert!(text.contains("Content-Type: application/json"));
        assert!(text.contains(r#"{"status": "success"}"#));
    }
}
