use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

use crate::{
    area::{self, lsdb::LSDB_MAP},
    lsa::router::{LS_ID_POINT_TO_POINT, LS_ID_STUB, LS_ID_TRANSIT, LS_ID_VIRTUAL_LINK},
    neighbor,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeType {
    Router,
    Transit,
    Stub,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Node {
    pub node_type: NodeType,
    pub id: u32,
}

#[derive(Clone, Copy)]
pub struct Edge {
    pub cost: u16,
}

pub struct Graph {
    pub nodes: HashSet<Node>,
    pub edges: HashMap<Node, HashMap<Node, Edge>>,
}

lazy_static::lazy_static! {
    pub static ref GRAPH : Arc<RwLock<Graph>> = Arc::new(RwLock::new(Graph::new()));
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: HashMap::new(),
        }
    }
    pub fn add_node(&mut self, node: Node) {
        if !self.nodes.contains(&node) {
            self.nodes.insert(node);
        }
    }
    pub fn add_edge(&mut self, node1: Node, node2: Node, edge: Edge) {
        self.edges
            .entry(node1)
            .or_insert(HashMap::new())
            .insert(node2, edge);
    }
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
    }
}

/// use the area's LSDB to build the graph
/// now we currently only think about the router node
pub async fn build_graph() {
    let mut graph = GRAPH.write().await;
    graph.clear();
    let lsdb_map = LSDB_MAP.read().await;
    for (_, lsdb) in lsdb_map.iter() {
        let locked_lsdb = lsdb.read().await;
        let locked_router_lsa = locked_lsdb.router_lsa.read().await;
        for (identifier, lsa) in &*locked_router_lsa {
            let locked_lsa = lsa.read().await;
            match locked_lsa.header.lsa_type {
                crate::lsa::router::ROUTER_LSA_TYPE => {
                    let node = Node {
                        node_type: NodeType::Router,
                        id: locked_lsa.header.link_state_id,
                    };
                    graph.add_node(node);
                    for link in &locked_lsa.link_states {
                        match link.ls_type {
                            LS_ID_STUB => {}
                            LS_ID_POINT_TO_POINT => {}
                            LS_ID_TRANSIT => {}
                            LS_ID_VIRTUAL_LINK => {
                                crate::util::error(&format!(
                                    "unimplemented link type: {}",
                                    link.ls_type
                                ));
                            }
                            _ => {
                                crate::util::error(&format!(
                                    "unimplemented link type: {}",
                                    link.ls_type
                                ));
                            }
                        }
                    }
                }
                _ => {
                    crate::util::error(&format!(
                        "unimplemented lsa type: {}",
                        locked_lsa.header.lsa_type
                    ));
                }
            }
        }
    }
}
