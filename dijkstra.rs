// dijkstra.rs
//
// 概要:
//   - グラフ上の単一始点最短経路を求めるダイクストラ法の実装サンプル
//
// 実装機能:
//   - ノード間の辺を追加
//   - 始点から各ノードまでの最短距離を計算
//
// 使用例:
//   let mut g = Graph::new(5);
//   g.add_edge(0, 1, 10);
//   g.add_edge(0, 2, 3);
//   g.add_edge(2, 1, 1);
//   g.add_edge(1, 3, 2);
//   g.add_edge(2, 3, 8);
//   g.add_edge(3, 4, 7);
//   let dist = g.dijkstra(0);
//   println!("{:?}", dist); // [0, 4, 3, 6, 13]

use std::collections::BinaryHeap;
use std::cmp::Ordering;

#[derive(Copy, Clone, Eq, PartialEq)]
struct State {
    cost: usize,
    position: usize,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // Rust の BinaryHeap は最大ヒープなので逆順にする
        other.cost.cmp(&self.cost)
            .then_with(|| self.position.cmp(&other.position))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct Graph {
    adj: Vec<Vec<(usize, usize)>>, // (隣接ノード, 重み)
}

impl Graph {
    fn new(n: usize) -> Self {
        Graph { adj: vec![Vec::new(); n] }
    }

    fn add_edge(&mut self, u: usize, v: usize, w: usize) {
        self.adj[u].push((v, w));
    }

    fn dijkstra(&self, start: usize) -> Vec<usize> {
        let n = self.adj.len();
        let mut dist = vec![usize::MAX; n];
        let mut heap = BinaryHeap::new();

        dist[start] = 0;
        heap.push(State { cost: 0, position: start });

        while let Some(State { cost, position }) = heap.pop() {
            if cost > dist[position] {
                continue;
            }

            for &(next, weight) in &self.adj[position] {
                let next_cost = cost + weight;

                if next_cost < dist[next] {
                    dist[next] = next_cost;
                    heap.push(State { cost: next_cost, position: next });
                }
            }
        }

        dist
    }
}

fn main() {
    let mut g = Graph::new(5);
    g.add_edge(0, 1, 10);
    g.add_edge(0, 2, 3);
    g.add_edge(2, 1, 1);
    g.add_edge(1, 3, 2);
    g.add_edge(2, 3, 8);
    g.add_edge(3, 4, 7);

    let dist = g.dijkstra(0);
    println!("最短距離: {:?}", dist);
}
