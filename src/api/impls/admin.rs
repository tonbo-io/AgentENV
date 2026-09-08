use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use crate::observability::{DiskMetric, MachineInfo, NodeMetricsSnapshot, NodeSnapshot};
use agentenv_http_server::{apis::admin::*, models};

use super::ApiImpl;

impl From<MachineInfo> for models::MachineInfo {
    fn from(machine_info: MachineInfo) -> Self {
        models::MachineInfo::new(
            machine_info.cpu_family,
            machine_info.cpu_model,
            machine_info.cpu_model_name,
            machine_info.cpu_architecture,
        )
    }
}

impl From<DiskMetric> for models::DiskMetrics {
    fn from(disk: DiskMetric) -> Self {
        models::DiskMetrics::new(
            disk.mount_point,
            disk.device,
            disk.filesystem_type,
            disk.used_bytes,
            disk.total_bytes,
        )
    }
}

impl From<NodeMetricsSnapshot> for models::NodeMetrics {
    fn from(metrics: NodeMetricsSnapshot) -> Self {
        models::NodeMetrics::new(
            metrics.allocated_cpu,
            metrics.cpu_percent,
            metrics.cpu_count,
            metrics.allocated_memory_bytes,
            metrics.memory_used_bytes,
            metrics.memory_total_bytes,
            metrics
                .disks
                .into_iter()
                .map(models::DiskMetrics::from)
                .collect(),
            metrics.paused_allocated_cpu,
            metrics.paused_allocated_memory_bytes,
        )
    }
}

impl From<NodeSnapshot> for models::Node {
    fn from(node: NodeSnapshot) -> Self {
        models::Node::new(
            node.version,
            node.commit,
            node.node_id,
            node.service_instance_id,
            node.cluster_id.to_string(),
            node.machine_info.into(),
            if node.admission_closed {
                models::NodeStatus::NodeStatusDraining
            } else {
                models::NodeStatus::NodeStatusReady
            },
            node.sandbox_count,
            node.metrics.into(),
            node.create_successes,
            node.create_fails,
            node.sandbox_starting_count,
            node.paused_sandbox_count,
        )
    }
}

#[async_trait]
impl Admin<()> for ApiImpl {
    type Claims = super::Claims;

    async fn nodes_get(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
        _claims: &Self::Claims,
        query_params: &models::NodesGetQueryParams,
    ) -> Result<NodesGetResponse, ()> {
        let Some(observability) = self.observability() else {
            // When observability is disabled, the collection endpoint exposes
            // no nodes rather than returning a partial or synthetic record.
            return Ok(NodesGetResponse::Status200_SuccessfullyReturnedAllNodes(
                vec![],
            ));
        };
        if query_params
            .cluster_id
            .is_some_and(|cluster_id| cluster_id != observability.cluster_id())
        {
            return Ok(NodesGetResponse::Status200_SuccessfullyReturnedAllNodes(
                vec![],
            ));
        }
        let node = match observability.node_snapshot().await {
            Ok(node) => node,
            Err(err) => {
                return Ok(NodesGetResponse::Status500_ServerError(Self::error(
                    500,
                    err.to_string(),
                )));
            }
        };
        Ok(NodesGetResponse::Status200_SuccessfullyReturnedAllNodes(
            vec![models::Node::from(node)],
        ))
    }

    async fn nodes_node_id_activation_revocations_post(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
        _claims: &Self::Claims,
        path_params: &models::NodesNodeIdActivationRevocationsPostPathParams,
        body: &models::ActivationRevocationRequest,
    ) -> Result<NodesNodeIdActivationRevocationsPostResponse, ()> {
        use crate::sandbox::admission::{ActivationRevocation, NodeAdmission};
        use NodesNodeIdActivationRevocationsPostResponse as Response;
        let Some(observability) = self.observability() else {
            return Ok(Response::Status404_NotFound(Self::error(
                404,
                "node identity is unavailable",
            )));
        };
        if path_params.node_id != observability.node_id()
            || body.cluster_id != observability.cluster_id()
            || body.service_instance_id.to_string() != observability.service_instance_id()
        {
            return Ok(Response::Status409_NodeInstanceMismatch(Self::error(
                409,
                "revocation does not address this exact node service instance",
            )));
        }
        if body.activation_id.is_nil() || body.sandbox_id.is_nil() {
            return Ok(Response::Status400_BadRequest(Self::error(
                400,
                "activation and sandbox identities are required",
            )));
        }
        let Some(admission) = NodeAdmission::global() else {
            return Ok(Response::Status500_ServerError(Self::error(
                500,
                "durable execution admission is unavailable",
            )));
        };
        let sandbox = crate::types::SandboxId::from_uuid(body.sandbox_id);
        let disposition = match admission
            .revoke_unadmitted_activation(sandbox, body.activation_id)
            .await
        {
            Ok(ActivationRevocation::NeverAdmitted) => "NeverAdmitted",
            Ok(ActivationRevocation::PreviouslyAdmitted) => "PreviouslyAdmitted",
            Err(error) => {
                return Ok(Response::Status500_ServerError(Self::error(
                    500,
                    error.to_string(),
                )))
            }
        };
        Ok(Response::Status200_DurableAdmissionDisposition(
            models::ActivationRevocationObservation::new(
                observability.node_id().to_owned(),
                body.cluster_id,
                body.service_instance_id,
                body.sandbox_id,
                body.activation_id,
                disposition.to_owned(),
            ),
        ))
    }

