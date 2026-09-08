//! One shared incarnation precondition for cold/warm create, resume and fork.
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

impl ApiImpl {
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
                "launch does not address this exact node service instance",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
