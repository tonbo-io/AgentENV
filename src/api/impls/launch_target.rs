//! Shared node incarnation precondition for launch and terminal lifecycle requests.
use super::ApiImpl;
use agentenv_http_server::models;
use uuid::Uuid;

fn matches_target(target: &models::NodeLaunchTarget, actual: Option<(&str, Uuid, &str)>) -> bool {
    let Some((node, cluster, service)) = actual else {
        return false;
    };
    !target.node_id.is_empty()
        && !target.cluster_id.is_nil()
        && !target.service_instance_id.is_nil()
        && target.node_id == node
        && target.cluster_id == cluster
        && target.service_instance_id.to_string() == service
}

fn lifecycle_target(
    node: Option<&str>,
    cluster: Option<Uuid>,
    service: Option<Uuid>,
    activation: Option<Uuid>,
) -> Result<Option<models::NodeLaunchTarget>, ()> {
    match (node, cluster, service) {
        // Existing SQL orchestrator and public SDK lifecycle callers omit this
        // tuple until Kubernetes execution-authority handoff retires them.
        (None, None, None) => Ok(None),
        (Some(node), Some(cluster), Some(service)) if activation.is_some_and(|id| !id.is_nil()) => {
            Ok(Some(models::NodeLaunchTarget::new(
                node.to_owned(),
                cluster,
                service,
            )))
        }
        _ => Err(()),
    }
}

impl ApiImpl {
    pub(crate) fn check_lifecycle_target(
        &self,
        node: Option<&str>,
        cluster: Option<Uuid>,
        service: Option<Uuid>,
        activation: Option<Uuid>,
    ) -> Result<(), models::Error> {
        let target = lifecycle_target(node, cluster, service, activation).map_err(|_| {
            Self::error(
                409,
                "lifecycle requires a complete node target and activation",
            )
        })?;
        self.check_launch_target(target.as_ref())
    }

    pub(crate) fn check_launch_target(
        &self,
        target: Option<&models::NodeLaunchTarget>,
    ) -> Result<(), models::Error> {
        // Existing Cloud SQL orchestration/gateway callers omit the target.
        // Retire that compatibility at the Kubernetes execution-authority handoff.
        let Some(target) = target else {
            return Ok(());
        };
        let observability = self.observability();
        let actual = observability.as_ref().map(|node| {
            (
                node.node_id(),
                node.cluster_id(),
                node.service_instance_id(),
            )
        });
        if matches_target(target, actual) {
            Ok(())
        } else {
            Err(Self::error(
                409,
                "request does not address this exact node service instance",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lifecycle_target_requires_complete_tuple_and_activation() {
        let cluster = Uuid::from_u128(1);
        let service = Uuid::from_u128(2);
        let activation = Uuid::from_u128(3);
        for mask in 0..8 {
            let node = (mask & 1 != 0).then_some("node-a");
            let cluster = (mask & 2 != 0).then_some(cluster);
            let service = (mask & 4 != 0).then_some(service);
            for expected_activation in [None, Some(Uuid::nil()), Some(activation)] {
                let result = lifecycle_target(node, cluster, service, expected_activation);
                if mask == 0 {
                    assert!(result.unwrap().is_none());
                } else if mask == 7 && expected_activation == Some(activation) {
                    let target = result.unwrap().unwrap();
                    assert!(matches_target(
                        &target,
                        Some(("node-a", cluster.unwrap(), &service.unwrap().to_string()))
                    ));
                } else {
                    assert!(result.is_err(), "partial target mask {mask}");
                }
            }
        }
    }

    #[test]
    fn stale_wrong_missing_and_nil_node_identity_cannot_launch() {
        let target =
            models::NodeLaunchTarget::new("node-a".into(), Uuid::from_u128(1), Uuid::from_u128(2));
        let service = target.service_instance_id.to_string();
        assert!(matches_target(
            &target,
            Some(("node-a", target.cluster_id, &service))
        ));
        assert!(!matches_target(&target, None));
        assert!(!matches_target(
            &target,
            Some(("node-b", target.cluster_id, &service))
        ));
        assert!(!matches_target(
            &target,
            Some(("node-a", Uuid::from_u128(3), &service))
        ));
        assert!(!matches_target(
            &target,
            Some(("node-a", target.cluster_id, &Uuid::from_u128(4).to_string()))
        ));
        let mut invalid = target.clone();
        invalid.cluster_id = Uuid::nil();
        assert!(!matches_target(
            &invalid,
            Some(("node-a", Uuid::nil(), &service))
        ));
        invalid = target.clone();
        invalid.service_instance_id = Uuid::nil();
        assert!(!matches_target(
            &invalid,
            Some(("node-a", target.cluster_id, &Uuid::nil().to_string()))
        ));
        invalid = target.clone();
        invalid.node_id.clear();
        assert!(!matches_target(
            &invalid,
            Some(("", target.cluster_id, &service))
        ));
    }
}
