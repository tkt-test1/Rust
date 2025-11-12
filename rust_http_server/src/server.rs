// src/server.rs
//
// 【処理概要】
// HTTPサーバーの核となる部分。TCPリスナーとワーカースレッドプールを実装。
// 接続を受け付け、並行処理でリクエストを処理する。
//
// 【主な機能】
// - TCPソケットのバインドとリッスン
// - スレッドプールによる並行リクエスト処理
// - 接続ごとのリクエスト/レスポンスハンドリング
// - エラーハンドリングとグレースフルシャットダウン
//
// 【実装内容】
// 1. TcpListenerで指定アドレスをリッスン
// 2. 接続受付ループ（accept）
// 3. 各接続をスレッドプールのワーカーに振り分け
// 4. ワーカースレッドでHTTPリクエストをパース、ルーター処理、レスポンス送信
// 5. スレッドプール管理（ワーカー生成、ジョブキューイング）

use crate::http::HttpRequest;
use crate::router::Router;
use std::io::{self, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

/// HTTPサーバー
pub struct Server {
    address: String,
    router: Arc<Router>,
}

impl Server {
    /// 新しいサーバーを作成
    pub fn new(address: &str, router: Router) -> Self {
        Server {
            address: address.to_string(),
            router: Arc::new(router),
        }
    }

    /// サーバーを起動（ブロッキング）
    /// 
    /// 処理フロー:
    /// 1. TCPリスナーをバインド
    /// 2. スレッドプールを初期化（ワーカー数: 4）
    /// 3. 接続受付ループに入る
    /// 4. 各接続をスレッドプールに送信
    pub fn run(self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.address)?;
        println!("✅ Listening on {}\n", self.address);

        // スレッドプール作成（4ワーカー）
        let pool = ThreadPool::new(4);

        // 接続受付ループ
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let router = Arc::clone(&self.router);
                    
                    // ジョブをスレッドプールに送信
                    pool.execute(move || {
                        if let Err(e) = handle_connection(stream, router) {
                            eprintln!("❌ Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("❌ Connection failed: {}", e);
                }
            }
        }

        Ok(())
    }
}

/// 接続を処理する関数
/// 
/// 処理手順:
/// 1. HTTPリクエストをパース
/// 2. ルーターで処理
/// 3. レスポンスを送信
fn handle_connection(mut stream: TcpStream, router: Arc<Router>) -> io::Result<()> {
    // リクエストのパース
    let request = HttpRequest::parse(&mut stream)?;

    // ルーターで処理
    let response = router.handle(request);

    // レスポンスを送信
    let response_bytes = response.to_bytes();
    stream.write_all(&response_bytes)?;
    stream.flush()?;

    Ok(())
}

// ===== スレッドプール実装 =====

/// ワーカースレッドプール
/// 
/// 仕組み:
/// - 固定数のワーカースレッドを事前に起動
/// - ジョブ（クロージャ）をキューに追加
/// - ワーカーはキューからジョブを取り出して実行
/// - チャネル（mpsc）を使ってスレッド間通信
struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// 新しいスレッドプールを作成
    /// 
    /// size: ワーカースレッド数
    fn new(size: usize) -> Self {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        println!("🧵 Thread pool initialized with {} workers", size);

        ThreadPool { workers, sender }
    }

    /// ジョブを実行キューに追加
    fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}

/// ワーカースレッド
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    /// 新しいワーカーを作成
    /// 
    /// 処理フロー:
    /// 1. スレッドを起動
    /// 2. レシーバーからジョブを受信待機
    /// 3. ジョブを受信したら実行
    /// 4. 2に戻る（ループ）
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::spawn(move || loop {
            // ジョブを受信（ブロッキング）
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    // デバッグ用ログ（本番では削除推奨）
                    // println!("Worker {} executing job", id);
                    job();
                }
                Err(_) => {
                    // チャネルがクローズされたら終了
                    println!("Worker {} shutting down", id);
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

impl Drop for ThreadPool {
    /// スレッドプールが破棄される際に全ワーカーを停止
    fn drop(&mut self) {
        println!("\n🛑 Shutting down thread pool...");

        // センダーをドロップしてチャネルをクローズ
        drop(&self.sender);

        // 全ワーカーの終了を待つ
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }

        println!("✅ All workers stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_pool_creation() {
        let pool = ThreadPool::new(4);
        assert_eq!(pool.workers.len(), 4);
    }

    #[test]
    fn test_thread_pool_execute() {
        let pool = ThreadPool::new(2);
        let counter = Arc::new(Mutex::new(0));
        
        for _ in 0..10 {
            let counter = Arc::clone(&counter);
            pool.execute(move || {
                let mut num = counter.lock().unwrap();
                *num += 1;
            });
        }

        // ジョブが完了するまで少し待つ
        thread::sleep(std::time::Duration::from_millis(100));
        
        let final_count = *counter.lock().unwrap();
        assert_eq!(final_count, 10);
    }
}
