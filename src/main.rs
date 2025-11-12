// src/main.rs
//
// 【処理概要】
// エントリーポイント。HTTPサーバーの初期化とルーティング設定を行う。
// 
// 【主な機能】
// - サーバーインスタンスの生成
// - エンドポイント（ルート）の登録
// - ミドルウェア（ロギング、認証風処理）の設定
// - サーバーの起動とリクエスト受付
//
// 【実装内容】
// 1. ルーターを作成し、各URLパスにハンドラ関数を紐付け
// 2. グローバルミドルウェア（全リクエストで実行）を追加
// 3. サーバーを指定ポートでリッスン開始
// 4. 各リクエストをワーカースレッドプールで並行処理

mod server;
mod router;
mod http;

use server::Server;
use router::{Router, Request, Response, Middleware, MiddlewareResult};
use std::collections::HashMap;

fn main() {
    println!("=== Rust HTTP Server (標準ライブラリのみ実装) ===\n");

    // ルーターの初期化
    let mut router = Router::new();

    // ===== ミドルウェアの登録 =====
    
    // ロギングミドルウェア: 全リクエストのログを出力
    router.use_middleware(logging_middleware);
    
    // 認証風ミドルウェア: Authorizationヘッダーのチェック（デモ）
    router.use_middleware(auth_middleware);

    // ===== ルート（エンドポイント）の登録 =====
    
    // GET / - ルートパス
    router.get("/", Box::new(|_req| {
        Response::ok(r#"{"message": "Welcome to Rust HTTP Server!", "version": "1.0"}"#)
    }));

    // GET /api/users - ユーザー一覧取得
    router.get("/api/users", Box::new(|_req| {
        let users = r#"{"users": [
            {"id": 1, "name": "Alice", "role": "admin"},
            {"id": 2, "name": "Bob", "role": "user"},
            {"id": 3, "name": "Charlie", "role": "user"}
        ]}"#;
        Response::ok(users)
    }));

    // GET /api/users/:id - 特定ユーザー取得（パスパラメータ）
    router.get("/api/users/:id", Box::new(|req| {
        if let Some(id) = req.params.get("id") {
            let user = format!(
                r#"{{"id": {}, "name": "User {}", "email": "user{}@example.com"}}"#,
                id, id, id
            );
            Response::ok(&user)
        } else {
            Response::bad_request(r#"{"error": "User ID is required"}"#)
        }
    }));

    // POST /api/users - ユーザー作成（ボディ解析デモ）
    router.post("/api/users", Box::new(|req| {
        let body = String::from_utf8_lossy(&req.body);
        let response = format!(
            r#"{{"message": "User created", "received_data": {}}}"#,
            body
        );
        Response::created(&response)
    }));

    // GET /api/stats - サーバー統計情報
    router.get("/api/stats", Box::new(|_req| {
        let stats = r#"{"uptime": "unknown", "requests": "many", "threads": 4}"#;
        Response::ok(stats)
    }));

    // 404ハンドラー
    router.not_found(Box::new(|req| {
        let error = format!(
            r#"{{"error": "Not Found", "path": "{}"}}"#,
            req.path
        );
        Response::not_found(&error)
    }));

    // ===== サーバー起動 =====
    let addr = "127.0.0.1:8080";
    println!("🚀 Server starting on http://{}", addr);
    println!("📡 Available endpoints:");
    println!("   GET  /");
    println!("   GET  /api/users");
    println!("   GET  /api/users/:id");
    println!("   POST /api/users");
    println!("   GET  /api/stats");
    println!("\n💡 Try: curl http://localhost:8080/api/users\n");

    let server = Server::new(addr, router);
    
    // サーバー起動（ブロッキング）
    if let Err(e) = server.run() {
        eprintln!("❌ Server error: {}", e);
    }
}

// ===== ミドルウェア実装 =====

/// ロギングミドルウェア
/// 全リクエストのメソッドとパスをコンソールに出力
fn logging_middleware(req: &Request, _res: &mut Response) -> MiddlewareResult {
    println!("📝 {} {}", req.method, req.path);
    MiddlewareResult::Continue
}

/// 認証風ミドルウェア
/// Authorizationヘッダーをチェック（デモ用、簡易実装）
/// ヘッダーがない場合は警告を出すが、処理は続行
fn auth_middleware(req: &Request, res: &mut Response) -> MiddlewareResult {
    // /api/ で始まるパスのみ認証チェック
    if req.path.starts_with("/api/") {
        if let Some(auth) = req.headers.get("authorization") {
            println!("🔐 Auth header found: {}", auth);
        } else {
            println!("⚠️  No authorization header (continuing anyway for demo)");
            // 本番環境では、ここで401を返すべき
            // *res = Response::unauthorized(r#"{"error": "Unauthorized"}"#);
            // return MiddlewareResult::Stop;
        }
    }
    MiddlewareResult::Continue
}