    async fn nodes_node_id_drain_post(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
        _claims: &Self::Claims,
        path_params: &models::NodesNodeIdDrainPostPathParams,
        body: &models::NodeDrainRequest,
    ) -> Result<NodesNodeIdDrainPostResponse, ()> {
        let Some(observability) = self.observability() else {
            return Ok(NodesNodeIdDrainPostResponse::Status404_NotFound(
                Self::error(404, "node identity is unavailable"),
            ));
        };
        if path_params.node_id != observability.node_id()
            || body.cluster_id != observability.cluster_id()
            || body.service_instance_id != observability.service_instance_id()
        {
            return Ok(
                NodesNodeIdDrainPostResponse::Status409_NodeInstanceMismatch(Self::error(
                    409,
                    "drain request does not address this exact node service instance",
                )),
            );
        }
        if let Err(error) = self.orchestrator().drain_node(body.drain_id.clone()).await {
            return Ok(NodesNodeIdDrainPostResponse::Status500_ServerError(
                Self::error(500, error.to_string()),
            ));
        }
        let node = match observability.node_snapshot().await {
            Ok(node) => node,
            Err(error) => {
                return Ok(NodesNodeIdDrainPostResponse::Status500_ServerError(
                    Self::error(500, error.to_string()),
                ))
            }
        };
        let admission = self.orchestrator().node_admission_status();
        let operations = self.orchestrator().node_operation_status();
        Ok(
            NodesNodeIdDrainPostResponse::Status200_DurableAdmissionObservation(
                models::NodeDrainObservation {
                    node_id: node.node_id,
                    service_instance_id: node.service_instance_id,
                    drain_id: body.drain_id.clone(),
                    admission_closed: admission.closed,
                    in_flight_starts: admission.in_flight as u64,
                    in_flight_operations: operations.in_flight,
                    interrupted_operations: operations.interrupted,
                    sandbox_ids: node
                        .sandbox_ids
                        .into_iter()
                        .map(|id| id.to_string())
                        .collect(),
                    sandbox_count: node.sandbox_count.into(),
                    paused_sandbox_count: node.paused_sandbox_count.into(),
                    sandbox_starting_count: node.sandbox_starting_count.into(),
                },
            ),
        )
    }

    async fn nodes_node_id_get(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
        _claims: &Self::Claims,
        path_params: &models::NodesNodeIdGetPathParams,
        query_params: &models::NodesNodeIdGetQueryParams,
    ) -> Result<NodesNodeIdGetResponse, ()> {
        let Some(observability) = self.observability() else {
            // A disabled observability service behaves like node details are
            // unavailable on this process.
            return Ok(NodesNodeIdGetResponse::Status404_NotFound(Self::error(
                404,
                "observability is disabled on this node",
            )));
        };
        let cluster_mismatch = query_params
            .cluster_id
            .map(|cluster_id| cluster_id != observability.cluster_id())
            .unwrap_or(false);
        if path_params.node_id != observability.node_id() || cluster_mismatch {
            return Ok(NodesNodeIdGetResponse::Status404_NotFound(Self::error(
                404,
                format!("node {} not found", path_params.node_id),
            )));
        }

        let node = match observability.node_snapshot().await {
            Ok(node) => node,
            Err(err) => {
                return Ok(NodesNodeIdGetResponse::Status500_ServerError(Self::error(
                    500,
                    err.to_string(),
                )));
            }
        };

        let detail = models::NodeDetail::new(
            node.cluster_id.to_string(),
            node.version,
            node.commit,
            node.node_id,
            node.service_instance_id,
            node.machine_info.into(),
            if node.admission_closed {
                models::NodeStatus::NodeStatusDraining
            } else {
                models::NodeStatus::NodeStatusReady
            },
            node.sandbox_count,
            node.metrics.into(),
            vec![],
            node.create_successes,
            node.create_fails,
            node.paused_sandbox_count,
        );
        Ok(NodesNodeIdGetResponse::Status200_SuccessfullyReturnedTheNode(detail))
    }
}
