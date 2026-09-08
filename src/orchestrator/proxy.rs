mod connections;
pub(crate) use connections::{RouteConnections, RouteConnector, RouteStream};
use std::sync::Arc;
use std::{collections::HashMap, net::Ipv4Addr, time::SystemTime};

use crate::orchestrator::SandboxState;
use crate::types::SandboxId;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct ProxyTarget {
    pub ip: Ipv4Addr,
    /// The funded activation which published this route, independent of lease rollover.
    pub activation_id: Option<Uuid>,
    pub(crate) connections: Arc<RouteConnections>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProxyLookupResult {
    Ready(ProxyTarget),
    NotFound,
    Paused { auto_resume: bool },
    Unavailable(SandboxState),
    RouteMissing,
}

#[derive(Clone, Debug)]
pub(crate) struct ProxyRoute {
    target: ProxyTarget,
    version: u64,
    updated_at: SystemTime,
}

#[derive(Debug, Default)]
pub(crate) struct ProxyRouteTable {
    routes: HashMap<SandboxId, ProxyRoute>,
}

impl ProxyTarget {
    pub fn new(host_interaction_ip: Ipv4Addr, activation_id: Option<Uuid>) -> Self {
        Self {
            ip: host_interaction_ip,
            activation_id,
            connections: Arc::new(RouteConnections::default()),
        }
    }
}

// Address/activation equality is useful for observations; connection authority
// remains specific to a route publication and cannot be inferred from equality.
impl PartialEq for ProxyTarget {
    fn eq(&self, other: &Self) -> bool {
        self.ip == other.ip && self.activation_id == other.activation_id
    }
}
impl Eq for ProxyTarget {}

impl ProxyRoute {
    pub fn new(mut target: ProxyTarget, version: u64) -> Self {
        target.connections = Arc::new(RouteConnections::default());
        Self {
            target,
            version,
            updated_at: SystemTime::now(),
        }
    }

    pub(crate) async fn wait_retired(&self) {
        loop {
            match self.target.connections.wait_retired().await {
                Ok(()) => return,
                Err(error) => {
                    tracing::warn!(version = self.version, %error, "route socket retirement remains unacknowledged");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    self.target.connections.begin_retire();
                }
            }
        }
    }

    pub fn target(&self) -> &ProxyTarget {
        &self.target
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn updated_at(&self) -> SystemTime {
        self.updated_at
    }
}

impl ProxyRouteTable {
    pub fn upsert(
        &mut self,
        sandbox_id: SandboxId,
        target: ProxyTarget,
        version: u64,
    ) -> ProxyRoute {
        let mut route = ProxyRoute::new(target, version);
        if let Some(old) = self.routes.remove(&sandbox_id) {
            route.target.connections = Arc::new(RouteConnections::after(old.target.connections));
        }
        self.routes.insert(sandbox_id, route.clone());
        route
    }

    pub fn remove(&mut self, sandbox_id: &SandboxId) -> Option<ProxyRoute> {
        let route = self.routes.remove(sandbox_id)?;
        route.target.connections.begin_retire();
        Some(route)
    }

    #[cfg(test)]
    pub fn proxy_target(&self, sandbox_id: &SandboxId) -> Option<ProxyTarget> {
        self.routes
            .get(sandbox_id)
            .map(|route| route.target().clone())
    }

    pub fn route(&self, sandbox_id: &SandboxId) -> Option<&ProxyRoute> {
        self.routes.get(sandbox_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_table_only_exposes_inserted_routes() {
        let sandbox_id = SandboxId::new();
        let target = ProxyTarget::new(Ipv4Addr::LOCALHOST, None);
        let mut table = ProxyRouteTable::default();

        table.upsert(sandbox_id, target.clone(), 1);
        assert_eq!(table.proxy_target(&sandbox_id), Some(target.clone()));

        table.upsert(sandbox_id, target.clone(), 2);
        assert_eq!(table.proxy_target(&sandbox_id), Some(target));
        assert_eq!(table.routes.get(&sandbox_id).unwrap().version(), 2);
    }

    #[test]
    fn proxy_table_remove_drops_route() {
        let sandbox_id = SandboxId::new();
        let mut table = ProxyRouteTable::default();

        table.upsert(sandbox_id, ProxyTarget::new(Ipv4Addr::LOCALHOST, None), 3);
        let _removed = table.remove(&sandbox_id).unwrap();

        assert!(!table.routes.contains_key(&sandbox_id));
        assert!(table.proxy_target(&sandbox_id).is_none());
    }
    #[tokio::test]
    async fn restored_route_does_not_reopen_a_retired_resolution() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let id = SandboxId::new();
        let mut table = ProxyRouteTable::default();
        table.upsert(
            id,
            ProxyTarget::new(Ipv4Addr::LOCALHOST, Some(Uuid::new_v4())),
            1,
        );
        let old = table.proxy_target(&id).unwrap();
        let retired = table.remove(&id).unwrap();
        retired.wait_retired().await;
        table.upsert(id, retired.target().clone(), 2);
        assert!(old.connections.connect(address).await.is_err());
        let current = table.proxy_target(&id).unwrap();
        let connection = current.connections.connect(address).await.unwrap();
        drop(connection);
        table.remove(&id).unwrap().wait_retired().await;
    }
}
