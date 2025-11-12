// src/router.rs
//
// 【処理概要】
// ルーティングとミドルウェアシステムを実装。
// リクエストを適切なハンドラに振り分け、ミドルウェアチェーンを実行する。
//
// 【主な機能】
// - URLパスとハンドラ関数のマッピング
// - パスパラメータの抽出（例: /users/:id）
// - ミドルウェアチェーンの実行（前処理・後処理）
// - HTTPメソッド別のルーティング（GET, POST等）
//
// 【実装内容】
// 1. ルート登録（静的パス、動的パラメータ対応）
// 2. リクエストマッチング（正規表現ベース）
// 3. ミドルウェアの順次実行（Continue/Stop制御）
// 4. ハンドラ実行とレスポンス生成

use crate::http::{HttpRequest, HttpResponse};
use std::collections::HashMap;

/// リクエスト情報（ハンドラに渡される）
#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub params: HashMap<String, String>, // パスパラメータ（例: {:id => "123"}）
}

/// レスポンス情報（ハンドラが返す）
pub type Response = HttpResponse;

/// ハンドラ関数の型
/// リクエストを受け取り、レスポンスを返す
pub type Handler = Box<dyn Fn(&Request) -> Response + Send + Sync>;

/// ミドルウェア関数の型
/// リクエストとレスポンスを受け取り、処理を続けるか停止するかを返す
pub type Middleware = fn(&Request, &mut Response) -> MiddlewareResult;

/// ミドルウェアの実行結果
#[derive(Debug, PartialEq)]
pub enum MiddlewareResult {
    Continue, // 次のミドルウェア/ハンドラに進む
    Stop,     // ここで処理を停止（レスポンスを即座に返す）
}

/// ルート情報
struct Route {
    method: String,
    pattern: String,        // 元のパターン（例: "/users/:id"）
    param_names: Vec<String>, // パラメータ名のリスト
    handler: Handler,
}

/// ルーター本体
pub struct Router {
    routes: Vec<Route>,
    middlewares: Vec<Middleware>,
    not_found_handler: Option<Handler>,
}

impl Router {
    /// 新しいルーターを作成
    pub fn new() -> Self {
        Router {
            routes: Vec::new(),
            middlewares: Vec::new(),
            not_found_handler: None,
        }
    }

    /// GETルートを登録
    pub fn get(&mut self, pattern: &str, handler: Handler) {
        self.add_route("GET", pattern, handler);
    }

    /// POSTルートを登録
    pub fn post(&mut self, pattern: &str, handler: Handler) {
        self.add_route("POST", pattern, handler);
    }

    /// 任意のメソッドでルートを登録
    fn add_route(&mut self, method: &str, pattern: &str, handler: Handler) {
        let param_names = extract_param_names(pattern);
        
        self.routes.push(Route {
            method: method.to_string(),
            pattern: pattern.to_string(),
            param_names,
            handler,
        });
    }

    /// ミドルウェアを追加（登録順に実行される）
    pub fn use_middleware(&mut self, middleware: Middleware) {
        self.middlewares.push(middleware);
    }

    /// 404ハンドラーを設定
    pub fn not_found(&mut self, handler: Handler) {
        self.not_found_handler = Some(handler);
    }

    /// リクエストを処理してレスポンスを返す
    /// 
    /// 処理フロー:
    /// 1. HttpRequestをRequestに変換
    /// 2. ミドルウェアを順次実行
    /// 3. ルートをマッチング
    /// 4. マッチしたハンドラを実行
    /// 5. レスポンスを返す
    pub fn handle(&self, http_req: HttpRequest) -> Response {
        // Requestに変換
        let mut request = Request {
            method: http_req.method.clone(),
            path: http_req.path.clone(),
            headers: http_req.headers.clone(),
            body: http_req.body.clone(),
            params: HashMap::new(),
        };

        // デフォルトレスポンス
        let mut response = Response::ok(r#"{"status": "ok"}"#);

        // ミドルウェア実行
        for middleware in &self.middlewares {
            match middleware(&request, &mut response) {
                MiddlewareResult::Continue => continue,
                MiddlewareResult::Stop => return response,
            }
        }

        // ルートマッチング
        for route in &self.routes {
            // メソッドチェック
            if route.method != request.method {
                continue;
            }

            // パスマッチング
            if let Some(params) = match_path(&route.pattern, &route.param_names, &request.path) {
                request.params = params;
                return (route.handler)(&request);
            }
        }

        // 404ハンドラー
        if let Some(handler) = &self.not_found_handler {
            handler(&request)
        } else {
            Response::not_found(r#"{"error": "Not Found"}"#)
        }
    }
}

/// パターンからパラメータ名を抽出
/// 例: "/users/:id/posts/:post_id" -> ["id", "post_id"]
fn extract_param_names(pattern: &str) -> Vec<String> {
    pattern
        .split('/')
        .filter(|seg| seg.starts_with(':'))
        .map(|seg| seg[1..].to_string())
        .collect()
}

/// パスがパターンにマッチするかチェックし、パラメータを抽出
/// 
/// 例:
/// pattern: "/users/:id/posts/:post_id"
/// path: "/users/123/posts/456"
/// -> Some({"id": "123", "post_id": "456"})
fn match_path(
    pattern: &str,
    param_names: &[String],
    path: &str,
) -> Option<HashMap<String, String>> {
    let pattern_segments: Vec<&str> = pattern.split('/').collect();
    let path_segments: Vec<&str> = path.split('/').collect();

    // セグメント数が一致しない場合はマッチしない
    if pattern_segments.len() != path_segments.len() {
        return None;
    }

    let mut params = HashMap::new();
    let mut param_index = 0;

    for (pattern_seg, path_seg) in pattern_segments.iter().zip(path_segments.iter()) {
        if pattern_seg.starts_with(':') {
            // 動的セグメント（パラメータ）
            if param_index < param_names.len() {
                params.insert(param_names[param_index].clone(), path_seg.to_string());
                param_index += 1;
            }
        } else if pattern_seg != path_seg {
            // 静的セグメントが一致しない
            return None;
        }
    }

    Some(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_param_names() {
        let names = extract_param_names("/users/:id/posts/:post_id");
        assert_eq!(names, vec!["id", "post_id"]);
    }

    #[test]
    fn test_match_path() {
        let pattern = "/users/:id/posts/:post_id";
        let param_names = extract_param_names(pattern);
        let path = "/users/123/posts/456";
        
        let params = match_path(pattern, &param_names, path).unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));
        assert_eq!(params.get("post_id"), Some(&"456".to_string()));
    }

    #[test]
    fn test_match_path_no_match() {
        let pattern = "/users/:id";
        let param_names = extract_param_names(pattern);
        let path = "/posts/123";
        
        assert!(match_path(pattern, &param_names, path).is_none());
    }
}
