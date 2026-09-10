#![allow(unused_qualifications)]

use http::HeaderValue;
use validator::Validate;

#[cfg(feature = "server")]
use crate::header;
use crate::{models, types::*};

#[allow(dead_code)]
fn from_validation_error(e: validator::ValidationError) -> validator::ValidationErrors {
    let mut errs = validator::ValidationErrors::new();
    errs.add("na", e);
    errs
}

#[allow(dead_code)]
pub fn check_xss_string(v: &str) -> std::result::Result<(), validator::ValidationError> {
    if ammonia::is_html(v) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[allow(dead_code)]
pub fn check_xss_vec_string(v: &[String]) -> std::result::Result<(), validator::ValidationError> {
    if v.iter().any(|i| ammonia::is_html(i)) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[allow(dead_code)]
pub fn check_xss_map_string(
    v: &std::collections::HashMap<String, String>,
) -> std::result::Result<(), validator::ValidationError> {
    if v.keys().any(|k| ammonia::is_html(k)) || v.values().any(|v| ammonia::is_html(v)) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[allow(dead_code)]
pub fn check_xss_map_nested<T>(
    v: &std::collections::HashMap<String, T>,
) -> std::result::Result<(), validator::ValidationError>
where
    T: validator::Validate,
{
    if v.keys().any(|k| ammonia::is_html(k)) || v.values().any(|v| v.validate().is_err()) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[allow(dead_code)]
pub fn check_xss_map<T>(
    v: &std::collections::HashMap<String, T>,
) -> std::result::Result<(), validator::ValidationError> {
    if v.keys().any(|k| ammonia::is_html(k)) {
        std::result::Result::Err(validator::ValidationError::new("xss detected"))
    } else {
        std::result::Result::Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct NodesGetQueryParams {
    /// Identifier of the cluster
    #[serde(rename = "clusterID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct NodesNodeIdActivationRevocationsPostPathParams {
    pub node_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct NodesNodeIdDrainPostPathParams {
    pub node_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct NodesNodeIdGetPathParams {
    pub node_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct NodesNodeIdGetQueryParams {
    /// Identifier of the cluster
    #[serde(rename = "clusterID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesGetQueryParams {
    /// Metadata query used to filter the sandboxes (e.g. \"user=abc&app=prod\"). Each key and values must be URL encoded.
    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdConnectPostPathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdCustomExtensionParamsGetPathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdCustomExtensionParamsPatchPathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdDeletePathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdDeleteQueryParams {
    /// Expected funded activation identity. A mismatch returns conflict without modifying the sandbox. Omission temporarily retains ID-only callers during lifecycle authority migration.
    #[serde(rename = "expectedActivationID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_activation_id: Option<uuid::Uuid>,
    /// Exact runtime node identity. Supply together with expectedClusterID, expectedServiceInstanceID and expectedActivationID; incomplete or mismatched targets are rejected before lifecycle work.
    #[serde(rename = "expectedNodeID")]
    #[validate(length(min = 1))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_node_id: Option<String>,
    /// Cluster incarnation of the expected runtime node.
    #[serde(rename = "expectedClusterID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_cluster_id: Option<uuid::Uuid>,
    /// Process incarnation of the expected runtime node. A replacement process rejects a delayed lifecycle request even when the endpoint is reused.
    #[serde(rename = "expectedServiceInstanceID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_service_instance_id: Option<uuid::Uuid>,
    /// Absolute guest executable path, without arguments, run only after the expected activation acquires terminal deletion. Requires expectedActivationID. Best effort, bounded to 30 seconds; physical deletion continues on command failure or timeout. Paused sandboxes are never resumed to execute it.
    #[serde(rename = "terminalCommand")]
    #[validate(
                        length(min = 1, max = 1024),
                          regex(path = *RE_SANDBOXESSANDBOXIDDELETEQUERYPARAMS_TERMINAL_COMMAND),
              )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_command: Option<String>,
}

lazy_static::lazy_static! {
    static ref RE_SANDBOXESSANDBOXIDDELETEQUERYPARAMS_TERMINAL_COMMAND: regex::Regex = regex::Regex::new("^/").unwrap();
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdForkPostPathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdGetPathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdNetworkPutPathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdPausePostPathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdPausePostQueryParams {
    /// Expected funded activation identity. A mismatch returns conflict without modifying the sandbox. Omission temporarily retains ID-only callers during lifecycle authority migration.
    #[serde(rename = "expectedActivationID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_activation_id: Option<uuid::Uuid>,
    /// Exact runtime node identity. Supply together with expectedClusterID, expectedServiceInstanceID and expectedActivationID; incomplete or mismatched targets are rejected before lifecycle work.
    #[serde(rename = "expectedNodeID")]
    #[validate(length(min = 1))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_node_id: Option<String>,
    /// Cluster incarnation of the expected runtime node.
    #[serde(rename = "expectedClusterID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_cluster_id: Option<uuid::Uuid>,
    /// Process incarnation of the expected runtime node. A replacement process rejects a delayed lifecycle request even when the endpoint is reused.
    #[serde(rename = "expectedServiceInstanceID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_service_instance_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdPausedSnapshotsPostPathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdRefreshesPostPathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdResumePostPathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdSnapshotsPostPathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdTimeoutPostPathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdUsageGetPathParams {
    pub sandbox_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxesSandboxIdUsageGetQueryParams {
    /// Read an exact runtime instance, including retained final usage after another instance resumes.
    #[serde(rename = "runtimeInstanceID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_instance_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct V2SandboxesGetQueryParams {
    /// Metadata query used to filter the sandboxes (e.g. \"user=abc&app=prod\"). Each key and values must be URL encoded.
    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
    /// Filter sandboxes by one or more states
    #[serde(rename = "state")]
    #[serde(default)]
    pub state: Vec<models::SandboxState>,
    /// Cursor to start the list from
    #[serde(rename = "nextToken")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
    /// Maximum number of items to return per page
    #[serde(rename = "limit")]
    #[validate(range(min = 1u32, max = 100u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ExportSnapshotRootfsImagePathParams {
    pub snapshot_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SnapshotsGetQueryParams {
    #[serde(rename = "sandboxID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_id: Option<String>,
    /// Filter snapshots by name or ID, optionally tag-qualified (e.g. \"my-snapshot\", \"my-team/my-snapshot\" or \"my-snapshot:v1\").
    #[serde(rename = "name")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Maximum number of items to return per page
    #[serde(rename = "limit")]
    #[validate(range(min = 1u32, max = 100u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Cursor to start the list from
    #[serde(rename = "nextToken")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SnapshotsSnapshotIdDeletePathParams {
    pub snapshot_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SnapshotsSnapshotIdGetPathParams {
    pub snapshot_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplatesAliasesAliasGetPathParams {
    pub alias: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplatesGetQueryParams {
    #[serde(rename = "teamID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplatesTemplateIdBuildsBuildIdStatusGetPathParams {
    pub template_id: String,
    pub build_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplatesTemplateIdDeletePathParams {
    pub template_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplatesTemplateIdGetPathParams {
    pub template_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplatesTemplateIdGetQueryParams {
    /// Cursor to start the list from
    #[serde(rename = "nextToken")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
    /// Maximum number of items to return per page
    #[serde(rename = "limit")]
    #[validate(range(min = 1u32, max = 100u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct V2TemplatesGetQueryParams {
    #[serde(rename = "teamID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    /// Cursor to start the list from
    #[serde(rename = "nextToken")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
    /// Maximum number of items to return per page
    #[serde(rename = "limit")]
    #[validate(range(min = 1u32, max = 100u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct V2TemplatesTemplateIdBuildsBuildIdPostPathParams {
    pub template_id: String,
    pub build_id: String,
}

/// Admission history on this exact node only. NeverAdmitted prevents future first admission here; PreviouslyAdmitted requires physical reconciliation and proves neither running nor stopped. Neither is a cross-node fence or a host cleanup receipt.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ActivationRevocationObservation {
    #[serde(rename = "nodeID")]
    #[validate(custom(function = "check_xss_string"))]
    pub node_id: String,

    #[serde(rename = "clusterID")]
    pub cluster_id: uuid::Uuid,

    #[serde(rename = "serviceInstanceID")]
    pub service_instance_id: uuid::Uuid,

    #[serde(rename = "sandboxID")]
    pub sandbox_id: uuid::Uuid,

    #[serde(rename = "activationID")]
    pub activation_id: uuid::Uuid,

    /// Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "disposition")]
    #[validate(custom(function = "check_xss_string"))]
    pub disposition: String,
}

impl ActivationRevocationObservation {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        node_id: String,
        cluster_id: uuid::Uuid,
        service_instance_id: uuid::Uuid,
        sandbox_id: uuid::Uuid,
        activation_id: uuid::Uuid,
        disposition: String,
    ) -> ActivationRevocationObservation {
        ActivationRevocationObservation {
            node_id,
            cluster_id,
            service_instance_id,
            sandbox_id,
            activation_id,
            disposition,
        }
    }
}

/// Converts the ActivationRevocationObservation value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ActivationRevocationObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("nodeID".to_string()),
            Some(self.node_id.to_string()),
            // Skipping clusterID in query parameter serialization

            // Skipping serviceInstanceID in query parameter serialization

            // Skipping sandboxID in query parameter serialization

            // Skipping activationID in query parameter serialization
            Some("disposition".to_string()),
            Some(self.disposition.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ActivationRevocationObservation value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ActivationRevocationObservation {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub node_id: Vec<String>,
            pub cluster_id: Vec<uuid::Uuid>,
            pub service_instance_id: Vec<uuid::Uuid>,
            pub sandbox_id: Vec<uuid::Uuid>,
            pub activation_id: Vec<uuid::Uuid>,
            pub disposition: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing ActivationRevocationObservation".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "nodeID" => intermediate_rep.node_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "clusterID" => intermediate_rep.cluster_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "serviceInstanceID" => intermediate_rep.service_instance_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sandboxID" => intermediate_rep.sandbox_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "activationID" => intermediate_rep.activation_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "disposition" => intermediate_rep.disposition.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing ActivationRevocationObservation"
                                .to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ActivationRevocationObservation {
            node_id: intermediate_rep
                .node_id
                .into_iter()
                .next()
                .ok_or_else(|| "nodeID missing in ActivationRevocationObservation".to_string())?,
            cluster_id: intermediate_rep
                .cluster_id
                .into_iter()
                .next()
                .ok_or_else(|| {
                    "clusterID missing in ActivationRevocationObservation".to_string()
                })?,
            service_instance_id: intermediate_rep
                .service_instance_id
                .into_iter()
                .next()
                .ok_or_else(|| {
                    "serviceInstanceID missing in ActivationRevocationObservation".to_string()
                })?,
            sandbox_id: intermediate_rep
                .sandbox_id
                .into_iter()
                .next()
                .ok_or_else(|| {
                    "sandboxID missing in ActivationRevocationObservation".to_string()
                })?,
            activation_id: intermediate_rep
                .activation_id
                .into_iter()
                .next()
                .ok_or_else(|| {
                    "activationID missing in ActivationRevocationObservation".to_string()
                })?,
            disposition: intermediate_rep
                .disposition
                .into_iter()
                .next()
                .ok_or_else(|| {
                    "disposition missing in ActivationRevocationObservation".to_string()
                })?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ActivationRevocationObservation> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ActivationRevocationObservation>>
    for HeaderValue
{
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<ActivationRevocationObservation>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for ActivationRevocationObservation - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue>
    for header::IntoHeaderValue<ActivationRevocationObservation>
{
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <ActivationRevocationObservation as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into ActivationRevocationObservation - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ActivationRevocationRequest {
    #[serde(rename = "clusterID")]
    pub cluster_id: uuid::Uuid,

    #[serde(rename = "serviceInstanceID")]
    pub service_instance_id: uuid::Uuid,

    #[serde(rename = "sandboxID")]
    pub sandbox_id: uuid::Uuid,

    #[serde(rename = "activationID")]
    pub activation_id: uuid::Uuid,
}

impl ActivationRevocationRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        cluster_id: uuid::Uuid,
        service_instance_id: uuid::Uuid,
        sandbox_id: uuid::Uuid,
        activation_id: uuid::Uuid,
    ) -> ActivationRevocationRequest {
        ActivationRevocationRequest {
            cluster_id,
            service_instance_id,
            sandbox_id,
            activation_id,
        }
    }
}

/// Converts the ActivationRevocationRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ActivationRevocationRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping clusterID in query parameter serialization

            // Skipping serviceInstanceID in query parameter serialization

            // Skipping sandboxID in query parameter serialization

            // Skipping activationID in query parameter serialization

        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ActivationRevocationRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ActivationRevocationRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub cluster_id: Vec<uuid::Uuid>,
            pub service_instance_id: Vec<uuid::Uuid>,
            pub sandbox_id: Vec<uuid::Uuid>,
            pub activation_id: Vec<uuid::Uuid>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing ActivationRevocationRequest".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "clusterID" => intermediate_rep.cluster_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "serviceInstanceID" => intermediate_rep.service_instance_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sandboxID" => intermediate_rep.sandbox_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "activationID" => intermediate_rep.activation_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing ActivationRevocationRequest".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ActivationRevocationRequest {
            cluster_id: intermediate_rep
                .cluster_id
                .into_iter()
                .next()
                .ok_or_else(|| "clusterID missing in ActivationRevocationRequest".to_string())?,
            service_instance_id: intermediate_rep
                .service_instance_id
                .into_iter()
                .next()
                .ok_or_else(|| {
                    "serviceInstanceID missing in ActivationRevocationRequest".to_string()
                })?,
            sandbox_id: intermediate_rep
                .sandbox_id
                .into_iter()
                .next()
                .ok_or_else(|| "sandboxID missing in ActivationRevocationRequest".to_string())?,
            activation_id: intermediate_rep
                .activation_id
                .into_iter()
                .next()
                .ok_or_else(|| "activationID missing in ActivationRevocationRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ActivationRevocationRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ActivationRevocationRequest>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<ActivationRevocationRequest>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for ActivationRevocationRequest - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ActivationRevocationRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <ActivationRevocationRequest as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into ActivationRevocationRequest - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Block drive to attach when starting a sandbox. Attached drives are sandbox launch inputs; if the sandbox is later snapshotted, the current drive state is captured into the resulting snapshot.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct AttachedDrive {
    /// Identifier used for the attached Firecracker drive. Must be non-empty, unique per sandbox, and must not contain \"/\".
    #[serde(rename = "driveID")]
    #[validate(custom(function = "check_xss_string"))]
    pub drive_id: String,

    /// Whether the attached drive should be mounted read-only. Defaults to true.
    #[serde(rename = "readOnly")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,

    /// Absolute guest mount path. Defaults to `/mnt/{driveID}`. Must not be `/`, contain `..`, spaces, commas, or colons, and must not be under reserved paths such as `/proc`, `/sys`, `/dev`, `/run`, `/agentenv`, or `/opt/agentenv`.
    #[serde(rename = "mountPath")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_path: Option<String>,

    /// Optional sub-directory inside the drive to expose at `mountPath`, matching Kubernetes `volumeMounts.subPath`. When set, only the named sub-directory of the drive is visible at `mountPath` inside the guest. Must be a relative path, must not be empty, must not contain `..`, spaces, commas, or colons, and must not start with `/`.
    #[serde(rename = "subPath")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_path: Option<String>,

    /// Disk size for the sandbox in MiB
    #[serde(rename = "diskSizeMB")]
    #[validate(range(min = 0u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_size_mb: Option<u32>,

    #[serde(rename = "source")]
    #[validate(nested)]
    pub source: models::AttachedDriveSource,
}

impl AttachedDrive {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(drive_id: String, source: models::AttachedDriveSource) -> AttachedDrive {
        AttachedDrive {
            drive_id,
            read_only: Some(true),
            mount_path: None,
            sub_path: None,
            disk_size_mb: None,
            source,
        }
    }
}

/// Converts the AttachedDrive value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for AttachedDrive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("driveID".to_string()),
            Some(self.drive_id.to_string()),
            self.read_only
                .as_ref()
                .map(|read_only| ["readOnly".to_string(), read_only.to_string()].join(",")),
            self.mount_path
                .as_ref()
                .map(|mount_path| ["mountPath".to_string(), mount_path.to_string()].join(",")),
            self.sub_path
                .as_ref()
                .map(|sub_path| ["subPath".to_string(), sub_path.to_string()].join(",")),
            self.disk_size_mb
                .as_ref()
                .map(|disk_size_mb| ["diskSizeMB".to_string(), disk_size_mb.to_string()].join(",")),
            // Skipping source in query parameter serialization
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a AttachedDrive value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for AttachedDrive {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub drive_id: Vec<String>,
            pub read_only: Vec<bool>,
            pub mount_path: Vec<String>,
            pub sub_path: Vec<String>,
            pub disk_size_mb: Vec<u32>,
            pub source: Vec<models::AttachedDriveSource>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing AttachedDrive".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "driveID" => intermediate_rep.drive_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "readOnly" => intermediate_rep.read_only.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "mountPath" => intermediate_rep.mount_path.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "subPath" => intermediate_rep.sub_path.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "diskSizeMB" => intermediate_rep.disk_size_mb.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "source" => intermediate_rep.source.push(
                        <models::AttachedDriveSource as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing AttachedDrive".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(AttachedDrive {
            drive_id: intermediate_rep
                .drive_id
                .into_iter()
                .next()
                .ok_or_else(|| "driveID missing in AttachedDrive".to_string())?,
            read_only: intermediate_rep.read_only.into_iter().next(),
            mount_path: intermediate_rep.mount_path.into_iter().next(),
            sub_path: intermediate_rep.sub_path.into_iter().next(),
            disk_size_mb: intermediate_rep.disk_size_mb.into_iter().next(),
            source: intermediate_rep
                .source
                .into_iter()
                .next()
                .ok_or_else(|| "source missing in AttachedDrive".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<AttachedDrive> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<AttachedDrive>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<AttachedDrive>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for AttachedDrive - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<AttachedDrive> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <AttachedDrive as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into AttachedDrive - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Source for a sandbox attached drive. `image` must be provided.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct AttachedDriveSource {
    /// OCI image reference to resolve into an OverlayBD drive.
    #[serde(rename = "image")]
    #[validate(custom(function = "check_xss_string"))]
    pub image: String,
}

impl AttachedDriveSource {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(image: String) -> AttachedDriveSource {
        AttachedDriveSource { image }
    }
}

/// Converts the AttachedDriveSource value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for AttachedDriveSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> =
            vec![Some("image".to_string()), Some(self.image.to_string())];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a AttachedDriveSource value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for AttachedDriveSource {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub image: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing AttachedDriveSource".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "image" => intermediate_rep.image.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing AttachedDriveSource".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(AttachedDriveSource {
            image: intermediate_rep
                .image
                .into_iter()
                .next()
                .ok_or_else(|| "image missing in AttachedDriveSource".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<AttachedDriveSource> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<AttachedDriveSource>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<AttachedDriveSource>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for AttachedDriveSource - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<AttachedDriveSource> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <AttachedDriveSource as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into AttachedDriveSource - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct BuildLogEntry {
    /// Timestamp of the log entry
    #[serde(rename = "timestamp")]
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Log message content
    #[serde(rename = "message")]
    #[validate(custom(function = "check_xss_string"))]
    pub message: String,

    #[serde(rename = "level")]
    #[validate(nested)]
    pub level: models::LogLevel,

    /// Step in the build process related to the log entry
    #[serde(rename = "step")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
}

impl BuildLogEntry {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        timestamp: chrono::DateTime<chrono::Utc>,
        message: String,
        level: models::LogLevel,
    ) -> BuildLogEntry {
        BuildLogEntry {
            timestamp,
            message,
            level,
            step: None,
        }
    }
}

/// Converts the BuildLogEntry value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for BuildLogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping timestamp in query parameter serialization
            Some("message".to_string()),
            Some(self.message.to_string()),
            // Skipping level in query parameter serialization
            self.step
                .as_ref()
                .map(|step| ["step".to_string(), step.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a BuildLogEntry value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for BuildLogEntry {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub timestamp: Vec<chrono::DateTime<chrono::Utc>>,
            pub message: Vec<String>,
            pub level: Vec<models::LogLevel>,
            pub step: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing BuildLogEntry".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "timestamp" => intermediate_rep.timestamp.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "message" => intermediate_rep.message.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "level" => intermediate_rep.level.push(
                        <models::LogLevel as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "step" => intermediate_rep.step.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing BuildLogEntry".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(BuildLogEntry {
            timestamp: intermediate_rep
                .timestamp
                .into_iter()
                .next()
                .ok_or_else(|| "timestamp missing in BuildLogEntry".to_string())?,
            message: intermediate_rep
                .message
                .into_iter()
                .next()
                .ok_or_else(|| "message missing in BuildLogEntry".to_string())?,
            level: intermediate_rep
                .level
                .into_iter()
                .next()
                .ok_or_else(|| "level missing in BuildLogEntry".to_string())?,
            step: intermediate_rep.step.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<BuildLogEntry> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<BuildLogEntry>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<BuildLogEntry>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for BuildLogEntry - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<BuildLogEntry> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <BuildLogEntry as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into BuildLogEntry - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct BuildStatusReason {
    /// Message with the status reason, currently reporting only for error status
    #[serde(rename = "message")]
    #[validate(custom(function = "check_xss_string"))]
    pub message: String,

    /// Step that failed
    #[serde(rename = "step")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,

    /// Log entries related to the status reason
    #[serde(rename = "logEntries")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_entries: Option<Vec<models::BuildLogEntry>>,
}

impl BuildStatusReason {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(message: String) -> BuildStatusReason {
        BuildStatusReason {
            message,
            step: None,
            log_entries: None,
        }
    }
}

/// Converts the BuildStatusReason value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for BuildStatusReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("message".to_string()),
            Some(self.message.to_string()),
            self.step
                .as_ref()
                .map(|step| ["step".to_string(), step.to_string()].join(",")),
            // Skipping logEntries in query parameter serialization
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a BuildStatusReason value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for BuildStatusReason {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub message: Vec<String>,
            pub step: Vec<String>,
            pub log_entries: Vec<Vec<models::BuildLogEntry>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing BuildStatusReason".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "message" => intermediate_rep.message.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "step" => intermediate_rep.step.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "logEntries" => return std::result::Result::Err(
                        "Parsing a container in this style is not supported in BuildStatusReason"
                            .to_string(),
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing BuildStatusReason".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(BuildStatusReason {
            message: intermediate_rep
                .message
                .into_iter()
                .next()
                .ok_or_else(|| "message missing in BuildStatusReason".to_string())?,
            step: intermediate_rep.step.into_iter().next(),
            log_entries: intermediate_rep.log_entries.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<BuildStatusReason> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<BuildStatusReason>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<BuildStatusReason>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for BuildStatusReason - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<BuildStatusReason> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <BuildStatusReason as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into BuildStatusReason - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ConnectSandbox {
    #[serde(rename = "executionLease")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_lease: Option<models::ExecutionLease>,

    /// Timeout in seconds from the current time after which the sandbox should expire
    #[serde(rename = "timeout")]
    #[validate(range(min = 0u32))]
    pub timeout: u32,
}

impl ConnectSandbox {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(timeout: u32) -> ConnectSandbox {
        ConnectSandbox {
            execution_lease: None,
            timeout,
        }
    }
}

/// Converts the ConnectSandbox value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ConnectSandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping executionLease in query parameter serialization
            Some("timeout".to_string()),
            Some(self.timeout.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ConnectSandbox value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ConnectSandbox {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub execution_lease: Vec<models::ExecutionLease>,
            pub timeout: Vec<u32>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing ConnectSandbox".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "executionLease" => intermediate_rep.execution_lease.push(
                        <models::ExecutionLease as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "timeout" => intermediate_rep.timeout.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing ConnectSandbox".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ConnectSandbox {
            execution_lease: intermediate_rep.execution_lease.into_iter().next(),
            timeout: intermediate_rep
                .timeout
                .into_iter()
                .next()
                .ok_or_else(|| "timeout missing in ConnectSandbox".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ConnectSandbox> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ConnectSandbox>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<ConnectSandbox>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for ConnectSandbox - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ConnectSandbox> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <ConnectSandbox as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into ConnectSandbox - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// CPU cores for the sandbox
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CpuCount {}

impl CpuCount {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> CpuCount {
        CpuCount {}
    }
}

/// Converts the CpuCount value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for CpuCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a CpuCount value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for CpuCount {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {}

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing CpuCount".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing CpuCount".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CpuCount {})
    }
}

// Methods for converting between header::IntoHeaderValue<CpuCount> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<CpuCount>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<CpuCount>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for CpuCount - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<CpuCount> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <CpuCount as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into CpuCount - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DiskMetrics {
    /// Mount point of the disk
    #[serde(rename = "mountPoint")]
    #[validate(custom(function = "check_xss_string"))]
    pub mount_point: String,

    /// Device name
    #[serde(rename = "device")]
    #[validate(custom(function = "check_xss_string"))]
    pub device: String,

    /// Filesystem type (e.g., ext4, xfs)
    #[serde(rename = "filesystemType")]
    #[validate(custom(function = "check_xss_string"))]
    pub filesystem_type: String,

    /// Used space in bytes
    #[serde(rename = "usedBytes")]
    pub used_bytes: u64,

    /// Total space in bytes
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
}

impl DiskMetrics {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        mount_point: String,
        device: String,
        filesystem_type: String,
        used_bytes: u64,
        total_bytes: u64,
    ) -> DiskMetrics {
        DiskMetrics {
            mount_point,
            device,
            filesystem_type,
            used_bytes,
            total_bytes,
        }
    }
}

/// Converts the DiskMetrics value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for DiskMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("mountPoint".to_string()),
            Some(self.mount_point.to_string()),
            Some("device".to_string()),
            Some(self.device.to_string()),
            Some("filesystemType".to_string()),
            Some(self.filesystem_type.to_string()),
            Some("usedBytes".to_string()),
            Some(self.used_bytes.to_string()),
            Some("totalBytes".to_string()),
            Some(self.total_bytes.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a DiskMetrics value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for DiskMetrics {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub mount_point: Vec<String>,
            pub device: Vec<String>,
            pub filesystem_type: Vec<String>,
            pub used_bytes: Vec<u64>,
            pub total_bytes: Vec<u64>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing DiskMetrics".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "mountPoint" => intermediate_rep.mount_point.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "device" => intermediate_rep.device.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "filesystemType" => intermediate_rep.filesystem_type.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "usedBytes" => intermediate_rep.used_bytes.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "totalBytes" => intermediate_rep.total_bytes.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing DiskMetrics".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(DiskMetrics {
            mount_point: intermediate_rep
                .mount_point
                .into_iter()
                .next()
                .ok_or_else(|| "mountPoint missing in DiskMetrics".to_string())?,
            device: intermediate_rep
                .device
                .into_iter()
                .next()
                .ok_or_else(|| "device missing in DiskMetrics".to_string())?,
            filesystem_type: intermediate_rep
                .filesystem_type
                .into_iter()
                .next()
                .ok_or_else(|| "filesystemType missing in DiskMetrics".to_string())?,
            used_bytes: intermediate_rep
                .used_bytes
                .into_iter()
                .next()
                .ok_or_else(|| "usedBytes missing in DiskMetrics".to_string())?,
            total_bytes: intermediate_rep
                .total_bytes
                .into_iter()
                .next()
                .ok_or_else(|| "totalBytes missing in DiskMetrics".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<DiskMetrics> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<DiskMetrics>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<DiskMetrics>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for DiskMetrics - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<DiskMetrics> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <DiskMetrics as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into DiskMetrics - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Disk size for the sandbox in MiB
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct DiskSizeMb {}

impl DiskSizeMb {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> DiskSizeMb {
        DiskSizeMb {}
    }
}

/// Converts the DiskSizeMb value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for DiskSizeMb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a DiskSizeMb value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for DiskSizeMb {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {}

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing DiskSizeMb".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing DiskSizeMb".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(DiskSizeMb {})
    }
}

// Methods for converting between header::IntoHeaderValue<DiskSizeMb> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<DiskSizeMb>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<DiskSizeMb>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for DiskSizeMb - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<DiskSizeMb> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <DiskSizeMb as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into DiskSizeMb - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Version of the envd running in the sandbox
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct EnvdVersion(pub String);

impl validator::Validate for EnvdVersion {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<String> for EnvdVersion {
    fn from(x: String) -> Self {
        EnvdVersion(x)
    }
}

impl std::fmt::Display for EnvdVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for EnvdVersion {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(EnvdVersion(x.to_string()))
    }
}

impl std::convert::From<EnvdVersion> for String {
    fn from(x: EnvdVersion) -> Self {
        x.0
    }
}

impl std::ops::Deref for EnvdVersion {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for EnvdVersion {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Error {
    /// Error code
    #[serde(rename = "code")]
    pub code: i32,

    /// Error
    #[serde(rename = "message")]
    #[validate(custom(function = "check_xss_string"))]
    pub message: String,
}

impl Error {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(code: i32, message: String) -> Error {
        Error { code, message }
    }
}

/// Converts the Error value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("code".to_string()),
            Some(self.code.to_string()),
            Some("message".to_string()),
            Some(self.message.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Error value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Error {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub code: Vec<i32>,
            pub message: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing Error".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "code" => intermediate_rep.code.push(
                        <i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "message" => intermediate_rep.message.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing Error".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Error {
            code: intermediate_rep
                .code
                .into_iter()
                .next()
                .ok_or_else(|| "code missing in Error".to_string())?,
            message: intermediate_rep
                .message
                .into_iter()
                .next()
                .ok_or_else(|| "message missing in Error".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Error> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Error>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<Error>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for Error - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Error> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <Error as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => {
                    std::result::Result::Ok(header::IntoHeaderValue(value))
                }
                std::result::Result::Err(err) => std::result::Result::Err(format!(
                    r#"Unable to convert header value '{value}' into Error - {err}"#
                )),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Absolute funded authorization for one physical activation. New activations require sequence zero; renewal increases sequence. Equal sequences must replay identical payloads. An expired activation cannot be renewed or restarted.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ExecutionLease {
    #[serde(rename = "activationId")]
    pub activation_id: uuid::Uuid,

    #[serde(rename = "operationId")]
    pub operation_id: uuid::Uuid,

    #[serde(rename = "sequence")]
    pub sequence: u64,

    /// Absolute UTC deadline in milliseconds since the Unix epoch.
    #[serde(rename = "expiresAtUnixMs")]
    pub expires_at_unix_ms: u64,
}

impl ExecutionLease {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        activation_id: uuid::Uuid,
        operation_id: uuid::Uuid,
        sequence: u64,
        expires_at_unix_ms: u64,
    ) -> ExecutionLease {
        ExecutionLease {
            activation_id,
            operation_id,
            sequence,
            expires_at_unix_ms,
        }
    }
}

/// Converts the ExecutionLease value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ExecutionLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping activationId in query parameter serialization

            // Skipping operationId in query parameter serialization
            Some("sequence".to_string()),
            Some(self.sequence.to_string()),
            Some("expiresAtUnixMs".to_string()),
            Some(self.expires_at_unix_ms.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ExecutionLease value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ExecutionLease {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub activation_id: Vec<uuid::Uuid>,
            pub operation_id: Vec<uuid::Uuid>,
            pub sequence: Vec<u64>,
            pub expires_at_unix_ms: Vec<u64>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing ExecutionLease".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "activationId" => intermediate_rep.activation_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "operationId" => intermediate_rep.operation_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sequence" => intermediate_rep.sequence.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "expiresAtUnixMs" => intermediate_rep.expires_at_unix_ms.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing ExecutionLease".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ExecutionLease {
            activation_id: intermediate_rep
                .activation_id
                .into_iter()
                .next()
                .ok_or_else(|| "activationId missing in ExecutionLease".to_string())?,
            operation_id: intermediate_rep
                .operation_id
                .into_iter()
                .next()
                .ok_or_else(|| "operationId missing in ExecutionLease".to_string())?,
            sequence: intermediate_rep
                .sequence
                .into_iter()
                .next()
                .ok_or_else(|| "sequence missing in ExecutionLease".to_string())?,
            expires_at_unix_ms: intermediate_rep
                .expires_at_unix_ms
                .into_iter()
                .next()
                .ok_or_else(|| "expiresAtUnixMs missing in ExecutionLease".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ExecutionLease> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ExecutionLease>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<ExecutionLease>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for ExecutionLease - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ExecutionLease> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <ExecutionLease as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into ExecutionLease - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ExecutionProxyTarget {
    #[serde(rename = "activationID")]
    pub activation_id: uuid::Uuid,
}

impl ExecutionProxyTarget {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(activation_id: uuid::Uuid) -> ExecutionProxyTarget {
        ExecutionProxyTarget { activation_id }
    }
}

/// Converts the ExecutionProxyTarget value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ExecutionProxyTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping activationID in query parameter serialization

        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ExecutionProxyTarget value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ExecutionProxyTarget {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub activation_id: Vec<uuid::Uuid>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing ExecutionProxyTarget".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "activationID" => intermediate_rep.activation_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing ExecutionProxyTarget".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ExecutionProxyTarget {
            activation_id: intermediate_rep
                .activation_id
                .into_iter()
                .next()
                .ok_or_else(|| "activationID missing in ExecutionProxyTarget".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ExecutionProxyTarget> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ExecutionProxyTarget>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<ExecutionProxyTarget>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for ExecutionProxyTarget - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ExecutionProxyTarget> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <ExecutionProxyTarget as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into ExecutionProxyTarget - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ListedSandbox {
    /// Identifier of the template from which is the sandbox created
    #[serde(rename = "templateID")]
    #[validate(custom(function = "check_xss_string"))]
    pub template_id: String,

    /// Alias of the template
    #[serde(rename = "alias")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,

    /// Identifier of the sandbox
    #[serde(rename = "sandboxID")]
    #[validate(custom(function = "check_xss_string"))]
    pub sandbox_id: String,

    /// Identifier of the client
    #[serde(rename = "clientID")]
    #[validate(custom(function = "check_xss_string"))]
    pub client_id: String,

    /// Time when the sandbox was started
    #[serde(rename = "startedAt")]
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// Immutable start boundary of the current running runtime activation. Changes after a successful pause and resume.
    #[serde(rename = "runtimeStartedAt")]
    pub runtime_started_at: chrono::DateTime<chrono::Utc>,

    /// Time when the sandbox will expire
    #[serde(rename = "endAt")]
    pub end_at: chrono::DateTime<chrono::Utc>,

    /// CPU cores for the sandbox
    #[serde(rename = "cpuCount")]
    #[validate(range(min = 1u32))]
    pub cpu_count: u32,

    /// Memory for the sandbox in MiB
    #[serde(rename = "memoryMB")]
    #[validate(range(min = 128u32))]
    pub memory_mb: u32,

    /// Disk size for the sandbox in MiB
    #[serde(rename = "diskSizeMB")]
    #[validate(range(min = 0u32))]
    pub disk_size_mb: u32,

    #[serde(rename = "metadata")]
    #[validate(custom(function = "check_xss_map_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,

    #[serde(rename = "state")]
    #[validate(nested)]
    pub state: models::SandboxState,

    /// Version of the envd running in the sandbox
    #[serde(rename = "envdVersion")]
    #[validate(custom(function = "check_xss_string"))]
    pub envd_version: String,
}

impl ListedSandbox {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        template_id: String,
        sandbox_id: String,
        client_id: String,
        started_at: chrono::DateTime<chrono::Utc>,
        runtime_started_at: chrono::DateTime<chrono::Utc>,
        end_at: chrono::DateTime<chrono::Utc>,
        cpu_count: u32,
        memory_mb: u32,
        disk_size_mb: u32,
        state: models::SandboxState,
        envd_version: String,
    ) -> ListedSandbox {
        ListedSandbox {
            template_id,
            alias: None,
            sandbox_id,
            client_id,
            started_at,
            runtime_started_at,
            end_at,
            cpu_count,
            memory_mb,
            disk_size_mb,
            metadata: None,
            state,
            envd_version,
        }
    }
}

/// Converts the ListedSandbox value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ListedSandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("templateID".to_string()),
            Some(self.template_id.to_string()),
            self.alias
                .as_ref()
                .map(|alias| ["alias".to_string(), alias.to_string()].join(",")),
            Some("sandboxID".to_string()),
            Some(self.sandbox_id.to_string()),
            Some("clientID".to_string()),
            Some(self.client_id.to_string()),
            // Skipping startedAt in query parameter serialization

            // Skipping runtimeStartedAt in query parameter serialization

            // Skipping endAt in query parameter serialization
            Some("cpuCount".to_string()),
            Some(self.cpu_count.to_string()),
            Some("memoryMB".to_string()),
            Some(self.memory_mb.to_string()),
            Some("diskSizeMB".to_string()),
            Some(self.disk_size_mb.to_string()),
            // Skipping metadata in query parameter serialization

            // Skipping state in query parameter serialization
            Some("envdVersion".to_string()),
            Some(self.envd_version.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ListedSandbox value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ListedSandbox {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub template_id: Vec<String>,
            pub alias: Vec<String>,
            pub sandbox_id: Vec<String>,
            pub client_id: Vec<String>,
            pub started_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub runtime_started_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub end_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub cpu_count: Vec<u32>,
            pub memory_mb: Vec<u32>,
            pub disk_size_mb: Vec<u32>,
            pub metadata: Vec<std::collections::HashMap<String, String>>,
            pub state: Vec<models::SandboxState>,
            pub envd_version: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing ListedSandbox".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "templateID" => intermediate_rep.template_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "alias" => intermediate_rep.alias.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sandboxID" => intermediate_rep.sandbox_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "clientID" => intermediate_rep.client_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "startedAt" => intermediate_rep.started_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "runtimeStartedAt" => intermediate_rep.runtime_started_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "endAt" => intermediate_rep.end_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "cpuCount" => intermediate_rep.cpu_count.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "memoryMB" => intermediate_rep.memory_mb.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "diskSizeMB" => intermediate_rep.disk_size_mb.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "metadata" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in ListedSandbox"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "state" => intermediate_rep.state.push(
                        <models::SandboxState as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "envdVersion" => intermediate_rep.envd_version.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing ListedSandbox".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ListedSandbox {
            template_id: intermediate_rep
                .template_id
                .into_iter()
                .next()
                .ok_or_else(|| "templateID missing in ListedSandbox".to_string())?,
            alias: intermediate_rep.alias.into_iter().next(),
            sandbox_id: intermediate_rep
                .sandbox_id
                .into_iter()
                .next()
                .ok_or_else(|| "sandboxID missing in ListedSandbox".to_string())?,
            client_id: intermediate_rep
                .client_id
                .into_iter()
                .next()
                .ok_or_else(|| "clientID missing in ListedSandbox".to_string())?,
            started_at: intermediate_rep
                .started_at
                .into_iter()
                .next()
                .ok_or_else(|| "startedAt missing in ListedSandbox".to_string())?,
            runtime_started_at: intermediate_rep
                .runtime_started_at
                .into_iter()
                .next()
                .ok_or_else(|| "runtimeStartedAt missing in ListedSandbox".to_string())?,
            end_at: intermediate_rep
                .end_at
                .into_iter()
                .next()
                .ok_or_else(|| "endAt missing in ListedSandbox".to_string())?,
            cpu_count: intermediate_rep
                .cpu_count
                .into_iter()
                .next()
                .ok_or_else(|| "cpuCount missing in ListedSandbox".to_string())?,
            memory_mb: intermediate_rep
                .memory_mb
                .into_iter()
                .next()
                .ok_or_else(|| "memoryMB missing in ListedSandbox".to_string())?,
            disk_size_mb: intermediate_rep
                .disk_size_mb
                .into_iter()
                .next()
                .ok_or_else(|| "diskSizeMB missing in ListedSandbox".to_string())?,
            metadata: intermediate_rep.metadata.into_iter().next(),
            state: intermediate_rep
                .state
                .into_iter()
                .next()
                .ok_or_else(|| "state missing in ListedSandbox".to_string())?,
            envd_version: intermediate_rep
                .envd_version
                .into_iter()
                .next()
                .ok_or_else(|| "envdVersion missing in ListedSandbox".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ListedSandbox> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ListedSandbox>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<ListedSandbox>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for ListedSandbox - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ListedSandbox> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <ListedSandbox as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into ListedSandbox - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// State of the sandbox
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum LogLevel {
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "error")]
    Error,
}

impl validator::Validate for LogLevel {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "debug" => std::result::Result::Ok(LogLevel::Debug),
            "info" => std::result::Result::Ok(LogLevel::Info),
            "warn" => std::result::Result::Ok(LogLevel::Warn),
            "error" => std::result::Result::Ok(LogLevel::Error),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct MachineInfo {
    /// CPU family of the node
    #[serde(rename = "cpuFamily")]
    #[validate(custom(function = "check_xss_string"))]
    pub cpu_family: String,

    /// CPU model of the node
    #[serde(rename = "cpuModel")]
    #[validate(custom(function = "check_xss_string"))]
    pub cpu_model: String,

    /// CPU model name of the node
    #[serde(rename = "cpuModelName")]
    #[validate(custom(function = "check_xss_string"))]
    pub cpu_model_name: String,

    /// CPU architecture of the node
    #[serde(rename = "cpuArchitecture")]
    #[validate(custom(function = "check_xss_string"))]
    pub cpu_architecture: String,
}

impl MachineInfo {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        cpu_family: String,
        cpu_model: String,
        cpu_model_name: String,
        cpu_architecture: String,
    ) -> MachineInfo {
        MachineInfo {
            cpu_family,
            cpu_model,
            cpu_model_name,
            cpu_architecture,
        }
    }
}

/// Converts the MachineInfo value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for MachineInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("cpuFamily".to_string()),
            Some(self.cpu_family.to_string()),
            Some("cpuModel".to_string()),
            Some(self.cpu_model.to_string()),
            Some("cpuModelName".to_string()),
            Some(self.cpu_model_name.to_string()),
            Some("cpuArchitecture".to_string()),
            Some(self.cpu_architecture.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a MachineInfo value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for MachineInfo {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub cpu_family: Vec<String>,
            pub cpu_model: Vec<String>,
            pub cpu_model_name: Vec<String>,
            pub cpu_architecture: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing MachineInfo".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "cpuFamily" => intermediate_rep.cpu_family.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "cpuModel" => intermediate_rep.cpu_model.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "cpuModelName" => intermediate_rep.cpu_model_name.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "cpuArchitecture" => intermediate_rep.cpu_architecture.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing MachineInfo".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(MachineInfo {
            cpu_family: intermediate_rep
                .cpu_family
                .into_iter()
                .next()
                .ok_or_else(|| "cpuFamily missing in MachineInfo".to_string())?,
            cpu_model: intermediate_rep
                .cpu_model
                .into_iter()
                .next()
                .ok_or_else(|| "cpuModel missing in MachineInfo".to_string())?,
            cpu_model_name: intermediate_rep
                .cpu_model_name
                .into_iter()
                .next()
                .ok_or_else(|| "cpuModelName missing in MachineInfo".to_string())?,
            cpu_architecture: intermediate_rep
                .cpu_architecture
                .into_iter()
                .next()
                .ok_or_else(|| "cpuArchitecture missing in MachineInfo".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<MachineInfo> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<MachineInfo>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<MachineInfo>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for MachineInfo - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<MachineInfo> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <MachineInfo as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into MachineInfo - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Memory for the sandbox in MiB
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct MemoryMb {}

impl MemoryMb {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> MemoryMb {
        MemoryMb {}
    }
}

/// Converts the MemoryMb value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for MemoryMb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a MemoryMb value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for MemoryMb {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {}

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing MemoryMb".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing MemoryMb".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(MemoryMb {})
    }
}

// Methods for converting between header::IntoHeaderValue<MemoryMb> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<MemoryMb>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<MemoryMb>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for MemoryMb - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<MemoryMb> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <MemoryMb as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into MemoryMb - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct NewColdSandbox {
    #[serde(rename = "targetNodeInstance")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_node_instance: Option<models::NodeLaunchTarget>,

    #[serde(rename = "executionLease")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_lease: Option<models::ExecutionLease>,

    /// Explicit external OCI image reference to use as the sandbox rootfs. Cold starts may pull and convert OCI layers on a cache miss and can take tens of seconds.
    #[serde(rename = "image")]
    #[validate(custom(function = "check_xss_string"))]
    pub image: String,

    /// Time to live for the sandbox in seconds.
    #[serde(rename = "timeout")]
    #[validate(range(min = 0u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,

    /// Automatically pauses the sandbox after the timeout
    #[serde(rename = "autoPause")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_pause: Option<bool>,

    #[serde(rename = "autoResume")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_resume: Option<models::SandboxAutoResumeConfig>,

    /// Secure all system communication with sandbox
    #[serde(rename = "secure")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure: Option<bool>,

    /// Allow sandbox to access the internet. When set to false, it behaves the same as specifying denyOut to 0.0.0.0/0 in the network config.
    #[serde(rename = "allowInternetAccess")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_internet_access: Option<bool>,

    #[serde(rename = "network")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<models::SandboxNetworkConfig>,

    #[serde(rename = "metadata")]
    #[validate(custom(function = "check_xss_map_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,

    #[serde(rename = "envVars")]
    #[validate(custom(function = "check_xss_map_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<std::collections::HashMap<String, String>>,

    /// Opaque JSON object interpreted only by the custom extension. An absent value and an empty object are equivalent: both mean empty params.
    #[serde(rename = "customExtensionParams")]
    #[validate(custom(function = "check_xss_map"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_extension_params: Option<std::collections::HashMap<String, crate::types::Object>>,

    /// CPU cores for the cold-start sandbox.
    #[serde(rename = "cpuCount")]
    #[validate(range(min = 1u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_count: Option<u32>,

    /// Memory for the cold-start sandbox in MiB.
    #[serde(rename = "memoryMB")]
    #[validate(range(min = 128u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<u32>,

    /// Disk size for the sandbox in MiB
    #[serde(rename = "diskSizeMB")]
    #[validate(range(min = 0u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_size_mb: Option<u32>,

    /// Attached drives for the cold-start sandbox. If the sandbox is later snapshotted, current drive state is captured into the snapshot.
    #[serde(rename = "attachedDrives")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_drives: Option<Vec<models::AttachedDrive>>,

    /// Additional kernel command-line arguments appended to the Firecracker boot args for this cold-start sandbox. Arguments are whitespace-separated and may contain only 0-9, a-z, A-Z, underscore, hyphen, dot, slash, plus, or equals sign, so base64 values are allowed. The server also applies the firecracker.allowed_extra_boot_args_prefixes allowlist; non-matching or invalid arguments are silently ignored.
    #[serde(rename = "extraBootArgs")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_boot_args: Option<String>,
}

impl NewColdSandbox {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(image: String) -> NewColdSandbox {
        NewColdSandbox {
            target_node_instance: None,
            execution_lease: None,
            image,
            timeout: Some(15),
            auto_pause: Some(true),
            auto_resume: None,
            secure: None,
            allow_internet_access: None,
            network: None,
            metadata: None,
            env_vars: None,
            custom_extension_params: None,
            cpu_count: None,
            memory_mb: None,
            disk_size_mb: None,
            attached_drives: None,
            extra_boot_args: None,
        }
    }
}

/// Converts the NewColdSandbox value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for NewColdSandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping targetNodeInstance in query parameter serialization

            // Skipping executionLease in query parameter serialization
            Some("image".to_string()),
            Some(self.image.to_string()),
            self.timeout
                .as_ref()
                .map(|timeout| ["timeout".to_string(), timeout.to_string()].join(",")),
            self.auto_pause
                .as_ref()
                .map(|auto_pause| ["autoPause".to_string(), auto_pause.to_string()].join(",")),
            // Skipping autoResume in query parameter serialization
            self.secure
                .as_ref()
                .map(|secure| ["secure".to_string(), secure.to_string()].join(",")),
            self.allow_internet_access
                .as_ref()
                .map(|allow_internet_access| {
                    [
                        "allowInternetAccess".to_string(),
                        allow_internet_access.to_string(),
                    ]
                    .join(",")
                }),
            // Skipping network in query parameter serialization

            // Skipping metadata in query parameter serialization

            // Skipping envVars in query parameter serialization

            // Skipping customExtensionParams in query parameter serialization
            // Skipping customExtensionParams in query parameter serialization
            self.cpu_count
                .as_ref()
                .map(|cpu_count| ["cpuCount".to_string(), cpu_count.to_string()].join(",")),
            self.memory_mb
                .as_ref()
                .map(|memory_mb| ["memoryMB".to_string(), memory_mb.to_string()].join(",")),
            self.disk_size_mb
                .as_ref()
                .map(|disk_size_mb| ["diskSizeMB".to_string(), disk_size_mb.to_string()].join(",")),
            // Skipping attachedDrives in query parameter serialization
            self.extra_boot_args.as_ref().map(|extra_boot_args| {
                ["extraBootArgs".to_string(), extra_boot_args.to_string()].join(",")
            }),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a NewColdSandbox value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for NewColdSandbox {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub target_node_instance: Vec<models::NodeLaunchTarget>,
            pub execution_lease: Vec<models::ExecutionLease>,
            pub image: Vec<String>,
            pub timeout: Vec<u32>,
            pub auto_pause: Vec<bool>,
            pub auto_resume: Vec<models::SandboxAutoResumeConfig>,
            pub secure: Vec<bool>,
            pub allow_internet_access: Vec<bool>,
            pub network: Vec<models::SandboxNetworkConfig>,
            pub metadata: Vec<std::collections::HashMap<String, String>>,
            pub env_vars: Vec<std::collections::HashMap<String, String>>,
            pub custom_extension_params:
                Vec<std::collections::HashMap<String, crate::types::Object>>,
            pub cpu_count: Vec<u32>,
            pub memory_mb: Vec<u32>,
            pub disk_size_mb: Vec<u32>,
            pub attached_drives: Vec<Vec<models::AttachedDrive>>,
            pub extra_boot_args: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing NewColdSandbox".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "targetNodeInstance" => intermediate_rep.target_node_instance.push(
                        <models::NodeLaunchTarget as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "executionLease" => intermediate_rep.execution_lease.push(
                        <models::ExecutionLease as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "image" => intermediate_rep.image.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "timeout" => intermediate_rep.timeout.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "autoPause" => intermediate_rep.auto_pause.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "autoResume" => intermediate_rep.auto_resume.push(
                        <models::SandboxAutoResumeConfig as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "secure" => intermediate_rep.secure.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "allowInternetAccess" => intermediate_rep.allow_internet_access.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "network" => intermediate_rep.network.push(
                        <models::SandboxNetworkConfig as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    "metadata" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in NewColdSandbox"
                                .to_string(),
                        );
                    }
                    "envVars" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in NewColdSandbox"
                                .to_string(),
                        );
                    }
                    "customExtensionParams" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in NewColdSandbox"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "cpuCount" => intermediate_rep.cpu_count.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "memoryMB" => intermediate_rep.memory_mb.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "diskSizeMB" => intermediate_rep.disk_size_mb.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "attachedDrives" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in NewColdSandbox"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "extraBootArgs" => intermediate_rep.extra_boot_args.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing NewColdSandbox".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(NewColdSandbox {
            target_node_instance: intermediate_rep.target_node_instance.into_iter().next(),
            execution_lease: intermediate_rep.execution_lease.into_iter().next(),
            image: intermediate_rep
                .image
                .into_iter()
                .next()
                .ok_or_else(|| "image missing in NewColdSandbox".to_string())?,
            timeout: intermediate_rep.timeout.into_iter().next(),
            auto_pause: intermediate_rep.auto_pause.into_iter().next(),
            auto_resume: intermediate_rep.auto_resume.into_iter().next(),
            secure: intermediate_rep.secure.into_iter().next(),
            allow_internet_access: intermediate_rep.allow_internet_access.into_iter().next(),
            network: intermediate_rep.network.into_iter().next(),
            metadata: intermediate_rep.metadata.into_iter().next(),
            env_vars: intermediate_rep.env_vars.into_iter().next(),
            custom_extension_params: intermediate_rep.custom_extension_params.into_iter().next(),
            cpu_count: intermediate_rep.cpu_count.into_iter().next(),
            memory_mb: intermediate_rep.memory_mb.into_iter().next(),
            disk_size_mb: intermediate_rep.disk_size_mb.into_iter().next(),
            attached_drives: intermediate_rep.attached_drives.into_iter().next(),
            extra_boot_args: intermediate_rep.extra_boot_args.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<NewColdSandbox> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<NewColdSandbox>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<NewColdSandbox>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for NewColdSandbox - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<NewColdSandbox> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <NewColdSandbox as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into NewColdSandbox - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct NewSandbox {
    #[serde(rename = "targetNodeInstance")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_node_instance: Option<models::NodeLaunchTarget>,

    #[serde(rename = "executionLease")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_lease: Option<models::ExecutionLease>,

    /// Identifier of the required template/snapshot. Use POST /sandboxes-cold to create a sandbox directly from an external OCI image.
    #[serde(rename = "templateID")]
    #[validate(custom(function = "check_xss_string"))]
    pub template_id: String,

    /// Time to live for the sandbox in seconds.
    #[serde(rename = "timeout")]
    #[validate(range(min = 0u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,

    /// Automatically pauses the sandbox after the timeout
    #[serde(rename = "autoPause")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_pause: Option<bool>,

    #[serde(rename = "autoResume")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_resume: Option<models::SandboxAutoResumeConfig>,

    /// Secure all system communication with sandbox
    #[serde(rename = "secure")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure: Option<bool>,

    /// Allow sandbox to access the internet. When set to false, it behaves the same as specifying denyOut to 0.0.0.0/0 in the network config.
    #[serde(rename = "allow_internet_access")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_internet_access: Option<bool>,

    #[serde(rename = "network")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<models::SandboxNetworkConfig>,

    #[serde(rename = "metadata")]
    #[validate(custom(function = "check_xss_map_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,

    #[serde(rename = "placement")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<models::SandboxPlacement>,

    #[serde(rename = "envVars")]
    #[validate(custom(function = "check_xss_map_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<std::collections::HashMap<String, String>>,

    /// Opaque JSON object interpreted only by the custom extension. An absent value and an empty object are equivalent: both mean empty params.
    #[serde(rename = "customExtensionParams")]
    #[validate(custom(function = "check_xss_map"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_extension_params: Option<std::collections::HashMap<String, crate::types::Object>>,

    /// MCP configuration for the sandbox
    #[serde(rename = "mcp")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp: Option<Nullable<std::collections::HashMap<String, crate::types::Object>>>,
}

impl NewSandbox {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(template_id: String) -> NewSandbox {
        NewSandbox {
            target_node_instance: None,
            execution_lease: None,
            template_id,
            timeout: Some(15),
            auto_pause: Some(true),
            auto_resume: None,
            secure: None,
            allow_internet_access: None,
            network: None,
            metadata: None,
            placement: None,
            env_vars: None,
            custom_extension_params: None,
            mcp: None,
        }
    }
}

/// Converts the NewSandbox value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for NewSandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping targetNodeInstance in query parameter serialization

            // Skipping executionLease in query parameter serialization
            Some("templateID".to_string()),
            Some(self.template_id.to_string()),
            self.timeout
                .as_ref()
                .map(|timeout| ["timeout".to_string(), timeout.to_string()].join(",")),
            self.auto_pause
                .as_ref()
                .map(|auto_pause| ["autoPause".to_string(), auto_pause.to_string()].join(",")),
            // Skipping autoResume in query parameter serialization
            self.secure
                .as_ref()
                .map(|secure| ["secure".to_string(), secure.to_string()].join(",")),
            self.allow_internet_access
                .as_ref()
                .map(|allow_internet_access| {
                    [
                        "allow_internet_access".to_string(),
                        allow_internet_access.to_string(),
                    ]
                    .join(",")
                }),
            // Skipping network in query parameter serialization

            // Skipping metadata in query parameter serialization

            // Skipping placement in query parameter serialization

            // Skipping envVars in query parameter serialization

            // Skipping customExtensionParams in query parameter serialization
            // Skipping customExtensionParams in query parameter serialization

            // Skipping mcp in query parameter serialization
            // Skipping mcp in query parameter serialization
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a NewSandbox value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for NewSandbox {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub target_node_instance: Vec<models::NodeLaunchTarget>,
            pub execution_lease: Vec<models::ExecutionLease>,
            pub template_id: Vec<String>,
            pub timeout: Vec<u32>,
            pub auto_pause: Vec<bool>,
            pub auto_resume: Vec<models::SandboxAutoResumeConfig>,
            pub secure: Vec<bool>,
            pub allow_internet_access: Vec<bool>,
            pub network: Vec<models::SandboxNetworkConfig>,
            pub metadata: Vec<std::collections::HashMap<String, String>>,
            pub placement: Vec<models::SandboxPlacement>,
            pub env_vars: Vec<std::collections::HashMap<String, String>>,
            pub custom_extension_params:
                Vec<std::collections::HashMap<String, crate::types::Object>>,
            pub mcp: Vec<std::collections::HashMap<String, crate::types::Object>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing NewSandbox".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "targetNodeInstance" => intermediate_rep.target_node_instance.push(
                        <models::NodeLaunchTarget as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "executionLease" => intermediate_rep.execution_lease.push(
                        <models::ExecutionLease as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "templateID" => intermediate_rep.template_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "timeout" => intermediate_rep.timeout.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "autoPause" => intermediate_rep.auto_pause.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "autoResume" => intermediate_rep.auto_resume.push(
                        <models::SandboxAutoResumeConfig as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "secure" => intermediate_rep.secure.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "allow_internet_access" => intermediate_rep.allow_internet_access.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "network" => intermediate_rep.network.push(
                        <models::SandboxNetworkConfig as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    "metadata" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in NewSandbox"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "placement" => intermediate_rep.placement.push(
                        <models::SandboxPlacement as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    "envVars" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in NewSandbox"
                                .to_string(),
                        );
                    }
                    "customExtensionParams" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in NewSandbox"
                                .to_string(),
                        );
                    }
                    "mcp" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in NewSandbox"
                                .to_string(),
                        );
                    }
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing NewSandbox".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(NewSandbox {
            target_node_instance: intermediate_rep.target_node_instance.into_iter().next(),
            execution_lease: intermediate_rep.execution_lease.into_iter().next(),
            template_id: intermediate_rep
                .template_id
                .into_iter()
                .next()
                .ok_or_else(|| "templateID missing in NewSandbox".to_string())?,
            timeout: intermediate_rep.timeout.into_iter().next(),
            auto_pause: intermediate_rep.auto_pause.into_iter().next(),
            auto_resume: intermediate_rep.auto_resume.into_iter().next(),
            secure: intermediate_rep.secure.into_iter().next(),
            allow_internet_access: intermediate_rep.allow_internet_access.into_iter().next(),
            network: intermediate_rep.network.into_iter().next(),
            metadata: intermediate_rep.metadata.into_iter().next(),
            placement: intermediate_rep.placement.into_iter().next(),
            env_vars: intermediate_rep.env_vars.into_iter().next(),
            custom_extension_params: intermediate_rep.custom_extension_params.into_iter().next(),
            mcp: std::result::Result::Err(
                "Nullable types not supported in NewSandbox".to_string(),
            )?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<NewSandbox> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<NewSandbox>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<NewSandbox>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for NewSandbox - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<NewSandbox> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <NewSandbox as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into NewSandbox - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Node {
    /// Host observation timestamp; repeated responses are not new samples.
    #[serde(rename = "reportedAtUnixMs")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported_at_unix_ms: Option<i64>,

    /// Opaque snapshot placement domain. Empty means compatibility is unknown.
    #[serde(rename = "snapshotCompatibilityKey")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_compatibility_key: Option<String>,

    /// Version of the orchestrator
    #[serde(rename = "version")]
    #[validate(custom(function = "check_xss_string"))]
    pub version: String,

    /// Commit of the orchestrator
    #[serde(rename = "commit")]
    #[validate(custom(function = "check_xss_string"))]
    pub commit: String,

    /// Identifier of the node
    #[serde(rename = "id")]
    #[validate(custom(function = "check_xss_string"))]
    pub id: String,

    /// Service instance identifier of the node
    #[serde(rename = "serviceInstanceID")]
    #[validate(custom(function = "check_xss_string"))]
    pub service_instance_id: String,

    /// Identifier of the cluster
    #[serde(rename = "clusterID")]
    #[validate(custom(function = "check_xss_string"))]
    pub cluster_id: String,

    #[serde(rename = "machineInfo")]
    #[validate(nested)]
    pub machine_info: models::MachineInfo,

    #[serde(rename = "status")]
    #[validate(nested)]
    pub status: models::NodeStatus,

    /// Number of sandboxes running on the node
    #[serde(rename = "sandboxCount")]
    pub sandbox_count: u32,

    #[serde(rename = "metrics")]
    #[validate(nested)]
    pub metrics: models::NodeMetrics,

    /// Number of sandbox create successes
    #[serde(rename = "createSuccesses")]
    pub create_successes: u64,

    /// Number of sandbox create fails
    #[serde(rename = "createFails")]
    pub create_fails: u64,

    /// Number of starting Sandboxes
    #[serde(rename = "sandboxStartingCount")]
    pub sandbox_starting_count: u32,

    /// Number of sandboxes currently in the Paused state
    #[serde(rename = "sandboxPausedCount")]
    pub sandbox_paused_count: u32,
}

impl Node {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        version: String,
        commit: String,
        id: String,
        service_instance_id: String,
        cluster_id: String,
        machine_info: models::MachineInfo,
        status: models::NodeStatus,
        sandbox_count: u32,
        metrics: models::NodeMetrics,
        create_successes: u64,
        create_fails: u64,
        sandbox_starting_count: u32,
        sandbox_paused_count: u32,
    ) -> Node {
        Node {
            reported_at_unix_ms: None,
            snapshot_compatibility_key: None,
            version,
            commit,
            id,
            service_instance_id,
            cluster_id,
            machine_info,
            status,
            sandbox_count,
            metrics,
            create_successes,
            create_fails,
            sandbox_starting_count,
            sandbox_paused_count,
        }
    }
}

/// Converts the Node value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            self.reported_at_unix_ms
                .as_ref()
                .map(|reported_at_unix_ms| {
                    [
                        "reportedAtUnixMs".to_string(),
                        reported_at_unix_ms.to_string(),
                    ]
                    .join(",")
                }),
            self.snapshot_compatibility_key
                .as_ref()
                .map(|snapshot_compatibility_key| {
                    [
                        "snapshotCompatibilityKey".to_string(),
                        snapshot_compatibility_key.to_string(),
                    ]
                    .join(",")
                }),
            Some("version".to_string()),
            Some(self.version.to_string()),
            Some("commit".to_string()),
            Some(self.commit.to_string()),
            Some("id".to_string()),
            Some(self.id.to_string()),
            Some("serviceInstanceID".to_string()),
            Some(self.service_instance_id.to_string()),
            Some("clusterID".to_string()),
            Some(self.cluster_id.to_string()),
            // Skipping machineInfo in query parameter serialization

            // Skipping status in query parameter serialization
            Some("sandboxCount".to_string()),
            Some(self.sandbox_count.to_string()),
            // Skipping metrics in query parameter serialization
            Some("createSuccesses".to_string()),
            Some(self.create_successes.to_string()),
            Some("createFails".to_string()),
            Some(self.create_fails.to_string()),
            Some("sandboxStartingCount".to_string()),
            Some(self.sandbox_starting_count.to_string()),
            Some("sandboxPausedCount".to_string()),
            Some(self.sandbox_paused_count.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Node value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Node {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub reported_at_unix_ms: Vec<i64>,
            pub snapshot_compatibility_key: Vec<String>,
            pub version: Vec<String>,
            pub commit: Vec<String>,
            pub id: Vec<String>,
            pub service_instance_id: Vec<String>,
            pub cluster_id: Vec<String>,
            pub machine_info: Vec<models::MachineInfo>,
            pub status: Vec<models::NodeStatus>,
            pub sandbox_count: Vec<u32>,
            pub metrics: Vec<models::NodeMetrics>,
            pub create_successes: Vec<u64>,
            pub create_fails: Vec<u64>,
            pub sandbox_starting_count: Vec<u32>,
            pub sandbox_paused_count: Vec<u32>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing Node".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "reportedAtUnixMs" => intermediate_rep.reported_at_unix_ms.push(
                        <i64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "snapshotCompatibilityKey" => intermediate_rep.snapshot_compatibility_key.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "version" => intermediate_rep.version.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "commit" => intermediate_rep.commit.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "serviceInstanceID" => intermediate_rep.service_instance_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "clusterID" => intermediate_rep.cluster_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "machineInfo" => intermediate_rep.machine_info.push(
                        <models::MachineInfo as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "status" => intermediate_rep.status.push(
                        <models::NodeStatus as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sandboxCount" => intermediate_rep.sandbox_count.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "metrics" => intermediate_rep.metrics.push(
                        <models::NodeMetrics as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "createSuccesses" => intermediate_rep.create_successes.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "createFails" => intermediate_rep.create_fails.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sandboxStartingCount" => intermediate_rep.sandbox_starting_count.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sandboxPausedCount" => intermediate_rep.sandbox_paused_count.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing Node".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Node {
            reported_at_unix_ms: intermediate_rep.reported_at_unix_ms.into_iter().next(),
            snapshot_compatibility_key: intermediate_rep
                .snapshot_compatibility_key
                .into_iter()
                .next(),
            version: intermediate_rep
                .version
                .into_iter()
                .next()
                .ok_or_else(|| "version missing in Node".to_string())?,
            commit: intermediate_rep
                .commit
                .into_iter()
                .next()
                .ok_or_else(|| "commit missing in Node".to_string())?,
            id: intermediate_rep
                .id
                .into_iter()
                .next()
                .ok_or_else(|| "id missing in Node".to_string())?,
            service_instance_id: intermediate_rep
                .service_instance_id
                .into_iter()
                .next()
                .ok_or_else(|| "serviceInstanceID missing in Node".to_string())?,
            cluster_id: intermediate_rep
                .cluster_id
                .into_iter()
                .next()
                .ok_or_else(|| "clusterID missing in Node".to_string())?,
            machine_info: intermediate_rep
                .machine_info
                .into_iter()
                .next()
                .ok_or_else(|| "machineInfo missing in Node".to_string())?,
            status: intermediate_rep
                .status
                .into_iter()
                .next()
                .ok_or_else(|| "status missing in Node".to_string())?,
            sandbox_count: intermediate_rep
                .sandbox_count
                .into_iter()
                .next()
                .ok_or_else(|| "sandboxCount missing in Node".to_string())?,
            metrics: intermediate_rep
                .metrics
                .into_iter()
                .next()
                .ok_or_else(|| "metrics missing in Node".to_string())?,
            create_successes: intermediate_rep
                .create_successes
                .into_iter()
                .next()
                .ok_or_else(|| "createSuccesses missing in Node".to_string())?,
            create_fails: intermediate_rep
                .create_fails
                .into_iter()
                .next()
                .ok_or_else(|| "createFails missing in Node".to_string())?,
            sandbox_starting_count: intermediate_rep
                .sandbox_starting_count
                .into_iter()
                .next()
                .ok_or_else(|| "sandboxStartingCount missing in Node".to_string())?,
            sandbox_paused_count: intermediate_rep
                .sandbox_paused_count
                .into_iter()
                .next()
                .ok_or_else(|| "sandboxPausedCount missing in Node".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Node> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Node>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<Node>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for Node - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Node> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => match <Node as std::str::FromStr>::from_str(value) {
                std::result::Result::Ok(value) => {
                    std::result::Result::Ok(header::IntoHeaderValue(value))
                }
                std::result::Result::Err(err) => std::result::Result::Err(format!(
                    r#"Unable to convert header value '{value}' into Node - {err}"#
                )),
            },
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct NodeDetail {
    /// Identifier of the cluster
    #[serde(rename = "clusterID")]
    #[validate(custom(function = "check_xss_string"))]
    pub cluster_id: String,

    /// Version of the orchestrator
    #[serde(rename = "version")]
    #[validate(custom(function = "check_xss_string"))]
    pub version: String,

    /// Commit of the orchestrator
    #[serde(rename = "commit")]
    #[validate(custom(function = "check_xss_string"))]
    pub commit: String,

    /// Identifier of the node
    #[serde(rename = "id")]
    #[validate(custom(function = "check_xss_string"))]
    pub id: String,

    /// Service instance identifier of the node
    #[serde(rename = "serviceInstanceID")]
    #[validate(custom(function = "check_xss_string"))]
    pub service_instance_id: String,

    #[serde(rename = "machineInfo")]
    #[validate(nested)]
    pub machine_info: models::MachineInfo,

    #[serde(rename = "status")]
    #[validate(nested)]
    pub status: models::NodeStatus,

    /// Number of sandboxes running on the node
    #[serde(rename = "sandboxCount")]
    pub sandbox_count: u32,

    #[serde(rename = "metrics")]
    #[validate(nested)]
    pub metrics: models::NodeMetrics,

    /// List of cached builds id on the node
    #[serde(rename = "cachedBuilds")]
    #[validate(custom(function = "check_xss_vec_string"))]
    pub cached_builds: Vec<String>,

    /// Number of sandbox create successes
    #[serde(rename = "createSuccesses")]
    pub create_successes: u64,

    /// Number of sandbox create fails
    #[serde(rename = "createFails")]
    pub create_fails: u64,

    /// Number of sandboxes currently in the Paused state
    #[serde(rename = "sandboxPausedCount")]
    pub sandbox_paused_count: u32,
}

impl NodeDetail {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        cluster_id: String,
        version: String,
        commit: String,
        id: String,
        service_instance_id: String,
        machine_info: models::MachineInfo,
        status: models::NodeStatus,
        sandbox_count: u32,
        metrics: models::NodeMetrics,
        cached_builds: Vec<String>,
        create_successes: u64,
        create_fails: u64,
        sandbox_paused_count: u32,
    ) -> NodeDetail {
        NodeDetail {
            cluster_id,
            version,
            commit,
            id,
            service_instance_id,
            machine_info,
            status,
            sandbox_count,
            metrics,
            cached_builds,
            create_successes,
            create_fails,
            sandbox_paused_count,
        }
    }
}

/// Converts the NodeDetail value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for NodeDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("clusterID".to_string()),
            Some(self.cluster_id.to_string()),
            Some("version".to_string()),
            Some(self.version.to_string()),
            Some("commit".to_string()),
            Some(self.commit.to_string()),
            Some("id".to_string()),
            Some(self.id.to_string()),
            Some("serviceInstanceID".to_string()),
            Some(self.service_instance_id.to_string()),
            // Skipping machineInfo in query parameter serialization

            // Skipping status in query parameter serialization
            Some("sandboxCount".to_string()),
            Some(self.sandbox_count.to_string()),
            // Skipping metrics in query parameter serialization
            Some("cachedBuilds".to_string()),
            Some(
                self.cached_builds
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            Some("createSuccesses".to_string()),
            Some(self.create_successes.to_string()),
            Some("createFails".to_string()),
            Some(self.create_fails.to_string()),
            Some("sandboxPausedCount".to_string()),
            Some(self.sandbox_paused_count.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a NodeDetail value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for NodeDetail {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub cluster_id: Vec<String>,
            pub version: Vec<String>,
            pub commit: Vec<String>,
            pub id: Vec<String>,
            pub service_instance_id: Vec<String>,
            pub machine_info: Vec<models::MachineInfo>,
            pub status: Vec<models::NodeStatus>,
            pub sandbox_count: Vec<u32>,
            pub metrics: Vec<models::NodeMetrics>,
            pub cached_builds: Vec<Vec<String>>,
            pub create_successes: Vec<u64>,
            pub create_fails: Vec<u64>,
            pub sandbox_paused_count: Vec<u32>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing NodeDetail".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "clusterID" => intermediate_rep.cluster_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "version" => intermediate_rep.version.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "commit" => intermediate_rep.commit.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "serviceInstanceID" => intermediate_rep.service_instance_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "machineInfo" => intermediate_rep.machine_info.push(
                        <models::MachineInfo as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "status" => intermediate_rep.status.push(
                        <models::NodeStatus as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sandboxCount" => intermediate_rep.sandbox_count.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "metrics" => intermediate_rep.metrics.push(
                        <models::NodeMetrics as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    "cachedBuilds" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in NodeDetail"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "createSuccesses" => intermediate_rep.create_successes.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "createFails" => intermediate_rep.create_fails.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sandboxPausedCount" => intermediate_rep.sandbox_paused_count.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing NodeDetail".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(NodeDetail {
            cluster_id: intermediate_rep
                .cluster_id
                .into_iter()
                .next()
                .ok_or_else(|| "clusterID missing in NodeDetail".to_string())?,
            version: intermediate_rep
                .version
                .into_iter()
                .next()
                .ok_or_else(|| "version missing in NodeDetail".to_string())?,
            commit: intermediate_rep
                .commit
                .into_iter()
                .next()
                .ok_or_else(|| "commit missing in NodeDetail".to_string())?,
            id: intermediate_rep
                .id
                .into_iter()
                .next()
                .ok_or_else(|| "id missing in NodeDetail".to_string())?,
            service_instance_id: intermediate_rep
                .service_instance_id
                .into_iter()
                .next()
                .ok_or_else(|| "serviceInstanceID missing in NodeDetail".to_string())?,
            machine_info: intermediate_rep
                .machine_info
                .into_iter()
                .next()
                .ok_or_else(|| "machineInfo missing in NodeDetail".to_string())?,
            status: intermediate_rep
                .status
                .into_iter()
                .next()
                .ok_or_else(|| "status missing in NodeDetail".to_string())?,
            sandbox_count: intermediate_rep
                .sandbox_count
                .into_iter()
                .next()
                .ok_or_else(|| "sandboxCount missing in NodeDetail".to_string())?,
            metrics: intermediate_rep
                .metrics
                .into_iter()
                .next()
                .ok_or_else(|| "metrics missing in NodeDetail".to_string())?,
            cached_builds: intermediate_rep
                .cached_builds
                .into_iter()
                .next()
                .ok_or_else(|| "cachedBuilds missing in NodeDetail".to_string())?,
            create_successes: intermediate_rep
                .create_successes
                .into_iter()
                .next()
                .ok_or_else(|| "createSuccesses missing in NodeDetail".to_string())?,
            create_fails: intermediate_rep
                .create_fails
                .into_iter()
                .next()
                .ok_or_else(|| "createFails missing in NodeDetail".to_string())?,
            sandbox_paused_count: intermediate_rep
                .sandbox_paused_count
                .into_iter()
                .next()
                .ok_or_else(|| "sandboxPausedCount missing in NodeDetail".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<NodeDetail> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<NodeDetail>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<NodeDetail>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for NodeDetail - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<NodeDetail> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <NodeDetail as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into NodeDetail - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Durable admission closure and current node observations. This is not authorization to terminate a host or proof of physical cleanup.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct NodeDrainObservation {
    #[serde(rename = "nodeID")]
    #[validate(custom(function = "check_xss_string"))]
    pub node_id: String,

    #[serde(rename = "serviceInstanceID")]
    #[validate(custom(function = "check_xss_string"))]
    pub service_instance_id: String,

    #[serde(rename = "drainID")]
    #[validate(custom(function = "check_xss_string"))]
    pub drain_id: String,

    #[serde(rename = "admissionClosed")]
    pub admission_closed: bool,

    /// Complete local metadata inventory, including paused and cleanup-pending sandboxes. Not proof of physical cleanup.
    #[serde(rename = "sandboxIDs")]
    #[validate(custom(function = "check_xss_vec_string"))]
    pub sandbox_ids: Vec<String>,

    #[serde(rename = "inFlightStarts")]
    #[validate(range(min = 0u64))]
    pub in_flight_starts: u64,

    /// Cancellation-safe lifecycle tasks still executing, including cleanup and snapshot operations.
    #[serde(rename = "inFlightOperations")]
    #[validate(range(min = 0u64))]
    pub in_flight_operations: u64,

    /// Tasks that ended without a completion result in this service instance. A nonzero value requires recovery; process restart is not proof of physical cleanup.
    #[serde(rename = "interruptedOperations")]
    #[validate(range(min = 0u64))]
    pub interrupted_operations: u64,

    #[serde(rename = "sandboxCount")]
    #[validate(range(min = 0u64))]
    pub sandbox_count: u64,

    #[serde(rename = "pausedSandboxCount")]
    #[validate(range(min = 0u64))]
    pub paused_sandbox_count: u64,

    #[serde(rename = "sandboxStartingCount")]
    #[validate(range(min = 0u64))]
    pub sandbox_starting_count: u64,
}

impl NodeDrainObservation {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        node_id: String,
        service_instance_id: String,
        drain_id: String,
        admission_closed: bool,
        sandbox_ids: Vec<String>,
        in_flight_starts: u64,
        in_flight_operations: u64,
        interrupted_operations: u64,
        sandbox_count: u64,
        paused_sandbox_count: u64,
        sandbox_starting_count: u64,
    ) -> NodeDrainObservation {
        NodeDrainObservation {
            node_id,
            service_instance_id,
            drain_id,
            admission_closed,
            sandbox_ids,
            in_flight_starts,
            in_flight_operations,
            interrupted_operations,
            sandbox_count,
            paused_sandbox_count,
            sandbox_starting_count,
        }
    }
}

/// Converts the NodeDrainObservation value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for NodeDrainObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("nodeID".to_string()),
            Some(self.node_id.to_string()),
            Some("serviceInstanceID".to_string()),
            Some(self.service_instance_id.to_string()),
            Some("drainID".to_string()),
            Some(self.drain_id.to_string()),
            Some("admissionClosed".to_string()),
            Some(self.admission_closed.to_string()),
            Some("sandboxIDs".to_string()),
            Some(
                self.sandbox_ids
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            Some("inFlightStarts".to_string()),
            Some(self.in_flight_starts.to_string()),
            Some("inFlightOperations".to_string()),
            Some(self.in_flight_operations.to_string()),
            Some("interruptedOperations".to_string()),
            Some(self.interrupted_operations.to_string()),
            Some("sandboxCount".to_string()),
            Some(self.sandbox_count.to_string()),
            Some("pausedSandboxCount".to_string()),
            Some(self.paused_sandbox_count.to_string()),
            Some("sandboxStartingCount".to_string()),
            Some(self.sandbox_starting_count.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a NodeDrainObservation value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for NodeDrainObservation {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub node_id: Vec<String>,
            pub service_instance_id: Vec<String>,
            pub drain_id: Vec<String>,
            pub admission_closed: Vec<bool>,
            pub sandbox_ids: Vec<Vec<String>>,
            pub in_flight_starts: Vec<u64>,
            pub in_flight_operations: Vec<u64>,
            pub interrupted_operations: Vec<u64>,
            pub sandbox_count: Vec<u64>,
            pub paused_sandbox_count: Vec<u64>,
            pub sandbox_starting_count: Vec<u64>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing NodeDrainObservation".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "nodeID" => intermediate_rep.node_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "serviceInstanceID" => intermediate_rep.service_instance_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "drainID" => intermediate_rep.drain_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "admissionClosed" => intermediate_rep.admission_closed.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "sandboxIDs" => return std::result::Result::Err("Parsing a container in this style is not supported in NodeDrainObservation".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "inFlightStarts" => intermediate_rep.in_flight_starts.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "inFlightOperations" => intermediate_rep.in_flight_operations.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "interruptedOperations" => intermediate_rep.interrupted_operations.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "sandboxCount" => intermediate_rep.sandbox_count.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "pausedSandboxCount" => intermediate_rep.paused_sandbox_count.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "sandboxStartingCount" => intermediate_rep.sandbox_starting_count.push(<u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing NodeDrainObservation".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(NodeDrainObservation {
            node_id: intermediate_rep
                .node_id
                .into_iter()
                .next()
                .ok_or_else(|| "nodeID missing in NodeDrainObservation".to_string())?,
            service_instance_id: intermediate_rep
                .service_instance_id
                .into_iter()
                .next()
                .ok_or_else(|| "serviceInstanceID missing in NodeDrainObservation".to_string())?,
            drain_id: intermediate_rep
                .drain_id
                .into_iter()
                .next()
                .ok_or_else(|| "drainID missing in NodeDrainObservation".to_string())?,
            admission_closed: intermediate_rep
                .admission_closed
                .into_iter()
                .next()
                .ok_or_else(|| "admissionClosed missing in NodeDrainObservation".to_string())?,
            sandbox_ids: intermediate_rep
                .sandbox_ids
                .into_iter()
                .next()
                .ok_or_else(|| "sandboxIDs missing in NodeDrainObservation".to_string())?,
            in_flight_starts: intermediate_rep
                .in_flight_starts
                .into_iter()
                .next()
                .ok_or_else(|| "inFlightStarts missing in NodeDrainObservation".to_string())?,
            in_flight_operations: intermediate_rep
                .in_flight_operations
                .into_iter()
                .next()
                .ok_or_else(|| "inFlightOperations missing in NodeDrainObservation".to_string())?,
            interrupted_operations: intermediate_rep
                .interrupted_operations
                .into_iter()
                .next()
                .ok_or_else(|| {
                    "interruptedOperations missing in NodeDrainObservation".to_string()
                })?,
            sandbox_count: intermediate_rep
                .sandbox_count
                .into_iter()
                .next()
                .ok_or_else(|| "sandboxCount missing in NodeDrainObservation".to_string())?,
            paused_sandbox_count: intermediate_rep
                .paused_sandbox_count
                .into_iter()
                .next()
                .ok_or_else(|| "pausedSandboxCount missing in NodeDrainObservation".to_string())?,
            sandbox_starting_count: intermediate_rep
                .sandbox_starting_count
                .into_iter()
                .next()
                .ok_or_else(|| {
                    "sandboxStartingCount missing in NodeDrainObservation".to_string()
                })?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<NodeDrainObservation> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<NodeDrainObservation>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<NodeDrainObservation>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for NodeDrainObservation - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<NodeDrainObservation> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <NodeDrainObservation as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into NodeDrainObservation - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct NodeDrainRequest {
    #[serde(rename = "clusterID")]
    pub cluster_id: uuid::Uuid,

    #[serde(rename = "serviceInstanceID")]
    #[validate(length(min = 1, max = 128), custom(function = "check_xss_string"))]
    pub service_instance_id: String,

    #[serde(rename = "drainID")]
    #[validate(
            length(min = 1, max = 128),
            regex(path = *RE_NODEDRAINREQUEST_DRAIN_ID),
          custom(function = "check_xss_string"),
    )]
    pub drain_id: String,
}

lazy_static::lazy_static! {
    static ref RE_NODEDRAINREQUEST_DRAIN_ID: regex::Regex = regex::Regex::new("^[A-Za-z0-9_:-]+$").unwrap();
}

impl NodeDrainRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        cluster_id: uuid::Uuid,
        service_instance_id: String,
        drain_id: String,
    ) -> NodeDrainRequest {
        NodeDrainRequest {
            cluster_id,
            service_instance_id,
            drain_id,
        }
    }
}

/// Converts the NodeDrainRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for NodeDrainRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping clusterID in query parameter serialization
            Some("serviceInstanceID".to_string()),
            Some(self.service_instance_id.to_string()),
            Some("drainID".to_string()),
            Some(self.drain_id.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a NodeDrainRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for NodeDrainRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub cluster_id: Vec<uuid::Uuid>,
            pub service_instance_id: Vec<String>,
            pub drain_id: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing NodeDrainRequest".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "clusterID" => intermediate_rep.cluster_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "serviceInstanceID" => intermediate_rep.service_instance_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "drainID" => intermediate_rep.drain_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing NodeDrainRequest".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(NodeDrainRequest {
            cluster_id: intermediate_rep
                .cluster_id
                .into_iter()
                .next()
                .ok_or_else(|| "clusterID missing in NodeDrainRequest".to_string())?,
            service_instance_id: intermediate_rep
                .service_instance_id
                .into_iter()
                .next()
                .ok_or_else(|| "serviceInstanceID missing in NodeDrainRequest".to_string())?,
            drain_id: intermediate_rep
                .drain_id
                .into_iter()
                .next()
                .ok_or_else(|| "drainID missing in NodeDrainRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<NodeDrainRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<NodeDrainRequest>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<NodeDrainRequest>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for NodeDrainRequest - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<NodeDrainRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <NodeDrainRequest as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into NodeDrainRequest - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Exact runtime service incarnation selected before dispatch. A mismatch is rejected before launch side effects. This is not a funding or authorization grant.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct NodeLaunchTarget {
    #[serde(rename = "nodeID")]
    #[validate(length(min = 1), custom(function = "check_xss_string"))]
    pub node_id: String,

    #[serde(rename = "clusterID")]
    pub cluster_id: uuid::Uuid,

    #[serde(rename = "serviceInstanceID")]
    pub service_instance_id: uuid::Uuid,
}

impl NodeLaunchTarget {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        node_id: String,
        cluster_id: uuid::Uuid,
        service_instance_id: uuid::Uuid,
    ) -> NodeLaunchTarget {
        NodeLaunchTarget {
            node_id,
            cluster_id,
            service_instance_id,
        }
    }
}

/// Converts the NodeLaunchTarget value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for NodeLaunchTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("nodeID".to_string()),
            Some(self.node_id.to_string()),
            // Skipping clusterID in query parameter serialization

            // Skipping serviceInstanceID in query parameter serialization
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a NodeLaunchTarget value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for NodeLaunchTarget {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub node_id: Vec<String>,
            pub cluster_id: Vec<uuid::Uuid>,
            pub service_instance_id: Vec<uuid::Uuid>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing NodeLaunchTarget".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "nodeID" => intermediate_rep.node_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "clusterID" => intermediate_rep.cluster_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "serviceInstanceID" => intermediate_rep.service_instance_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing NodeLaunchTarget".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(NodeLaunchTarget {
            node_id: intermediate_rep
                .node_id
                .into_iter()
                .next()
                .ok_or_else(|| "nodeID missing in NodeLaunchTarget".to_string())?,
            cluster_id: intermediate_rep
                .cluster_id
                .into_iter()
                .next()
                .ok_or_else(|| "clusterID missing in NodeLaunchTarget".to_string())?,
            service_instance_id: intermediate_rep
                .service_instance_id
                .into_iter()
                .next()
                .ok_or_else(|| "serviceInstanceID missing in NodeLaunchTarget".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<NodeLaunchTarget> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<NodeLaunchTarget>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<NodeLaunchTarget>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for NodeLaunchTarget - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<NodeLaunchTarget> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <NodeLaunchTarget as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into NodeLaunchTarget - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Node metrics
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct NodeMetrics {
    /// Number of allocated CPU cores for the active running sandboxes (excludes paused)
    #[serde(rename = "allocatedCPU")]
    pub allocated_cpu: u32,

    /// Node CPU usage percentage
    #[serde(rename = "cpuPercent")]
    pub cpu_percent: u32,

    /// Total number of CPU cores on the node
    #[serde(rename = "cpuCount")]
    pub cpu_count: u32,

    /// Amount of allocated memory in bytes for the active running sandboxes (excludes paused)
    #[serde(rename = "allocatedMemoryBytes")]
    pub allocated_memory_bytes: u64,

    /// Node memory used in bytes
    #[serde(rename = "memoryUsedBytes")]
    pub memory_used_bytes: u64,

    /// Total node memory in bytes
    #[serde(rename = "memoryTotalBytes")]
    pub memory_total_bytes: u64,

    /// Detailed metrics for each disk/mount point
    #[serde(rename = "disks")]
    #[validate(nested)]
    pub disks: Vec<models::DiskMetrics>,

    /// Sum of CPU reservations across all sandboxes currently in the Paused state
    #[serde(rename = "pausedAllocatedCPU")]
    pub paused_allocated_cpu: u32,

    /// Sum of memory reservations (bytes) across all sandboxes currently in the Paused state
    #[serde(rename = "pausedAllocatedMemoryBytes")]
    pub paused_allocated_memory_bytes: u64,
}

impl NodeMetrics {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        allocated_cpu: u32,
        cpu_percent: u32,
        cpu_count: u32,
        allocated_memory_bytes: u64,
        memory_used_bytes: u64,
        memory_total_bytes: u64,
        disks: Vec<models::DiskMetrics>,
        paused_allocated_cpu: u32,
        paused_allocated_memory_bytes: u64,
    ) -> NodeMetrics {
        NodeMetrics {
            allocated_cpu,
            cpu_percent,
            cpu_count,
            allocated_memory_bytes,
            memory_used_bytes,
            memory_total_bytes,
            disks,
            paused_allocated_cpu,
            paused_allocated_memory_bytes,
        }
    }
}

/// Converts the NodeMetrics value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for NodeMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("allocatedCPU".to_string()),
            Some(self.allocated_cpu.to_string()),
            Some("cpuPercent".to_string()),
            Some(self.cpu_percent.to_string()),
            Some("cpuCount".to_string()),
            Some(self.cpu_count.to_string()),
            Some("allocatedMemoryBytes".to_string()),
            Some(self.allocated_memory_bytes.to_string()),
            Some("memoryUsedBytes".to_string()),
            Some(self.memory_used_bytes.to_string()),
            Some("memoryTotalBytes".to_string()),
            Some(self.memory_total_bytes.to_string()),
            // Skipping disks in query parameter serialization
            Some("pausedAllocatedCPU".to_string()),
            Some(self.paused_allocated_cpu.to_string()),
            Some("pausedAllocatedMemoryBytes".to_string()),
            Some(self.paused_allocated_memory_bytes.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a NodeMetrics value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for NodeMetrics {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub allocated_cpu: Vec<u32>,
            pub cpu_percent: Vec<u32>,
            pub cpu_count: Vec<u32>,
            pub allocated_memory_bytes: Vec<u64>,
            pub memory_used_bytes: Vec<u64>,
            pub memory_total_bytes: Vec<u64>,
            pub disks: Vec<Vec<models::DiskMetrics>>,
            pub paused_allocated_cpu: Vec<u32>,
            pub paused_allocated_memory_bytes: Vec<u64>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing NodeMetrics".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "allocatedCPU" => intermediate_rep.allocated_cpu.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "cpuPercent" => intermediate_rep.cpu_percent.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "cpuCount" => intermediate_rep.cpu_count.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "allocatedMemoryBytes" => intermediate_rep.allocated_memory_bytes.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "memoryUsedBytes" => intermediate_rep.memory_used_bytes.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "memoryTotalBytes" => intermediate_rep.memory_total_bytes.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "disks" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in NodeMetrics"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "pausedAllocatedCPU" => intermediate_rep.paused_allocated_cpu.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "pausedAllocatedMemoryBytes" => {
                        intermediate_rep.paused_allocated_memory_bytes.push(
                            <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                        )
                    }
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing NodeMetrics".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(NodeMetrics {
            allocated_cpu: intermediate_rep
                .allocated_cpu
                .into_iter()
                .next()
                .ok_or_else(|| "allocatedCPU missing in NodeMetrics".to_string())?,
            cpu_percent: intermediate_rep
                .cpu_percent
                .into_iter()
                .next()
                .ok_or_else(|| "cpuPercent missing in NodeMetrics".to_string())?,
            cpu_count: intermediate_rep
                .cpu_count
                .into_iter()
                .next()
                .ok_or_else(|| "cpuCount missing in NodeMetrics".to_string())?,
            allocated_memory_bytes: intermediate_rep
                .allocated_memory_bytes
                .into_iter()
                .next()
                .ok_or_else(|| "allocatedMemoryBytes missing in NodeMetrics".to_string())?,
            memory_used_bytes: intermediate_rep
                .memory_used_bytes
                .into_iter()
                .next()
                .ok_or_else(|| "memoryUsedBytes missing in NodeMetrics".to_string())?,
            memory_total_bytes: intermediate_rep
                .memory_total_bytes
                .into_iter()
                .next()
                .ok_or_else(|| "memoryTotalBytes missing in NodeMetrics".to_string())?,
            disks: intermediate_rep
                .disks
                .into_iter()
                .next()
                .ok_or_else(|| "disks missing in NodeMetrics".to_string())?,
            paused_allocated_cpu: intermediate_rep
                .paused_allocated_cpu
                .into_iter()
                .next()
                .ok_or_else(|| "pausedAllocatedCPU missing in NodeMetrics".to_string())?,
            paused_allocated_memory_bytes: intermediate_rep
                .paused_allocated_memory_bytes
                .into_iter()
                .next()
                .ok_or_else(|| "pausedAllocatedMemoryBytes missing in NodeMetrics".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<NodeMetrics> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<NodeMetrics>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<NodeMetrics>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for NodeMetrics - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<NodeMetrics> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <NodeMetrics as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into NodeMetrics - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Status of the node. - draining: the node is bound to be shut down. It will not accept new sandboxes and will stop once all existing sandboxes are done.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum NodeStatus {
    #[serde(rename = "ready")]
    NodeStatusReady,
    #[serde(rename = "draining")]
    NodeStatusDraining,
    #[serde(rename = "connecting")]
    NodeStatusConnecting,
    #[serde(rename = "unhealthy")]
    NodeStatusUnhealthy,
}

impl validator::Validate for NodeStatus {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            NodeStatus::NodeStatusReady => write!(f, "ready"),
            NodeStatus::NodeStatusDraining => write!(f, "draining"),
            NodeStatus::NodeStatusConnecting => write!(f, "connecting"),
            NodeStatus::NodeStatusUnhealthy => write!(f, "unhealthy"),
        }
    }
}

impl std::str::FromStr for NodeStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ready" => std::result::Result::Ok(NodeStatus::NodeStatusReady),
            "draining" => std::result::Result::Ok(NodeStatus::NodeStatusDraining),
            "connecting" => std::result::Result::Ok(NodeStatus::NodeStatusConnecting),
            "unhealthy" => std::result::Result::Ok(NodeStatus::NodeStatusUnhealthy),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct PausedSandboxSnapshotRequest {
    /// Stable publication identity used for retries. The same snapshot ID must not be reused for another source activation.
    #[serde(rename = "snapshotId")]
    pub snapshot_id: uuid::Uuid,

    /// Nonzero activation whose retained paused state is being published. This operation never starts guest execution or obtains new funding.
    #[serde(rename = "expectedActivationID")]
    pub expected_activation_id: uuid::Uuid,

    /// Optional snapshot template name.
    #[serde(rename = "name")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl PausedSandboxSnapshotRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        snapshot_id: uuid::Uuid,
        expected_activation_id: uuid::Uuid,
    ) -> PausedSandboxSnapshotRequest {
        PausedSandboxSnapshotRequest {
            snapshot_id,
            expected_activation_id,
            name: None,
        }
    }
}

/// Converts the PausedSandboxSnapshotRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for PausedSandboxSnapshotRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping snapshotId in query parameter serialization

            // Skipping expectedActivationID in query parameter serialization
            self.name
                .as_ref()
                .map(|name| ["name".to_string(), name.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a PausedSandboxSnapshotRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for PausedSandboxSnapshotRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub snapshot_id: Vec<uuid::Uuid>,
            pub expected_activation_id: Vec<uuid::Uuid>,
            pub name: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing PausedSandboxSnapshotRequest".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "snapshotId" => intermediate_rep.snapshot_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "expectedActivationID" => intermediate_rep.expected_activation_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "name" => intermediate_rep.name.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing PausedSandboxSnapshotRequest".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(PausedSandboxSnapshotRequest {
            snapshot_id: intermediate_rep
                .snapshot_id
                .into_iter()
                .next()
                .ok_or_else(|| "snapshotId missing in PausedSandboxSnapshotRequest".to_string())?,
            expected_activation_id: intermediate_rep
                .expected_activation_id
                .into_iter()
                .next()
                .ok_or_else(|| {
                    "expectedActivationID missing in PausedSandboxSnapshotRequest".to_string()
                })?,
            name: intermediate_rep.name.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<PausedSandboxSnapshotRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<PausedSandboxSnapshotRequest>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<PausedSandboxSnapshotRequest>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for PausedSandboxSnapshotRequest - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<PausedSandboxSnapshotRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <PausedSandboxSnapshotRequest as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into PausedSandboxSnapshotRequest - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct ResumedSandbox {
    /// Exact nonzero activation of the retained paused runtime. Fenced resume rejects running or transitional instances instead of updating their timeout. After an uncertain response, observe the target executionLease activation before deciding whether to retry. Optional only for legacy SDK and SQL lifecycle callers until the Kubernetes execution handoff retires those callers.
    #[serde(rename = "expectedSourceActivationID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_source_activation_id: Option<uuid::Uuid>,

    #[serde(rename = "targetNodeInstance")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_node_instance: Option<models::NodeLaunchTarget>,

    #[serde(rename = "executionLease")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_lease: Option<models::ExecutionLease>,

    /// Time to live for the sandbox in seconds.
    #[serde(rename = "timeout")]
    #[validate(range(min = 0u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
}

impl ResumedSandbox {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> ResumedSandbox {
        ResumedSandbox {
            expected_source_activation_id: None,
            target_node_instance: None,
            execution_lease: None,
            timeout: Some(15),
        }
    }
}

/// Converts the ResumedSandbox value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for ResumedSandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping expectedSourceActivationID in query parameter serialization

            // Skipping targetNodeInstance in query parameter serialization

            // Skipping executionLease in query parameter serialization
            self.timeout
                .as_ref()
                .map(|timeout| ["timeout".to_string(), timeout.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a ResumedSandbox value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for ResumedSandbox {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub expected_source_activation_id: Vec<uuid::Uuid>,
            pub target_node_instance: Vec<models::NodeLaunchTarget>,
            pub execution_lease: Vec<models::ExecutionLease>,
            pub timeout: Vec<u32>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing ResumedSandbox".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "expectedSourceActivationID" => {
                        intermediate_rep.expected_source_activation_id.push(
                            <uuid::Uuid as std::str::FromStr>::from_str(val)
                                .map_err(|x| x.to_string())?,
                        )
                    }
                    #[allow(clippy::redundant_clone)]
                    "targetNodeInstance" => intermediate_rep.target_node_instance.push(
                        <models::NodeLaunchTarget as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "executionLease" => intermediate_rep.execution_lease.push(
                        <models::ExecutionLease as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "timeout" => intermediate_rep.timeout.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing ResumedSandbox".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(ResumedSandbox {
            expected_source_activation_id: intermediate_rep
                .expected_source_activation_id
                .into_iter()
                .next(),
            target_node_instance: intermediate_rep.target_node_instance.into_iter().next(),
            execution_lease: intermediate_rep.execution_lease.into_iter().next(),
            timeout: intermediate_rep.timeout.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<ResumedSandbox> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<ResumedSandbox>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<ResumedSandbox>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for ResumedSandbox - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<ResumedSandbox> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <ResumedSandbox as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into ResumedSandbox - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Sandbox {
    #[serde(rename = "executionLease")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_lease: Option<models::ExecutionLease>,

    /// Identifier of the template from which is the sandbox created
    #[serde(rename = "templateID")]
    #[validate(custom(function = "check_xss_string"))]
    pub template_id: String,

    /// Identifier of the sandbox
    #[serde(rename = "sandboxID")]
    #[validate(custom(function = "check_xss_string"))]
    pub sandbox_id: String,

    /// Alias of the template
    #[serde(rename = "alias")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,

    /// Identifier of the client
    #[serde(rename = "clientID")]
    #[validate(custom(function = "check_xss_string"))]
    pub client_id: String,

    /// Version of the envd running in the sandbox
    #[serde(rename = "envdVersion")]
    #[validate(custom(function = "check_xss_string"))]
    pub envd_version: String,

    /// Immutable start boundary of the current running runtime activation. Changes after a successful pause and resume.
    #[serde(rename = "runtimeStartedAt")]
    pub runtime_started_at: chrono::DateTime<chrono::Utc>,

    /// Access token used for envd communication
    #[serde(rename = "envdAccessToken")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envd_access_token: Option<String>,

    /// Token required for accessing sandbox via proxy.
    #[serde(rename = "trafficAccessToken")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traffic_access_token: Option<Nullable<String>>,

    /// Base domain where the sandbox traffic is accessible
    #[serde(rename = "domain")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<Nullable<String>>,
}

impl Sandbox {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        template_id: String,
        sandbox_id: String,
        client_id: String,
        envd_version: String,
        runtime_started_at: chrono::DateTime<chrono::Utc>,
    ) -> Sandbox {
        Sandbox {
            execution_lease: None,
            template_id,
            sandbox_id,
            alias: None,
            client_id,
            envd_version,
            runtime_started_at,
            envd_access_token: None,
            traffic_access_token: None,
            domain: None,
        }
    }
}

/// Converts the Sandbox value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Sandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping executionLease in query parameter serialization
            Some("templateID".to_string()),
            Some(self.template_id.to_string()),
            Some("sandboxID".to_string()),
            Some(self.sandbox_id.to_string()),
            self.alias
                .as_ref()
                .map(|alias| ["alias".to_string(), alias.to_string()].join(",")),
            Some("clientID".to_string()),
            Some(self.client_id.to_string()),
            Some("envdVersion".to_string()),
            Some(self.envd_version.to_string()),
            // Skipping runtimeStartedAt in query parameter serialization
            self.envd_access_token.as_ref().map(|envd_access_token| {
                ["envdAccessToken".to_string(), envd_access_token.to_string()].join(",")
            }),
            self.traffic_access_token
                .as_ref()
                .map(|traffic_access_token| {
                    [
                        "trafficAccessToken".to_string(),
                        traffic_access_token
                            .as_ref()
                            .map_or("null".to_string(), |x| x.to_string()),
                    ]
                    .join(",")
                }),
            self.domain.as_ref().map(|domain| {
                [
                    "domain".to_string(),
                    domain
                        .as_ref()
                        .map_or("null".to_string(), |x| x.to_string()),
                ]
                .join(",")
            }),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Sandbox value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Sandbox {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub execution_lease: Vec<models::ExecutionLease>,
            pub template_id: Vec<String>,
            pub sandbox_id: Vec<String>,
            pub alias: Vec<String>,
            pub client_id: Vec<String>,
            pub envd_version: Vec<String>,
            pub runtime_started_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub envd_access_token: Vec<String>,
            pub traffic_access_token: Vec<String>,
            pub domain: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing Sandbox".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "executionLease" => intermediate_rep.execution_lease.push(
                        <models::ExecutionLease as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "templateID" => intermediate_rep.template_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sandboxID" => intermediate_rep.sandbox_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "alias" => intermediate_rep.alias.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "clientID" => intermediate_rep.client_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "envdVersion" => intermediate_rep.envd_version.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "runtimeStartedAt" => intermediate_rep.runtime_started_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "envdAccessToken" => intermediate_rep.envd_access_token.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "trafficAccessToken" => {
                        return std::result::Result::Err(
                            "Parsing a nullable type in this style is not supported in Sandbox"
                                .to_string(),
                        );
                    }
                    "domain" => {
                        return std::result::Result::Err(
                            "Parsing a nullable type in this style is not supported in Sandbox"
                                .to_string(),
                        );
                    }
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing Sandbox".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Sandbox {
            execution_lease: intermediate_rep.execution_lease.into_iter().next(),
            template_id: intermediate_rep
                .template_id
                .into_iter()
                .next()
                .ok_or_else(|| "templateID missing in Sandbox".to_string())?,
            sandbox_id: intermediate_rep
                .sandbox_id
                .into_iter()
                .next()
                .ok_or_else(|| "sandboxID missing in Sandbox".to_string())?,
            alias: intermediate_rep.alias.into_iter().next(),
            client_id: intermediate_rep
                .client_id
                .into_iter()
                .next()
                .ok_or_else(|| "clientID missing in Sandbox".to_string())?,
            envd_version: intermediate_rep
                .envd_version
                .into_iter()
                .next()
                .ok_or_else(|| "envdVersion missing in Sandbox".to_string())?,
            runtime_started_at: intermediate_rep
                .runtime_started_at
                .into_iter()
                .next()
                .ok_or_else(|| "runtimeStartedAt missing in Sandbox".to_string())?,
            envd_access_token: intermediate_rep.envd_access_token.into_iter().next(),
            traffic_access_token: std::result::Result::Err(
                "Nullable types not supported in Sandbox".to_string(),
            )?,
            domain: std::result::Result::Err(
                "Nullable types not supported in Sandbox".to_string(),
            )?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Sandbox> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Sandbox>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<Sandbox>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for Sandbox - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Sandbox> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <Sandbox as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into Sandbox - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Auto-resume configuration for paused sandboxes.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxAutoResumeConfig {
    /// Auto-resume enabled flag for paused sandboxes. Default false.
    #[serde(rename = "enabled")]
    pub enabled: bool,
}

impl SandboxAutoResumeConfig {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> SandboxAutoResumeConfig {
        SandboxAutoResumeConfig { enabled: false }
    }
}

/// Converts the SandboxAutoResumeConfig value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SandboxAutoResumeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> =
            vec![Some("enabled".to_string()), Some(self.enabled.to_string())];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SandboxAutoResumeConfig value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SandboxAutoResumeConfig {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub enabled: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SandboxAutoResumeConfig".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "enabled" => intermediate_rep.enabled.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SandboxAutoResumeConfig".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SandboxAutoResumeConfig {
            enabled: intermediate_rep
                .enabled
                .into_iter()
                .next()
                .ok_or_else(|| "enabled missing in SandboxAutoResumeConfig".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SandboxAutoResumeConfig> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SandboxAutoResumeConfig>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SandboxAutoResumeConfig>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SandboxAutoResumeConfig - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SandboxAutoResumeConfig> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SandboxAutoResumeConfig as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SandboxAutoResumeConfig - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Auto-resume enabled flag for paused sandboxes. Default false.
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxAutoResumeEnabled(pub bool);

impl validator::Validate for SandboxAutoResumeEnabled {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::convert::From<bool> for SandboxAutoResumeEnabled {
    fn from(x: bool) -> Self {
        SandboxAutoResumeEnabled(x)
    }
}

impl std::convert::From<SandboxAutoResumeEnabled> for bool {
    fn from(x: SandboxAutoResumeEnabled) -> Self {
        x.0
    }
}

impl std::ops::Deref for SandboxAutoResumeEnabled {
    type Target = bool;
    fn deref(&self) -> &bool {
        &self.0
    }
}

impl std::ops::DerefMut for SandboxAutoResumeEnabled {
    fn deref_mut(&mut self) -> &mut bool {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxDetail {
    #[serde(rename = "executionLease")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_lease: Option<models::ExecutionLease>,

    /// Identifier of the template from which is the sandbox created
    #[serde(rename = "templateID")]
    #[validate(custom(function = "check_xss_string"))]
    pub template_id: String,

    /// Alias of the template
    #[serde(rename = "alias")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,

    /// Identifier of the sandbox
    #[serde(rename = "sandboxID")]
    #[validate(custom(function = "check_xss_string"))]
    pub sandbox_id: String,

    /// Identifier of the client
    #[serde(rename = "clientID")]
    #[validate(custom(function = "check_xss_string"))]
    pub client_id: String,

    /// Time when the sandbox was started
    #[serde(rename = "startedAt")]
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// Immutable start boundary of the current running runtime activation. Changes after a successful pause and resume.
    #[serde(rename = "runtimeStartedAt")]
    pub runtime_started_at: chrono::DateTime<chrono::Utc>,

    /// Time when the sandbox will expire
    #[serde(rename = "endAt")]
    pub end_at: chrono::DateTime<chrono::Utc>,

    /// Version of the envd running in the sandbox
    #[serde(rename = "envdVersion")]
    #[validate(custom(function = "check_xss_string"))]
    pub envd_version: String,

    /// Access token used for envd communication
    #[serde(rename = "envdAccessToken")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envd_access_token: Option<String>,

    /// Whether internet access was explicitly enabled or disabled for the sandbox. Null means it was not explicitly set.
    #[serde(rename = "allowInternetAccess")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_internet_access: Option<Nullable<bool>>,

    /// Base domain where the sandbox traffic is accessible
    #[serde(rename = "domain")]
    #[serde(deserialize_with = "deserialize_optional_nullable")]
    #[serde(default = "default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<Nullable<String>>,

    /// CPU cores for the sandbox
    #[serde(rename = "cpuCount")]
    #[validate(range(min = 1u32))]
    pub cpu_count: u32,

    /// Memory for the sandbox in MiB
    #[serde(rename = "memoryMB")]
    #[validate(range(min = 128u32))]
    pub memory_mb: u32,

    /// Disk size for the sandbox in MiB
    #[serde(rename = "diskSizeMB")]
    #[validate(range(min = 0u32))]
    pub disk_size_mb: u32,

    #[serde(rename = "metadata")]
    #[validate(custom(function = "check_xss_map_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,

    #[serde(rename = "state")]
    #[validate(nested)]
    pub state: models::SandboxState,

    #[serde(rename = "network")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<models::SandboxNetworkConfig>,

    #[serde(rename = "lifecycle")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<models::SandboxLifecycle>,
}

impl SandboxDetail {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        template_id: String,
        sandbox_id: String,
        client_id: String,
        started_at: chrono::DateTime<chrono::Utc>,
        runtime_started_at: chrono::DateTime<chrono::Utc>,
        end_at: chrono::DateTime<chrono::Utc>,
        envd_version: String,
        cpu_count: u32,
        memory_mb: u32,
        disk_size_mb: u32,
        state: models::SandboxState,
    ) -> SandboxDetail {
        SandboxDetail {
            execution_lease: None,
            template_id,
            alias: None,
            sandbox_id,
            client_id,
            started_at,
            runtime_started_at,
            end_at,
            envd_version,
            envd_access_token: None,
            allow_internet_access: None,
            domain: None,
            cpu_count,
            memory_mb,
            disk_size_mb,
            metadata: None,
            state,
            network: None,
            lifecycle: None,
        }
    }
}

/// Converts the SandboxDetail value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SandboxDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping executionLease in query parameter serialization
            Some("templateID".to_string()),
            Some(self.template_id.to_string()),
            self.alias
                .as_ref()
                .map(|alias| ["alias".to_string(), alias.to_string()].join(",")),
            Some("sandboxID".to_string()),
            Some(self.sandbox_id.to_string()),
            Some("clientID".to_string()),
            Some(self.client_id.to_string()),
            // Skipping startedAt in query parameter serialization

            // Skipping runtimeStartedAt in query parameter serialization

            // Skipping endAt in query parameter serialization
            Some("envdVersion".to_string()),
            Some(self.envd_version.to_string()),
            self.envd_access_token.as_ref().map(|envd_access_token| {
                ["envdAccessToken".to_string(), envd_access_token.to_string()].join(",")
            }),
            self.allow_internet_access
                .as_ref()
                .map(|allow_internet_access| {
                    [
                        "allowInternetAccess".to_string(),
                        allow_internet_access
                            .as_ref()
                            .map_or("null".to_string(), |x| x.to_string()),
                    ]
                    .join(",")
                }),
            self.domain.as_ref().map(|domain| {
                [
                    "domain".to_string(),
                    domain
                        .as_ref()
                        .map_or("null".to_string(), |x| x.to_string()),
                ]
                .join(",")
            }),
            Some("cpuCount".to_string()),
            Some(self.cpu_count.to_string()),
            Some("memoryMB".to_string()),
            Some(self.memory_mb.to_string()),
            Some("diskSizeMB".to_string()),
            Some(self.disk_size_mb.to_string()),
            // Skipping metadata in query parameter serialization

            // Skipping state in query parameter serialization

            // Skipping network in query parameter serialization

            // Skipping lifecycle in query parameter serialization
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SandboxDetail value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SandboxDetail {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub execution_lease: Vec<models::ExecutionLease>,
            pub template_id: Vec<String>,
            pub alias: Vec<String>,
            pub sandbox_id: Vec<String>,
            pub client_id: Vec<String>,
            pub started_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub runtime_started_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub end_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub envd_version: Vec<String>,
            pub envd_access_token: Vec<String>,
            pub allow_internet_access: Vec<bool>,
            pub domain: Vec<String>,
            pub cpu_count: Vec<u32>,
            pub memory_mb: Vec<u32>,
            pub disk_size_mb: Vec<u32>,
            pub metadata: Vec<std::collections::HashMap<String, String>>,
            pub state: Vec<models::SandboxState>,
            pub network: Vec<models::SandboxNetworkConfig>,
            pub lifecycle: Vec<models::SandboxLifecycle>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SandboxDetail".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "executionLease" => intermediate_rep.execution_lease.push(
                        <models::ExecutionLease as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "templateID" => intermediate_rep.template_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "alias" => intermediate_rep.alias.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sandboxID" => intermediate_rep.sandbox_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "clientID" => intermediate_rep.client_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "startedAt" => intermediate_rep.started_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "runtimeStartedAt" => intermediate_rep.runtime_started_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "endAt" => intermediate_rep.end_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "envdVersion" => intermediate_rep.envd_version.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "envdAccessToken" => intermediate_rep.envd_access_token.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "allowInternetAccess" => return std::result::Result::Err(
                        "Parsing a nullable type in this style is not supported in SandboxDetail"
                            .to_string(),
                    ),
                    "domain" => return std::result::Result::Err(
                        "Parsing a nullable type in this style is not supported in SandboxDetail"
                            .to_string(),
                    ),
                    #[allow(clippy::redundant_clone)]
                    "cpuCount" => intermediate_rep.cpu_count.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "memoryMB" => intermediate_rep.memory_mb.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "diskSizeMB" => intermediate_rep.disk_size_mb.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "metadata" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in SandboxDetail"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "state" => intermediate_rep.state.push(
                        <models::SandboxState as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "network" => intermediate_rep.network.push(
                        <models::SandboxNetworkConfig as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "lifecycle" => intermediate_rep.lifecycle.push(
                        <models::SandboxLifecycle as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SandboxDetail".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SandboxDetail {
            execution_lease: intermediate_rep.execution_lease.into_iter().next(),
            template_id: intermediate_rep
                .template_id
                .into_iter()
                .next()
                .ok_or_else(|| "templateID missing in SandboxDetail".to_string())?,
            alias: intermediate_rep.alias.into_iter().next(),
            sandbox_id: intermediate_rep
                .sandbox_id
                .into_iter()
                .next()
                .ok_or_else(|| "sandboxID missing in SandboxDetail".to_string())?,
            client_id: intermediate_rep
                .client_id
                .into_iter()
                .next()
                .ok_or_else(|| "clientID missing in SandboxDetail".to_string())?,
            started_at: intermediate_rep
                .started_at
                .into_iter()
                .next()
                .ok_or_else(|| "startedAt missing in SandboxDetail".to_string())?,
            runtime_started_at: intermediate_rep
                .runtime_started_at
                .into_iter()
                .next()
                .ok_or_else(|| "runtimeStartedAt missing in SandboxDetail".to_string())?,
            end_at: intermediate_rep
                .end_at
                .into_iter()
                .next()
                .ok_or_else(|| "endAt missing in SandboxDetail".to_string())?,
            envd_version: intermediate_rep
                .envd_version
                .into_iter()
                .next()
                .ok_or_else(|| "envdVersion missing in SandboxDetail".to_string())?,
            envd_access_token: intermediate_rep.envd_access_token.into_iter().next(),
            allow_internet_access: std::result::Result::Err(
                "Nullable types not supported in SandboxDetail".to_string(),
            )?,
            domain: std::result::Result::Err(
                "Nullable types not supported in SandboxDetail".to_string(),
            )?,
            cpu_count: intermediate_rep
                .cpu_count
                .into_iter()
                .next()
                .ok_or_else(|| "cpuCount missing in SandboxDetail".to_string())?,
            memory_mb: intermediate_rep
                .memory_mb
                .into_iter()
                .next()
                .ok_or_else(|| "memoryMB missing in SandboxDetail".to_string())?,
            disk_size_mb: intermediate_rep
                .disk_size_mb
                .into_iter()
                .next()
                .ok_or_else(|| "diskSizeMB missing in SandboxDetail".to_string())?,
            metadata: intermediate_rep.metadata.into_iter().next(),
            state: intermediate_rep
                .state
                .into_iter()
                .next()
                .ok_or_else(|| "state missing in SandboxDetail".to_string())?,
            network: intermediate_rep.network.into_iter().next(),
            lifecycle: intermediate_rep.lifecycle.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SandboxDetail> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SandboxDetail>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SandboxDetail>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SandboxDetail - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SandboxDetail> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SandboxDetail as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SandboxDetail - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxForkRequest {
    #[serde(rename = "targetNodeInstance")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_node_instance: Option<models::NodeLaunchTarget>,

    #[serde(rename = "executionLease")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_lease: Option<models::ExecutionLease>,

    /// Time to live for the new forked sandboxes in seconds. When omitted, each fork inherits the source sandbox timeout.
    #[serde(rename = "timeout")]
    #[validate(range(min = 0u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,

    /// Number of forked sandboxes to create. All forks boot from the same snapshot, so the snapshot is captured once regardless of count. Each fork succeeds or fails independently; the outcome of each is reported in its entry of the response list.
    #[serde(rename = "count")]
    #[validate(range(min = 1u32, max = 100u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

impl SandboxForkRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> SandboxForkRequest {
        SandboxForkRequest {
            target_node_instance: None,
            execution_lease: None,
            timeout: None,
            count: Some(1),
        }
    }
}

/// Converts the SandboxForkRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SandboxForkRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping targetNodeInstance in query parameter serialization

            // Skipping executionLease in query parameter serialization
            self.timeout
                .as_ref()
                .map(|timeout| ["timeout".to_string(), timeout.to_string()].join(",")),
            self.count
                .as_ref()
                .map(|count| ["count".to_string(), count.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SandboxForkRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SandboxForkRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub target_node_instance: Vec<models::NodeLaunchTarget>,
            pub execution_lease: Vec<models::ExecutionLease>,
            pub timeout: Vec<u32>,
            pub count: Vec<u32>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SandboxForkRequest".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "targetNodeInstance" => intermediate_rep.target_node_instance.push(
                        <models::NodeLaunchTarget as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "executionLease" => intermediate_rep.execution_lease.push(
                        <models::ExecutionLease as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "timeout" => intermediate_rep.timeout.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "count" => intermediate_rep.count.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SandboxForkRequest".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SandboxForkRequest {
            target_node_instance: intermediate_rep.target_node_instance.into_iter().next(),
            execution_lease: intermediate_rep.execution_lease.into_iter().next(),
            timeout: intermediate_rep.timeout.into_iter().next(),
            count: intermediate_rep.count.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SandboxForkRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SandboxForkRequest>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SandboxForkRequest>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SandboxForkRequest - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SandboxForkRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SandboxForkRequest as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SandboxForkRequest - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Result of one requested fork. Exactly one of sandbox or error is set: sandbox when the fork started successfully, error when it failed to start.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxForkResult {
    #[serde(rename = "sandbox")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<models::Sandbox>,

    #[serde(rename = "error")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<models::Error>,
}

impl SandboxForkResult {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> SandboxForkResult {
        SandboxForkResult {
            sandbox: None,
            error: None,
        }
    }
}

/// Converts the SandboxForkResult value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SandboxForkResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping sandbox in query parameter serialization

            // Skipping error in query parameter serialization

        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SandboxForkResult value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SandboxForkResult {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub sandbox: Vec<models::Sandbox>,
            pub error: Vec<models::Error>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SandboxForkResult".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "sandbox" => intermediate_rep.sandbox.push(
                        <models::Sandbox as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "error" => intermediate_rep.error.push(
                        <models::Error as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SandboxForkResult".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SandboxForkResult {
            sandbox: intermediate_rep.sandbox.into_iter().next(),
            error: intermediate_rep.error.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SandboxForkResult> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SandboxForkResult>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SandboxForkResult>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SandboxForkResult - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SandboxForkResult> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SandboxForkResult as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SandboxForkResult - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Sandbox lifecycle policy returned by sandbox info.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxLifecycle {
    /// Whether the sandbox can auto-resume.
    #[serde(rename = "autoResume")]
    pub auto_resume: bool,

    #[serde(rename = "onTimeout")]
    #[validate(nested)]
    pub on_timeout: models::SandboxOnTimeout,
}

impl SandboxLifecycle {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(auto_resume: bool, on_timeout: models::SandboxOnTimeout) -> SandboxLifecycle {
        SandboxLifecycle {
            auto_resume,
            on_timeout,
        }
    }
}

/// Converts the SandboxLifecycle value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SandboxLifecycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("autoResume".to_string()),
            Some(self.auto_resume.to_string()),
            // Skipping onTimeout in query parameter serialization
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SandboxLifecycle value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SandboxLifecycle {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub auto_resume: Vec<bool>,
            pub on_timeout: Vec<models::SandboxOnTimeout>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SandboxLifecycle".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "autoResume" => intermediate_rep.auto_resume.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "onTimeout" => intermediate_rep.on_timeout.push(
                        <models::SandboxOnTimeout as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SandboxLifecycle".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SandboxLifecycle {
            auto_resume: intermediate_rep
                .auto_resume
                .into_iter()
                .next()
                .ok_or_else(|| "autoResume missing in SandboxLifecycle".to_string())?,
            on_timeout: intermediate_rep
                .on_timeout
                .into_iter()
                .next()
                .ok_or_else(|| "onTimeout missing in SandboxLifecycle".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SandboxLifecycle> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SandboxLifecycle>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SandboxLifecycle>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SandboxLifecycle - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SandboxLifecycle> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SandboxLifecycle as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SandboxLifecycle - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxNetworkConfig {
    /// Specify if the sandbox URLs should be accessible only with authentication.
    #[serde(rename = "allowPublicTraffic")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_public_traffic: Option<bool>,

    /// List of allowed destinations for egress traffic. Each entry can be a CIDR block (e.g. \"8.8.8.8/32\"), a bare IP address (e.g. \"8.8.8.8\"), or a domain name (e.g. \"example.com\", \"*.example.com\"). Allowed entries always take precedence over denied entries.
    #[serde(rename = "allowOut")]
    #[validate(custom(function = "check_xss_vec_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_out: Option<Vec<String>>,

    /// List of denied CIDR blocks or IP addresses for egress traffic. Domain names are not supported for deny rules.
    #[serde(rename = "denyOut")]
    #[validate(custom(function = "check_xss_vec_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_out: Option<Vec<String>>,

    /// Specify host mask which will be used for all sandbox requests
    #[serde(rename = "maskRequestHost")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_request_host: Option<String>,
}

impl SandboxNetworkConfig {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> SandboxNetworkConfig {
        SandboxNetworkConfig {
            allow_public_traffic: Some(true),
            allow_out: None,
            deny_out: None,
            mask_request_host: None,
        }
    }
}

/// Converts the SandboxNetworkConfig value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SandboxNetworkConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            self.allow_public_traffic
                .as_ref()
                .map(|allow_public_traffic| {
                    [
                        "allowPublicTraffic".to_string(),
                        allow_public_traffic.to_string(),
                    ]
                    .join(",")
                }),
            self.allow_out.as_ref().map(|allow_out| {
                [
                    "allowOut".to_string(),
                    allow_out
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                ]
                .join(",")
            }),
            self.deny_out.as_ref().map(|deny_out| {
                [
                    "denyOut".to_string(),
                    deny_out
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                ]
                .join(",")
            }),
            self.mask_request_host.as_ref().map(|mask_request_host| {
                ["maskRequestHost".to_string(), mask_request_host.to_string()].join(",")
            }),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SandboxNetworkConfig value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SandboxNetworkConfig {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub allow_public_traffic: Vec<bool>,
            pub allow_out: Vec<Vec<String>>,
            pub deny_out: Vec<Vec<String>>,
            pub mask_request_host: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SandboxNetworkConfig".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "allowPublicTraffic" => intermediate_rep.allow_public_traffic.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "allowOut" => return std::result::Result::Err("Parsing a container in this style is not supported in SandboxNetworkConfig".to_string()),
                    "denyOut" => return std::result::Result::Err("Parsing a container in this style is not supported in SandboxNetworkConfig".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "maskRequestHost" => intermediate_rep.mask_request_host.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing SandboxNetworkConfig".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SandboxNetworkConfig {
            allow_public_traffic: intermediate_rep.allow_public_traffic.into_iter().next(),
            allow_out: intermediate_rep.allow_out.into_iter().next(),
            deny_out: intermediate_rep.deny_out.into_iter().next(),
            mask_request_host: intermediate_rep.mask_request_host.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SandboxNetworkConfig> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SandboxNetworkConfig>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SandboxNetworkConfig>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SandboxNetworkConfig - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SandboxNetworkConfig> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SandboxNetworkConfig as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SandboxNetworkConfig - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Network configuration update for a running sandbox. Replaces the current egress rules with the provided configuration. Omitting a field clears it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxNetworkUpdateConfig {
    /// List of allowed destinations for egress traffic. Each entry can be a CIDR block (e.g. \"8.8.8.8/32\"), a bare IP address (e.g. \"8.8.8.8\"), or a domain name (e.g. \"example.com\", \"*.example.com\"). Allowed entries always take precedence over denied entries.
    #[serde(rename = "allowOut")]
    #[validate(custom(function = "check_xss_vec_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_out: Option<Vec<String>>,

    /// List of denied CIDR blocks or IP addresses for egress traffic. Domain names are not supported for deny rules.
    #[serde(rename = "denyOut")]
    #[validate(custom(function = "check_xss_vec_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_out: Option<Vec<String>>,

    /// Allow sandbox to access the internet. When set to false, it behaves the same as specifying denyOut to 0.0.0.0/0 in the network config.
    #[serde(rename = "allow_internet_access")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_internet_access: Option<bool>,
}

impl SandboxNetworkUpdateConfig {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> SandboxNetworkUpdateConfig {
        SandboxNetworkUpdateConfig {
            allow_out: None,
            deny_out: None,
            allow_internet_access: None,
        }
    }
}

/// Converts the SandboxNetworkUpdateConfig value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SandboxNetworkUpdateConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            self.allow_out.as_ref().map(|allow_out| {
                [
                    "allowOut".to_string(),
                    allow_out
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                ]
                .join(",")
            }),
            self.deny_out.as_ref().map(|deny_out| {
                [
                    "denyOut".to_string(),
                    deny_out
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                ]
                .join(",")
            }),
            self.allow_internet_access
                .as_ref()
                .map(|allow_internet_access| {
                    [
                        "allow_internet_access".to_string(),
                        allow_internet_access.to_string(),
                    ]
                    .join(",")
                }),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SandboxNetworkUpdateConfig value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SandboxNetworkUpdateConfig {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub allow_out: Vec<Vec<String>>,
            pub deny_out: Vec<Vec<String>>,
            pub allow_internet_access: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SandboxNetworkUpdateConfig".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "allowOut" => return std::result::Result::Err("Parsing a container in this style is not supported in SandboxNetworkUpdateConfig".to_string()),
                    "denyOut" => return std::result::Result::Err("Parsing a container in this style is not supported in SandboxNetworkUpdateConfig".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "allow_internet_access" => intermediate_rep.allow_internet_access.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing SandboxNetworkUpdateConfig".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SandboxNetworkUpdateConfig {
            allow_out: intermediate_rep.allow_out.into_iter().next(),
            deny_out: intermediate_rep.deny_out.into_iter().next(),
            allow_internet_access: intermediate_rep.allow_internet_access.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SandboxNetworkUpdateConfig> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SandboxNetworkUpdateConfig>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SandboxNetworkUpdateConfig>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SandboxNetworkUpdateConfig - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SandboxNetworkUpdateConfig> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SandboxNetworkUpdateConfig as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SandboxNetworkUpdateConfig - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Action taken when the sandbox times out.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum SandboxOnTimeout {
    #[serde(rename = "kill")]
    Kill,
    #[serde(rename = "pause")]
    Pause,
}

impl validator::Validate for SandboxOnTimeout {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for SandboxOnTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            SandboxOnTimeout::Kill => write!(f, "kill"),
            SandboxOnTimeout::Pause => write!(f, "pause"),
        }
    }
}

impl std::str::FromStr for SandboxOnTimeout {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "kill" => std::result::Result::Ok(SandboxOnTimeout::Kill),
            "pause" => std::result::Result::Ok(SandboxOnTimeout::Pause),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

/// Hard node-placement constraints applied and consumed by the multi-node gateway. A direct runtime-node request containing a placement object is rejected.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxPlacement {
    /// Exact required target node. Freshness, resource admission, and all other placement constraints still apply; no fallback to another node.
    #[serde(rename = "nodeID")]
    #[validate(length(min = 1, max = 200), custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,

    /// Sandbox IDs whose currently assigned nodes must be excluded. Every referenced assignment must exist and be live; placement fails closed otherwise.
    #[serde(rename = "differentNodeFrom")]
    #[validate(length(max = 32), custom(function = "check_xss_vec_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub different_node_from: Option<Vec<String>>,

    /// Sandbox IDs that define the required snapshot-compatibility class for the selected node. Every referenced assignment must exist and be live. AgentENV release, cluster, CPU identity, and Firecracker CPU configuration must match every reference.
    #[serde(rename = "snapshotCompatibleWith")]
    #[validate(length(max = 32), custom(function = "check_xss_vec_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_compatible_with: Option<Vec<String>>,
}

impl SandboxPlacement {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> SandboxPlacement {
        SandboxPlacement {
            node_id: None,
            different_node_from: None,
            snapshot_compatible_with: None,
        }
    }
}

/// Converts the SandboxPlacement value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SandboxPlacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            self.node_id
                .as_ref()
                .map(|node_id| ["nodeID".to_string(), node_id.to_string()].join(",")),
            self.different_node_from
                .as_ref()
                .map(|different_node_from| {
                    [
                        "differentNodeFrom".to_string(),
                        different_node_from
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join(","),
                    ]
                    .join(",")
                }),
            self.snapshot_compatible_with
                .as_ref()
                .map(|snapshot_compatible_with| {
                    [
                        "snapshotCompatibleWith".to_string(),
                        snapshot_compatible_with
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join(","),
                    ]
                    .join(",")
                }),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SandboxPlacement value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SandboxPlacement {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub node_id: Vec<String>,
            pub different_node_from: Vec<Vec<String>>,
            pub snapshot_compatible_with: Vec<Vec<String>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SandboxPlacement".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "nodeID" => intermediate_rep.node_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "differentNodeFrom" => return std::result::Result::Err(
                        "Parsing a container in this style is not supported in SandboxPlacement"
                            .to_string(),
                    ),
                    "snapshotCompatibleWith" => return std::result::Result::Err(
                        "Parsing a container in this style is not supported in SandboxPlacement"
                            .to_string(),
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SandboxPlacement".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SandboxPlacement {
            node_id: intermediate_rep.node_id.into_iter().next(),
            different_node_from: intermediate_rep.different_node_from.into_iter().next(),
            snapshot_compatible_with: intermediate_rep.snapshot_compatible_with.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SandboxPlacement> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SandboxPlacement>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SandboxPlacement>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SandboxPlacement - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SandboxPlacement> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SandboxPlacement as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SandboxPlacement - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxRefreshRequest {
    /// Duration for which the sandbox should be kept alive in seconds
    #[serde(rename = "duration")]
    #[validate(range(min = 0u16, max = 3600u16))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u16>,
}

impl SandboxRefreshRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> SandboxRefreshRequest {
        SandboxRefreshRequest { duration: None }
    }
}

/// Converts the SandboxRefreshRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SandboxRefreshRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            self.duration
                .as_ref()
                .map(|duration| ["duration".to_string(), duration.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SandboxRefreshRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SandboxRefreshRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub duration: Vec<u16>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SandboxRefreshRequest".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "duration" => intermediate_rep.duration.push(
                        <u16 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SandboxRefreshRequest".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SandboxRefreshRequest {
            duration: intermediate_rep.duration.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SandboxRefreshRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SandboxRefreshRequest>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SandboxRefreshRequest>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SandboxRefreshRequest - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SandboxRefreshRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SandboxRefreshRequest as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SandboxRefreshRequest - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxSnapshotRequest {
    /// Optional caller-assigned stable snapshot UUID. Repeating the same source, name, and UUID returns the already committed snapshot without capturing it again; reusing the UUID for a different snapshot is rejected.
    #[serde(rename = "snapshotId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<uuid::Uuid>,

    /// Optional name for the snapshot template. If a snapshot template with this name already exists, a new build will be assigned to the existing template instead of creating a new one.
    #[serde(rename = "name")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Keep the source sandbox paused before publishing and returning the committed snapshot. If publication fails, AgentENV attempts to resume the source so the request can be retried.
    #[serde(rename = "pauseAfterCapture")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pause_after_capture: Option<bool>,
}

impl SandboxSnapshotRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> SandboxSnapshotRequest {
        SandboxSnapshotRequest {
            snapshot_id: None,
            name: None,
            pause_after_capture: Some(false),
        }
    }
}

/// Converts the SandboxSnapshotRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SandboxSnapshotRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping snapshotId in query parameter serialization
            self.name
                .as_ref()
                .map(|name| ["name".to_string(), name.to_string()].join(",")),
            self.pause_after_capture
                .as_ref()
                .map(|pause_after_capture| {
                    [
                        "pauseAfterCapture".to_string(),
                        pause_after_capture.to_string(),
                    ]
                    .join(",")
                }),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SandboxSnapshotRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SandboxSnapshotRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub snapshot_id: Vec<uuid::Uuid>,
            pub name: Vec<String>,
            pub pause_after_capture: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SandboxSnapshotRequest".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "snapshotId" => intermediate_rep.snapshot_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "name" => intermediate_rep.name.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "pauseAfterCapture" => intermediate_rep.pause_after_capture.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SandboxSnapshotRequest".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SandboxSnapshotRequest {
            snapshot_id: intermediate_rep.snapshot_id.into_iter().next(),
            name: intermediate_rep.name.into_iter().next(),
            pause_after_capture: intermediate_rep.pause_after_capture.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SandboxSnapshotRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SandboxSnapshotRequest>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SandboxSnapshotRequest>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SandboxSnapshotRequest - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SandboxSnapshotRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SandboxSnapshotRequest as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SandboxSnapshotRequest - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// State of the sandbox
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum SandboxState {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "paused")]
    Paused,
}

impl validator::Validate for SandboxState {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for SandboxState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            SandboxState::Running => write!(f, "running"),
            SandboxState::Paused => write!(f, "paused"),
        }
    }
}

impl std::str::FromStr for SandboxState {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "running" => std::result::Result::Ok(SandboxState::Running),
            "paused" => std::result::Result::Ok(SandboxState::Paused),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxTimeoutRequest {
    /// Exact Node incarnation for funded renewal. Optional only for legacy SDK and SQL callers until Kubernetes lifecycle handoff; when supplied, executionLease is required.
    #[serde(rename = "targetNodeInstance")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_node_instance: Option<models::NodeLaunchTarget>,

    #[serde(rename = "executionLease")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_lease: Option<models::ExecutionLease>,

    /// Timeout in seconds from the current time after which the sandbox should expire
    #[serde(rename = "timeout")]
    #[validate(range(min = 0u32))]
    pub timeout: u32,
}

impl SandboxTimeoutRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(timeout: u32) -> SandboxTimeoutRequest {
        SandboxTimeoutRequest {
            target_node_instance: None,
            execution_lease: None,
            timeout,
        }
    }
}

/// Converts the SandboxTimeoutRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SandboxTimeoutRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping targetNodeInstance in query parameter serialization

            // Skipping executionLease in query parameter serialization
            Some("timeout".to_string()),
            Some(self.timeout.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SandboxTimeoutRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SandboxTimeoutRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub target_node_instance: Vec<models::NodeLaunchTarget>,
            pub execution_lease: Vec<models::ExecutionLease>,
            pub timeout: Vec<u32>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SandboxTimeoutRequest".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "targetNodeInstance" => intermediate_rep.target_node_instance.push(
                        <models::NodeLaunchTarget as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "executionLease" => intermediate_rep.execution_lease.push(
                        <models::ExecutionLease as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "timeout" => intermediate_rep.timeout.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SandboxTimeoutRequest".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SandboxTimeoutRequest {
            target_node_instance: intermediate_rep.target_node_instance.into_iter().next(),
            execution_lease: intermediate_rep.execution_lease.into_iter().next(),
            timeout: intermediate_rep
                .timeout
                .into_iter()
                .next()
                .ok_or_else(|| "timeout missing in SandboxTimeoutRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SandboxTimeoutRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SandboxTimeoutRequest>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SandboxTimeoutRequest>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SandboxTimeoutRequest - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SandboxTimeoutRequest> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SandboxTimeoutRequest as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SandboxTimeoutRequest - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Cumulative host resource use of the sandbox's most recent runtime instance on this node. A runtime instance is one Firecracker process: it starts when the sandbox boots or resumes here and ends when that process stops. Every counter is monotonic within an instance and starts from zero for the next one.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SandboxUsage {
    #[serde(rename = "sandboxID")]
    #[validate(custom(function = "check_xss_string"))]
    pub sandbox_id: String,

    /// Changes on every boot or resume of the sandbox on this node
    #[serde(rename = "runtimeInstanceID")]
    pub runtime_instance_id: uuid::Uuid,

    /// False once the runtime instance stopped; the counters are then final
    #[serde(rename = "running")]
    pub running: bool,

    /// When the counters for this runtime instance opened
    #[serde(rename = "startedAt")]
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// When the counters were last updated
    #[serde(rename = "sampledAt")]
    pub sampled_at: chrono::DateTime<chrono::Utc>,

    #[serde(rename = "sampleCount")]
    pub sample_count: u64,

    /// Whether the Firecracker process runs in a cgroup of its own. When false the CPU and memory counters are absent and only disk is measured.
    #[serde(rename = "cgroupAccounting")]
    pub cgroup_accounting: bool,

    /// Host CPU time consumed by the Firecracker process, in microseconds
    #[serde(rename = "cpuUsageMicros")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_usage_micros: Option<u64>,

    /// Host memory currently charged to the Firecracker process
    #[serde(rename = "memoryCurrentBytes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_current_bytes: Option<u64>,

    /// Integral of memoryCurrentBytes over the instance lifetime
    #[serde(rename = "memoryByteSeconds")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_byte_seconds: Option<u64>,

    /// Bytes the runtime work directory currently occupies on local disk
    #[serde(rename = "diskAllocatedBytes")]
    pub disk_allocated_bytes: u64,

    /// Integral of diskAllocatedBytes over the instance lifetime
    #[serde(rename = "diskByteSeconds")]
    pub disk_byte_seconds: u64,
}

impl SandboxUsage {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        sandbox_id: String,
        runtime_instance_id: uuid::Uuid,
        running: bool,
        started_at: chrono::DateTime<chrono::Utc>,
        sampled_at: chrono::DateTime<chrono::Utc>,
        sample_count: u64,
        cgroup_accounting: bool,
        disk_allocated_bytes: u64,
        disk_byte_seconds: u64,
    ) -> SandboxUsage {
        SandboxUsage {
            sandbox_id,
            runtime_instance_id,
            running,
            started_at,
            sampled_at,
            sample_count,
            cgroup_accounting,
            cpu_usage_micros: None,
            memory_current_bytes: None,
            memory_byte_seconds: None,
            disk_allocated_bytes,
            disk_byte_seconds,
        }
    }
}

/// Converts the SandboxUsage value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SandboxUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("sandboxID".to_string()),
            Some(self.sandbox_id.to_string()),
            // Skipping runtimeInstanceID in query parameter serialization
            Some("running".to_string()),
            Some(self.running.to_string()),
            // Skipping startedAt in query parameter serialization

            // Skipping sampledAt in query parameter serialization
            Some("sampleCount".to_string()),
            Some(self.sample_count.to_string()),
            Some("cgroupAccounting".to_string()),
            Some(self.cgroup_accounting.to_string()),
            self.cpu_usage_micros.as_ref().map(|cpu_usage_micros| {
                ["cpuUsageMicros".to_string(), cpu_usage_micros.to_string()].join(",")
            }),
            self.memory_current_bytes
                .as_ref()
                .map(|memory_current_bytes| {
                    [
                        "memoryCurrentBytes".to_string(),
                        memory_current_bytes.to_string(),
                    ]
                    .join(",")
                }),
            self.memory_byte_seconds
                .as_ref()
                .map(|memory_byte_seconds| {
                    [
                        "memoryByteSeconds".to_string(),
                        memory_byte_seconds.to_string(),
                    ]
                    .join(",")
                }),
            Some("diskAllocatedBytes".to_string()),
            Some(self.disk_allocated_bytes.to_string()),
            Some("diskByteSeconds".to_string()),
            Some(self.disk_byte_seconds.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SandboxUsage value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SandboxUsage {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub sandbox_id: Vec<String>,
            pub runtime_instance_id: Vec<uuid::Uuid>,
            pub running: Vec<bool>,
            pub started_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub sampled_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub sample_count: Vec<u64>,
            pub cgroup_accounting: Vec<bool>,
            pub cpu_usage_micros: Vec<u64>,
            pub memory_current_bytes: Vec<u64>,
            pub memory_byte_seconds: Vec<u64>,
            pub disk_allocated_bytes: Vec<u64>,
            pub disk_byte_seconds: Vec<u64>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SandboxUsage".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "sandboxID" => intermediate_rep.sandbox_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "runtimeInstanceID" => intermediate_rep.runtime_instance_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "running" => intermediate_rep.running.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "startedAt" => intermediate_rep.started_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sampledAt" => intermediate_rep.sampled_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "sampleCount" => intermediate_rep.sample_count.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "cgroupAccounting" => intermediate_rep.cgroup_accounting.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "cpuUsageMicros" => intermediate_rep.cpu_usage_micros.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "memoryCurrentBytes" => intermediate_rep.memory_current_bytes.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "memoryByteSeconds" => intermediate_rep.memory_byte_seconds.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "diskAllocatedBytes" => intermediate_rep.disk_allocated_bytes.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "diskByteSeconds" => intermediate_rep.disk_byte_seconds.push(
                        <u64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SandboxUsage".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SandboxUsage {
            sandbox_id: intermediate_rep
                .sandbox_id
                .into_iter()
                .next()
                .ok_or_else(|| "sandboxID missing in SandboxUsage".to_string())?,
            runtime_instance_id: intermediate_rep
                .runtime_instance_id
                .into_iter()
                .next()
                .ok_or_else(|| "runtimeInstanceID missing in SandboxUsage".to_string())?,
            running: intermediate_rep
                .running
                .into_iter()
                .next()
                .ok_or_else(|| "running missing in SandboxUsage".to_string())?,
            started_at: intermediate_rep
                .started_at
                .into_iter()
                .next()
                .ok_or_else(|| "startedAt missing in SandboxUsage".to_string())?,
            sampled_at: intermediate_rep
                .sampled_at
                .into_iter()
                .next()
                .ok_or_else(|| "sampledAt missing in SandboxUsage".to_string())?,
            sample_count: intermediate_rep
                .sample_count
                .into_iter()
                .next()
                .ok_or_else(|| "sampleCount missing in SandboxUsage".to_string())?,
            cgroup_accounting: intermediate_rep
                .cgroup_accounting
                .into_iter()
                .next()
                .ok_or_else(|| "cgroupAccounting missing in SandboxUsage".to_string())?,
            cpu_usage_micros: intermediate_rep.cpu_usage_micros.into_iter().next(),
            memory_current_bytes: intermediate_rep.memory_current_bytes.into_iter().next(),
            memory_byte_seconds: intermediate_rep.memory_byte_seconds.into_iter().next(),
            disk_allocated_bytes: intermediate_rep
                .disk_allocated_bytes
                .into_iter()
                .next()
                .ok_or_else(|| "diskAllocatedBytes missing in SandboxUsage".to_string())?,
            disk_byte_seconds: intermediate_rep
                .disk_byte_seconds
                .into_iter()
                .next()
                .ok_or_else(|| "diskByteSeconds missing in SandboxUsage".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SandboxUsage> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SandboxUsage>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SandboxUsage>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SandboxUsage - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SandboxUsage> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SandboxUsage as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SandboxUsage - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SnapshotInfo {
    /// The actual snapshot ID (stable identifier). Always contains the raw snapshot UUID, never an alias.
    #[serde(rename = "snapshotID")]
    #[validate(custom(function = "check_xss_string"))]
    pub snapshot_id: String,

    /// User-provided aliases for the snapshot. Empty if no alias was assigned during snapshot creation.
    #[serde(rename = "names")]
    #[validate(custom(function = "check_xss_vec_string"))]
    pub names: Vec<String>,

    /// CPU cores for the sandbox
    #[serde(rename = "cpuCount")]
    #[validate(range(min = 1u32))]
    pub cpu_count: u32,

    /// Memory for the sandbox in MiB
    #[serde(rename = "memoryMB")]
    #[validate(range(min = 128u32))]
    pub memory_mb: u32,

    /// Disk size for the sandbox in MiB
    #[serde(rename = "diskSizeMB")]
    #[validate(range(min = 0u32))]
    pub disk_size_mb: u32,

    /// Time when the snapshot was created
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Time when the snapshot was last updated
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// OverlayBD-native OCI image reference for the snapshot rootfs when source-registry image publication was enabled and succeeded. Omitted when no rootfs image was published.
    #[serde(rename = "imageRef")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_ref: Option<String>,
}

impl SnapshotInfo {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        snapshot_id: String,
        names: Vec<String>,
        cpu_count: u32,
        memory_mb: u32,
        disk_size_mb: u32,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> SnapshotInfo {
        SnapshotInfo {
            snapshot_id,
            names,
            cpu_count,
            memory_mb,
            disk_size_mb,
            created_at,
            updated_at,
            image_ref: None,
        }
    }
}

/// Converts the SnapshotInfo value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SnapshotInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("snapshotID".to_string()),
            Some(self.snapshot_id.to_string()),
            Some("names".to_string()),
            Some(
                self.names
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            Some("cpuCount".to_string()),
            Some(self.cpu_count.to_string()),
            Some("memoryMB".to_string()),
            Some(self.memory_mb.to_string()),
            Some("diskSizeMB".to_string()),
            Some(self.disk_size_mb.to_string()),
            // Skipping createdAt in query parameter serialization

            // Skipping updatedAt in query parameter serialization
            self.image_ref
                .as_ref()
                .map(|image_ref| ["imageRef".to_string(), image_ref.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SnapshotInfo value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SnapshotInfo {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub snapshot_id: Vec<String>,
            pub names: Vec<Vec<String>>,
            pub cpu_count: Vec<u32>,
            pub memory_mb: Vec<u32>,
            pub disk_size_mb: Vec<u32>,
            pub created_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub updated_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub image_ref: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SnapshotInfo".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "snapshotID" => intermediate_rep.snapshot_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "names" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in SnapshotInfo"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "cpuCount" => intermediate_rep.cpu_count.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "memoryMB" => intermediate_rep.memory_mb.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "diskSizeMB" => intermediate_rep.disk_size_mb.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "createdAt" => intermediate_rep.created_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "updatedAt" => intermediate_rep.updated_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "imageRef" => intermediate_rep.image_ref.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SnapshotInfo".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SnapshotInfo {
            snapshot_id: intermediate_rep
                .snapshot_id
                .into_iter()
                .next()
                .ok_or_else(|| "snapshotID missing in SnapshotInfo".to_string())?,
            names: intermediate_rep
                .names
                .into_iter()
                .next()
                .ok_or_else(|| "names missing in SnapshotInfo".to_string())?,
            cpu_count: intermediate_rep
                .cpu_count
                .into_iter()
                .next()
                .ok_or_else(|| "cpuCount missing in SnapshotInfo".to_string())?,
            memory_mb: intermediate_rep
                .memory_mb
                .into_iter()
                .next()
                .ok_or_else(|| "memoryMB missing in SnapshotInfo".to_string())?,
            disk_size_mb: intermediate_rep
                .disk_size_mb
                .into_iter()
                .next()
                .ok_or_else(|| "diskSizeMB missing in SnapshotInfo".to_string())?,
            created_at: intermediate_rep
                .created_at
                .into_iter()
                .next()
                .ok_or_else(|| "createdAt missing in SnapshotInfo".to_string())?,
            updated_at: intermediate_rep
                .updated_at
                .into_iter()
                .next()
                .ok_or_else(|| "updatedAt missing in SnapshotInfo".to_string())?,
            image_ref: intermediate_rep.image_ref.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SnapshotInfo> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SnapshotInfo>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SnapshotInfo>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SnapshotInfo - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SnapshotInfo> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SnapshotInfo as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SnapshotInfo - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SnapshotRootfsImageExport {
    #[serde(rename = "imageRef")]
    #[validate(custom(function = "check_xss_string"))]
    pub image_ref: String,

    #[serde(rename = "manifestDigest")]
    #[validate(custom(function = "check_xss_string"))]
    pub manifest_digest: String,

    #[serde(rename = "reused")]
    pub reused: bool,
}

impl SnapshotRootfsImageExport {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        image_ref: String,
        manifest_digest: String,
        reused: bool,
    ) -> SnapshotRootfsImageExport {
        SnapshotRootfsImageExport {
            image_ref,
            manifest_digest,
            reused,
        }
    }
}

/// Converts the SnapshotRootfsImageExport value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SnapshotRootfsImageExport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("imageRef".to_string()),
            Some(self.image_ref.to_string()),
            Some("manifestDigest".to_string()),
            Some(self.manifest_digest.to_string()),
            Some("reused".to_string()),
            Some(self.reused.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SnapshotRootfsImageExport value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SnapshotRootfsImageExport {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub image_ref: Vec<String>,
            pub manifest_digest: Vec<String>,
            pub reused: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SnapshotRootfsImageExport".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "imageRef" => intermediate_rep.image_ref.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "manifestDigest" => intermediate_rep.manifest_digest.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "reused" => intermediate_rep.reused.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SnapshotRootfsImageExport".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SnapshotRootfsImageExport {
            image_ref: intermediate_rep
                .image_ref
                .into_iter()
                .next()
                .ok_or_else(|| "imageRef missing in SnapshotRootfsImageExport".to_string())?,
            manifest_digest: intermediate_rep
                .manifest_digest
                .into_iter()
                .next()
                .ok_or_else(|| "manifestDigest missing in SnapshotRootfsImageExport".to_string())?,
            reused: intermediate_rep
                .reused
                .into_iter()
                .next()
                .ok_or_else(|| "reused missing in SnapshotRootfsImageExport".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SnapshotRootfsImageExport> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SnapshotRootfsImageExport>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SnapshotRootfsImageExport>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SnapshotRootfsImageExport - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<SnapshotRootfsImageExport> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SnapshotRootfsImageExport as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SnapshotRootfsImageExport - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct SnapshotRootfsImageExportRequest {
    /// Registry and repository path without a tag, digest or URL scheme
    #[serde(rename = "targetRepository")]
    #[validate(custom(function = "check_xss_string"))]
    pub target_repository: String,

    /// Explicit immutable publication tag chosen by the caller
    #[serde(rename = "tag")]
    #[validate(custom(function = "check_xss_string"))]
    pub tag: String,
}

impl SnapshotRootfsImageExportRequest {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(target_repository: String, tag: String) -> SnapshotRootfsImageExportRequest {
        SnapshotRootfsImageExportRequest {
            target_repository,
            tag,
        }
    }
}

/// Converts the SnapshotRootfsImageExportRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for SnapshotRootfsImageExportRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("targetRepository".to_string()),
            Some(self.target_repository.to_string()),
            Some("tag".to_string()),
            Some(self.tag.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SnapshotRootfsImageExportRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SnapshotRootfsImageExportRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub target_repository: Vec<String>,
            pub tag: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SnapshotRootfsImageExportRequest".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "targetRepository" => intermediate_rep.target_repository.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "tag" => intermediate_rep.tag.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SnapshotRootfsImageExportRequest"
                                .to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SnapshotRootfsImageExportRequest {
            target_repository: intermediate_rep
                .target_repository
                .into_iter()
                .next()
                .ok_or_else(|| {
                    "targetRepository missing in SnapshotRootfsImageExportRequest".to_string()
                })?,
            tag: intermediate_rep
                .tag
                .into_iter()
                .next()
                .ok_or_else(|| "tag missing in SnapshotRootfsImageExportRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<SnapshotRootfsImageExportRequest> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<SnapshotRootfsImageExportRequest>>
    for HeaderValue
{
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<SnapshotRootfsImageExportRequest>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for SnapshotRootfsImageExportRequest - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue>
    for header::IntoHeaderValue<SnapshotRootfsImageExportRequest>
{
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <SnapshotRootfsImageExportRequest as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into SnapshotRootfsImageExportRequest - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Template {
    /// Identifier of the template
    #[serde(rename = "templateID")]
    #[validate(custom(function = "check_xss_string"))]
    pub template_id: String,

    /// Identifier of the last successful build for given template
    #[serde(rename = "buildID")]
    #[validate(custom(function = "check_xss_string"))]
    pub build_id: String,

    /// CPU cores for the sandbox
    #[serde(rename = "cpuCount")]
    #[validate(range(min = 1u32))]
    pub cpu_count: u32,

    /// Memory for the sandbox in MiB
    #[serde(rename = "memoryMB")]
    #[validate(range(min = 128u32))]
    pub memory_mb: u32,

    /// Disk size for the sandbox in MiB
    #[serde(rename = "diskSizeMB")]
    #[validate(range(min = 0u32))]
    pub disk_size_mb: u32,

    /// Whether the template is public or only accessible by the team
    #[serde(rename = "public")]
    pub public: bool,

    /// Names of the template (namespace/alias format when namespaced)
    #[serde(rename = "names")]
    #[validate(custom(function = "check_xss_vec_string"))]
    pub names: Vec<String>,

    /// Time when the template was created
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Time when the template was last updated
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// Time when the template was last used
    #[serde(rename = "lastSpawnedAt")]
    pub last_spawned_at: Nullable<chrono::DateTime<chrono::Utc>>,

    /// Number of times the template was used
    #[serde(rename = "spawnCount")]
    pub spawn_count: i64,

    /// Number of times the template was built
    #[serde(rename = "buildCount")]
    pub build_count: i32,

    /// Version of the envd running in the sandbox
    #[serde(rename = "envdVersion")]
    #[validate(custom(function = "check_xss_string"))]
    pub envd_version: String,

    #[serde(rename = "buildStatus")]
    #[validate(nested)]
    pub build_status: models::TemplateBuildStatus,
}

impl Template {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        template_id: String,
        build_id: String,
        cpu_count: u32,
        memory_mb: u32,
        disk_size_mb: u32,
        public: bool,
        names: Vec<String>,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
        last_spawned_at: Nullable<chrono::DateTime<chrono::Utc>>,
        spawn_count: i64,
        build_count: i32,
        envd_version: String,
        build_status: models::TemplateBuildStatus,
    ) -> Template {
        Template {
            template_id,
            build_id,
            cpu_count,
            memory_mb,
            disk_size_mb,
            public,
            names,
            created_at,
            updated_at,
            last_spawned_at,
            spawn_count,
            build_count,
            envd_version,
            build_status,
        }
    }
}

/// Converts the Template value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("templateID".to_string()),
            Some(self.template_id.to_string()),
            Some("buildID".to_string()),
            Some(self.build_id.to_string()),
            Some("cpuCount".to_string()),
            Some(self.cpu_count.to_string()),
            Some("memoryMB".to_string()),
            Some(self.memory_mb.to_string()),
            Some("diskSizeMB".to_string()),
            Some(self.disk_size_mb.to_string()),
            Some("public".to_string()),
            Some(self.public.to_string()),
            Some("names".to_string()),
            Some(
                self.names
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            // Skipping createdAt in query parameter serialization

            // Skipping updatedAt in query parameter serialization

            // Skipping lastSpawnedAt in query parameter serialization
            Some("spawnCount".to_string()),
            Some(self.spawn_count.to_string()),
            Some("buildCount".to_string()),
            Some(self.build_count.to_string()),
            Some("envdVersion".to_string()),
            Some(self.envd_version.to_string()),
            // Skipping buildStatus in query parameter serialization
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Template value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Template {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub template_id: Vec<String>,
            pub build_id: Vec<String>,
            pub cpu_count: Vec<u32>,
            pub memory_mb: Vec<u32>,
            pub disk_size_mb: Vec<u32>,
            pub public: Vec<bool>,
            pub names: Vec<Vec<String>>,
            pub created_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub updated_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub last_spawned_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub spawn_count: Vec<i64>,
            pub build_count: Vec<i32>,
            pub envd_version: Vec<String>,
            pub build_status: Vec<models::TemplateBuildStatus>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing Template".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "templateID" => intermediate_rep.template_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "buildID" => intermediate_rep.build_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "cpuCount" => intermediate_rep.cpu_count.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "memoryMB" => intermediate_rep.memory_mb.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "diskSizeMB" => intermediate_rep.disk_size_mb.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "public" => intermediate_rep.public.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "names" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in Template"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "createdAt" => intermediate_rep.created_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "updatedAt" => intermediate_rep.updated_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    "lastSpawnedAt" => {
                        return std::result::Result::Err(
                            "Parsing a nullable type in this style is not supported in Template"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "spawnCount" => intermediate_rep.spawn_count.push(
                        <i64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "buildCount" => intermediate_rep.build_count.push(
                        <i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "envdVersion" => intermediate_rep.envd_version.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "buildStatus" => intermediate_rep.build_status.push(
                        <models::TemplateBuildStatus as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing Template".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Template {
            template_id: intermediate_rep
                .template_id
                .into_iter()
                .next()
                .ok_or_else(|| "templateID missing in Template".to_string())?,
            build_id: intermediate_rep
                .build_id
                .into_iter()
                .next()
                .ok_or_else(|| "buildID missing in Template".to_string())?,
            cpu_count: intermediate_rep
                .cpu_count
                .into_iter()
                .next()
                .ok_or_else(|| "cpuCount missing in Template".to_string())?,
            memory_mb: intermediate_rep
                .memory_mb
                .into_iter()
                .next()
                .ok_or_else(|| "memoryMB missing in Template".to_string())?,
            disk_size_mb: intermediate_rep
                .disk_size_mb
                .into_iter()
                .next()
                .ok_or_else(|| "diskSizeMB missing in Template".to_string())?,
            public: intermediate_rep
                .public
                .into_iter()
                .next()
                .ok_or_else(|| "public missing in Template".to_string())?,
            names: intermediate_rep
                .names
                .into_iter()
                .next()
                .ok_or_else(|| "names missing in Template".to_string())?,
            created_at: intermediate_rep
                .created_at
                .into_iter()
                .next()
                .ok_or_else(|| "createdAt missing in Template".to_string())?,
            updated_at: intermediate_rep
                .updated_at
                .into_iter()
                .next()
                .ok_or_else(|| "updatedAt missing in Template".to_string())?,
            last_spawned_at: std::result::Result::Err(
                "Nullable types not supported in Template".to_string(),
            )?,
            spawn_count: intermediate_rep
                .spawn_count
                .into_iter()
                .next()
                .ok_or_else(|| "spawnCount missing in Template".to_string())?,
            build_count: intermediate_rep
                .build_count
                .into_iter()
                .next()
                .ok_or_else(|| "buildCount missing in Template".to_string())?,
            envd_version: intermediate_rep
                .envd_version
                .into_iter()
                .next()
                .ok_or_else(|| "envdVersion missing in Template".to_string())?,
            build_status: intermediate_rep
                .build_status
                .into_iter()
                .next()
                .ok_or_else(|| "buildStatus missing in Template".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Template> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<Template>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<Template>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for Template - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<Template> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <Template as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into Template - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplateAliasResponse {
    /// Identifier of the template
    #[serde(rename = "templateID")]
    #[validate(custom(function = "check_xss_string"))]
    pub template_id: String,

    /// Whether the template is public or only accessible by the team
    #[serde(rename = "public")]
    pub public: bool,
}

impl TemplateAliasResponse {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(template_id: String, public: bool) -> TemplateAliasResponse {
        TemplateAliasResponse {
            template_id,
            public,
        }
    }
}

/// Converts the TemplateAliasResponse value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TemplateAliasResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("templateID".to_string()),
            Some(self.template_id.to_string()),
            Some("public".to_string()),
            Some(self.public.to_string()),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TemplateAliasResponse value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TemplateAliasResponse {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub template_id: Vec<String>,
            pub public: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing TemplateAliasResponse".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "templateID" => intermediate_rep.template_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "public" => intermediate_rep.public.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing TemplateAliasResponse".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TemplateAliasResponse {
            template_id: intermediate_rep
                .template_id
                .into_iter()
                .next()
                .ok_or_else(|| "templateID missing in TemplateAliasResponse".to_string())?,
            public: intermediate_rep
                .public
                .into_iter()
                .next()
                .ok_or_else(|| "public missing in TemplateAliasResponse".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TemplateAliasResponse> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TemplateAliasResponse>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<TemplateAliasResponse>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for TemplateAliasResponse - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TemplateAliasResponse> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <TemplateAliasResponse as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into TemplateAliasResponse - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplateBuild {
    /// Identifier of the build
    #[serde(rename = "buildID")]
    pub build_id: uuid::Uuid,

    #[serde(rename = "status")]
    #[validate(nested)]
    pub status: models::TemplateBuildStatus,

    /// Time when the build was created
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Time when the build was last updated
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// Time when the build was finished
    #[serde(rename = "finishedAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,

    /// CPU cores for the sandbox
    #[serde(rename = "cpuCount")]
    #[validate(range(min = 1u32))]
    pub cpu_count: u32,

    /// Memory for the sandbox in MiB
    #[serde(rename = "memoryMB")]
    #[validate(range(min = 128u32))]
    pub memory_mb: u32,

    /// Disk size for the sandbox in MiB
    #[serde(rename = "diskSizeMB")]
    #[validate(range(min = 0u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_size_mb: Option<u32>,

    /// Version of the envd running in the sandbox
    #[serde(rename = "envdVersion")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envd_version: Option<String>,
}

impl TemplateBuild {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        build_id: uuid::Uuid,
        status: models::TemplateBuildStatus,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
        cpu_count: u32,
        memory_mb: u32,
    ) -> TemplateBuild {
        TemplateBuild {
            build_id,
            status,
            created_at,
            updated_at,
            finished_at: None,
            cpu_count,
            memory_mb,
            disk_size_mb: None,
            envd_version: None,
        }
    }
}

/// Converts the TemplateBuild value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TemplateBuild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            // Skipping buildID in query parameter serialization

            // Skipping status in query parameter serialization

            // Skipping createdAt in query parameter serialization

            // Skipping updatedAt in query parameter serialization

            // Skipping finishedAt in query parameter serialization
            Some("cpuCount".to_string()),
            Some(self.cpu_count.to_string()),
            Some("memoryMB".to_string()),
            Some(self.memory_mb.to_string()),
            self.disk_size_mb
                .as_ref()
                .map(|disk_size_mb| ["diskSizeMB".to_string(), disk_size_mb.to_string()].join(",")),
            self.envd_version.as_ref().map(|envd_version| {
                ["envdVersion".to_string(), envd_version.to_string()].join(",")
            }),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TemplateBuild value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TemplateBuild {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub build_id: Vec<uuid::Uuid>,
            pub status: Vec<models::TemplateBuildStatus>,
            pub created_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub updated_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub finished_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub cpu_count: Vec<u32>,
            pub memory_mb: Vec<u32>,
            pub disk_size_mb: Vec<u32>,
            pub envd_version: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing TemplateBuild".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "buildID" => intermediate_rep.build_id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "status" => intermediate_rep.status.push(
                        <models::TemplateBuildStatus as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "createdAt" => intermediate_rep.created_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "updatedAt" => intermediate_rep.updated_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "finishedAt" => intermediate_rep.finished_at.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "cpuCount" => intermediate_rep.cpu_count.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "memoryMB" => intermediate_rep.memory_mb.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "diskSizeMB" => intermediate_rep.disk_size_mb.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "envdVersion" => intermediate_rep.envd_version.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing TemplateBuild".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TemplateBuild {
            build_id: intermediate_rep
                .build_id
                .into_iter()
                .next()
                .ok_or_else(|| "buildID missing in TemplateBuild".to_string())?,
            status: intermediate_rep
                .status
                .into_iter()
                .next()
                .ok_or_else(|| "status missing in TemplateBuild".to_string())?,
            created_at: intermediate_rep
                .created_at
                .into_iter()
                .next()
                .ok_or_else(|| "createdAt missing in TemplateBuild".to_string())?,
            updated_at: intermediate_rep
                .updated_at
                .into_iter()
                .next()
                .ok_or_else(|| "updatedAt missing in TemplateBuild".to_string())?,
            finished_at: intermediate_rep.finished_at.into_iter().next(),
            cpu_count: intermediate_rep
                .cpu_count
                .into_iter()
                .next()
                .ok_or_else(|| "cpuCount missing in TemplateBuild".to_string())?,
            memory_mb: intermediate_rep
                .memory_mb
                .into_iter()
                .next()
                .ok_or_else(|| "memoryMB missing in TemplateBuild".to_string())?,
            disk_size_mb: intermediate_rep.disk_size_mb.into_iter().next(),
            envd_version: intermediate_rep.envd_version.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TemplateBuild> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TemplateBuild>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<TemplateBuild>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for TemplateBuild - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TemplateBuild> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <TemplateBuild as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into TemplateBuild - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplateBuildInfo {
    /// Build logs
    #[serde(rename = "logs")]
    #[validate(custom(function = "check_xss_vec_string"))]
    pub logs: Vec<String>,

    /// Build logs structured
    #[serde(rename = "logEntries")]
    #[validate(nested)]
    pub log_entries: Vec<models::BuildLogEntry>,

    /// Identifier of the template
    #[serde(rename = "templateID")]
    #[validate(custom(function = "check_xss_string"))]
    pub template_id: String,

    /// Identifier of the build
    #[serde(rename = "buildID")]
    #[validate(custom(function = "check_xss_string"))]
    pub build_id: String,

    #[serde(rename = "status")]
    #[validate(nested)]
    pub status: models::TemplateBuildStatus,

    #[serde(rename = "reason")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<models::BuildStatusReason>,
}

impl TemplateBuildInfo {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        logs: Vec<String>,
        log_entries: Vec<models::BuildLogEntry>,
        template_id: String,
        build_id: String,
        status: models::TemplateBuildStatus,
    ) -> TemplateBuildInfo {
        TemplateBuildInfo {
            logs,
            log_entries,
            template_id,
            build_id,
            status,
            reason: None,
        }
    }
}

/// Converts the TemplateBuildInfo value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TemplateBuildInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("logs".to_string()),
            Some(
                self.logs
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            // Skipping logEntries in query parameter serialization
            Some("templateID".to_string()),
            Some(self.template_id.to_string()),
            Some("buildID".to_string()),
            Some(self.build_id.to_string()),
            // Skipping status in query parameter serialization

            // Skipping reason in query parameter serialization
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TemplateBuildInfo value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TemplateBuildInfo {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub logs: Vec<Vec<String>>,
            pub log_entries: Vec<Vec<models::BuildLogEntry>>,
            pub template_id: Vec<String>,
            pub build_id: Vec<String>,
            pub status: Vec<models::TemplateBuildStatus>,
            pub reason: Vec<models::BuildStatusReason>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing TemplateBuildInfo".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "logs" => return std::result::Result::Err(
                        "Parsing a container in this style is not supported in TemplateBuildInfo"
                            .to_string(),
                    ),
                    "logEntries" => return std::result::Result::Err(
                        "Parsing a container in this style is not supported in TemplateBuildInfo"
                            .to_string(),
                    ),
                    #[allow(clippy::redundant_clone)]
                    "templateID" => intermediate_rep.template_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "buildID" => intermediate_rep.build_id.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "status" => intermediate_rep.status.push(
                        <models::TemplateBuildStatus as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "reason" => intermediate_rep.reason.push(
                        <models::BuildStatusReason as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing TemplateBuildInfo".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TemplateBuildInfo {
            logs: intermediate_rep
                .logs
                .into_iter()
                .next()
                .ok_or_else(|| "logs missing in TemplateBuildInfo".to_string())?,
            log_entries: intermediate_rep
                .log_entries
                .into_iter()
                .next()
                .ok_or_else(|| "logEntries missing in TemplateBuildInfo".to_string())?,
            template_id: intermediate_rep
                .template_id
                .into_iter()
                .next()
                .ok_or_else(|| "templateID missing in TemplateBuildInfo".to_string())?,
            build_id: intermediate_rep
                .build_id
                .into_iter()
                .next()
                .ok_or_else(|| "buildID missing in TemplateBuildInfo".to_string())?,
            status: intermediate_rep
                .status
                .into_iter()
                .next()
                .ok_or_else(|| "status missing in TemplateBuildInfo".to_string())?,
            reason: intermediate_rep.reason.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TemplateBuildInfo> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TemplateBuildInfo>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<TemplateBuildInfo>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for TemplateBuildInfo - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TemplateBuildInfo> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <TemplateBuildInfo as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into TemplateBuildInfo - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplateBuildRequestV3 {
    /// Name of the template. Can include a tag with colon separator (e.g. \"my-template\" or \"my-template:v1\"). If tag is included, it will be treated as if the tag was provided in the tags array.
    #[serde(rename = "name")]
    #[validate(length(max = 128), custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Tags to assign to the template build
    #[serde(rename = "tags")]
    #[validate(custom(function = "check_xss_vec_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    /// CPU cores for the sandbox
    #[serde(rename = "cpuCount")]
    #[validate(range(min = 1u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_count: Option<u32>,

    /// Memory for the sandbox in MiB
    #[serde(rename = "memoryMB")]
    #[validate(range(min = 128u32))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<u32>,
}

impl TemplateBuildRequestV3 {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> TemplateBuildRequestV3 {
        TemplateBuildRequestV3 {
            name: None,
            tags: None,
            cpu_count: None,
            memory_mb: None,
        }
    }
}

/// Converts the TemplateBuildRequestV3 value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TemplateBuildRequestV3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            self.name
                .as_ref()
                .map(|name| ["name".to_string(), name.to_string()].join(",")),
            self.tags.as_ref().map(|tags| {
                [
                    "tags".to_string(),
                    tags.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                ]
                .join(",")
            }),
            self.cpu_count
                .as_ref()
                .map(|cpu_count| ["cpuCount".to_string(), cpu_count.to_string()].join(",")),
            self.memory_mb
                .as_ref()
                .map(|memory_mb| ["memoryMB".to_string(), memory_mb.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TemplateBuildRequestV3 value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TemplateBuildRequestV3 {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub name: Vec<String>,
            pub tags: Vec<Vec<String>>,
            pub cpu_count: Vec<u32>,
            pub memory_mb: Vec<u32>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing TemplateBuildRequestV3".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "name" => intermediate_rep.name.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "tags" => return std::result::Result::Err("Parsing a container in this style is not supported in TemplateBuildRequestV3".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "cpuCount" => intermediate_rep.cpu_count.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "memoryMB" => intermediate_rep.memory_mb.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing TemplateBuildRequestV3".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TemplateBuildRequestV3 {
            name: intermediate_rep.name.into_iter().next(),
            tags: intermediate_rep.tags.into_iter().next(),
            cpu_count: intermediate_rep.cpu_count.into_iter().next(),
            memory_mb: intermediate_rep.memory_mb.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TemplateBuildRequestV3> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TemplateBuildRequestV3>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<TemplateBuildRequestV3>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for TemplateBuildRequestV3 - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TemplateBuildRequestV3> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <TemplateBuildRequestV3 as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into TemplateBuildRequestV3 - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplateBuildStartV2 {
    /// Image to use as a base for the template build
    #[serde(rename = "fromImage")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_image: Option<String>,

    /// Template to use as a base for the template build
    #[serde(rename = "fromTemplate")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_template: Option<String>,

    /// Whether the whole build should be forced to run regardless of the cache
    #[serde(rename = "force")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,

    /// List of steps to execute in the template build
    #[serde(rename = "steps")]
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<models::TemplateStep>>,

    /// Start command to execute in the template after the build
    #[serde(rename = "startCmd")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_cmd: Option<String>,

    /// Ready check command to execute in the template after the build
    #[serde(rename = "readyCmd")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready_cmd: Option<String>,
}

impl TemplateBuildStartV2 {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new() -> TemplateBuildStartV2 {
        TemplateBuildStartV2 {
            from_image: None,
            from_template: None,
            force: Some(false),
            steps: None,
            start_cmd: None,
            ready_cmd: None,
        }
    }
}

/// Converts the TemplateBuildStartV2 value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TemplateBuildStartV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            self.from_image
                .as_ref()
                .map(|from_image| ["fromImage".to_string(), from_image.to_string()].join(",")),
            self.from_template.as_ref().map(|from_template| {
                ["fromTemplate".to_string(), from_template.to_string()].join(",")
            }),
            self.force
                .as_ref()
                .map(|force| ["force".to_string(), force.to_string()].join(",")),
            // Skipping steps in query parameter serialization
            self.start_cmd
                .as_ref()
                .map(|start_cmd| ["startCmd".to_string(), start_cmd.to_string()].join(",")),
            self.ready_cmd
                .as_ref()
                .map(|ready_cmd| ["readyCmd".to_string(), ready_cmd.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TemplateBuildStartV2 value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TemplateBuildStartV2 {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub from_image: Vec<String>,
            pub from_template: Vec<String>,
            pub force: Vec<bool>,
            pub steps: Vec<Vec<models::TemplateStep>>,
            pub start_cmd: Vec<String>,
            pub ready_cmd: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing TemplateBuildStartV2".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "fromImage" => intermediate_rep.from_image.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "fromTemplate" => intermediate_rep.from_template.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "force" => intermediate_rep.force.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "steps" => return std::result::Result::Err("Parsing a container in this style is not supported in TemplateBuildStartV2".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "startCmd" => intermediate_rep.start_cmd.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "readyCmd" => intermediate_rep.ready_cmd.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing TemplateBuildStartV2".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TemplateBuildStartV2 {
            from_image: intermediate_rep.from_image.into_iter().next(),
            from_template: intermediate_rep.from_template.into_iter().next(),
            force: intermediate_rep.force.into_iter().next(),
            steps: intermediate_rep.steps.into_iter().next(),
            start_cmd: intermediate_rep.start_cmd.into_iter().next(),
            ready_cmd: intermediate_rep.ready_cmd.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TemplateBuildStartV2> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TemplateBuildStartV2>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<TemplateBuildStartV2>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for TemplateBuildStartV2 - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TemplateBuildStartV2> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <TemplateBuildStartV2 as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into TemplateBuildStartV2 - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Status of the template build
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types, clippy::large_enum_variant)]
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "conversion", derive(frunk_enum_derive::LabelledGenericEnum))]
pub enum TemplateBuildStatus {
    #[serde(rename = "building")]
    Building,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "error")]
    Error,
}

impl validator::Validate for TemplateBuildStatus {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        std::result::Result::Ok(())
    }
}

impl std::fmt::Display for TemplateBuildStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            TemplateBuildStatus::Building => write!(f, "building"),
            TemplateBuildStatus::Waiting => write!(f, "waiting"),
            TemplateBuildStatus::Ready => write!(f, "ready"),
            TemplateBuildStatus::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for TemplateBuildStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "building" => std::result::Result::Ok(TemplateBuildStatus::Building),
            "waiting" => std::result::Result::Ok(TemplateBuildStatus::Waiting),
            "ready" => std::result::Result::Ok(TemplateBuildStatus::Ready),
            "error" => std::result::Result::Ok(TemplateBuildStatus::Error),
            _ => std::result::Result::Err(format!(r#"Value not valid: {s}"#)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplateRequestResponseV3 {
    /// Identifier of the template
    #[serde(rename = "templateID")]
    #[validate(custom(function = "check_xss_string"))]
    pub template_id: String,

    /// Identifier of the last successful build for given template
    #[serde(rename = "buildID")]
    #[validate(custom(function = "check_xss_string"))]
    pub build_id: String,

    /// Whether the template is public or only accessible by the team
    #[serde(rename = "public")]
    pub public: bool,

    /// Names of the template
    #[serde(rename = "names")]
    #[validate(custom(function = "check_xss_vec_string"))]
    pub names: Vec<String>,

    /// Tags assigned to the template build
    #[serde(rename = "tags")]
    #[validate(custom(function = "check_xss_vec_string"))]
    pub tags: Vec<String>,

    /// Aliases of the template
    #[serde(rename = "aliases")]
    #[validate(custom(function = "check_xss_vec_string"))]
    pub aliases: Vec<String>,
}

impl TemplateRequestResponseV3 {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        template_id: String,
        build_id: String,
        public: bool,
        names: Vec<String>,
        tags: Vec<String>,
        aliases: Vec<String>,
    ) -> TemplateRequestResponseV3 {
        TemplateRequestResponseV3 {
            template_id,
            build_id,
            public,
            names,
            tags,
            aliases,
        }
    }
}

/// Converts the TemplateRequestResponseV3 value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TemplateRequestResponseV3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("templateID".to_string()),
            Some(self.template_id.to_string()),
            Some("buildID".to_string()),
            Some(self.build_id.to_string()),
            Some("public".to_string()),
            Some(self.public.to_string()),
            Some("names".to_string()),
            Some(
                self.names
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            Some("tags".to_string()),
            Some(
                self.tags
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            Some("aliases".to_string()),
            Some(
                self.aliases
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TemplateRequestResponseV3 value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TemplateRequestResponseV3 {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub template_id: Vec<String>,
            pub build_id: Vec<String>,
            pub public: Vec<bool>,
            pub names: Vec<Vec<String>>,
            pub tags: Vec<Vec<String>>,
            pub aliases: Vec<Vec<String>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing TemplateRequestResponseV3".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "templateID" => intermediate_rep.template_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "buildID" => intermediate_rep.build_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "public" => intermediate_rep.public.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "names" => return std::result::Result::Err("Parsing a container in this style is not supported in TemplateRequestResponseV3".to_string()),
                    "tags" => return std::result::Result::Err("Parsing a container in this style is not supported in TemplateRequestResponseV3".to_string()),
                    "aliases" => return std::result::Result::Err("Parsing a container in this style is not supported in TemplateRequestResponseV3".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing TemplateRequestResponseV3".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TemplateRequestResponseV3 {
            template_id: intermediate_rep
                .template_id
                .into_iter()
                .next()
                .ok_or_else(|| "templateID missing in TemplateRequestResponseV3".to_string())?,
            build_id: intermediate_rep
                .build_id
                .into_iter()
                .next()
                .ok_or_else(|| "buildID missing in TemplateRequestResponseV3".to_string())?,
            public: intermediate_rep
                .public
                .into_iter()
                .next()
                .ok_or_else(|| "public missing in TemplateRequestResponseV3".to_string())?,
            names: intermediate_rep
                .names
                .into_iter()
                .next()
                .ok_or_else(|| "names missing in TemplateRequestResponseV3".to_string())?,
            tags: intermediate_rep
                .tags
                .into_iter()
                .next()
                .ok_or_else(|| "tags missing in TemplateRequestResponseV3".to_string())?,
            aliases: intermediate_rep
                .aliases
                .into_iter()
                .next()
                .ok_or_else(|| "aliases missing in TemplateRequestResponseV3".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TemplateRequestResponseV3> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TemplateRequestResponseV3>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<TemplateRequestResponseV3>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for TemplateRequestResponseV3 - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TemplateRequestResponseV3> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <TemplateRequestResponseV3 as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into TemplateRequestResponseV3 - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

/// Step in the template build process
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplateStep {
    /// Type of the step
    #[serde(rename = "type")]
    #[validate(custom(function = "check_xss_string"))]
    pub r_type: String,

    /// Arguments for the step
    #[serde(rename = "args")]
    #[validate(custom(function = "check_xss_vec_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,

    /// Hash of the files used in the step
    #[serde(rename = "filesHash")]
    #[validate(custom(function = "check_xss_string"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_hash: Option<String>,

    /// Whether the step should be forced to run regardless of the cache
    #[serde(rename = "force")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
}

impl TemplateStep {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(r_type: String) -> TemplateStep {
        TemplateStep {
            r_type,
            args: None,
            files_hash: None,
            force: Some(false),
        }
    }
}

/// Converts the TemplateStep value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TemplateStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("type".to_string()),
            Some(self.r_type.to_string()),
            self.args.as_ref().map(|args| {
                [
                    "args".to_string(),
                    args.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                ]
                .join(",")
            }),
            self.files_hash
                .as_ref()
                .map(|files_hash| ["filesHash".to_string(), files_hash.to_string()].join(",")),
            self.force
                .as_ref()
                .map(|force| ["force".to_string(), force.to_string()].join(",")),
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TemplateStep value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TemplateStep {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub r_type: Vec<String>,
            pub args: Vec<Vec<String>>,
            pub files_hash: Vec<String>,
            pub force: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing TemplateStep".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "type" => intermediate_rep.r_type.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "args" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in TemplateStep"
                                .to_string(),
                        );
                    }
                    #[allow(clippy::redundant_clone)]
                    "filesHash" => intermediate_rep.files_hash.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "force" => intermediate_rep.force.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing TemplateStep".to_string(),
                        );
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TemplateStep {
            r_type: intermediate_rep
                .r_type
                .into_iter()
                .next()
                .ok_or_else(|| "type missing in TemplateStep".to_string())?,
            args: intermediate_rep.args.into_iter().next(),
            files_hash: intermediate_rep.files_hash.into_iter().next(),
            force: intermediate_rep.force.into_iter().next(),
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TemplateStep> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TemplateStep>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<TemplateStep>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for TemplateStep - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TemplateStep> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <TemplateStep as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into TemplateStep - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct TemplateWithBuilds {
    /// Identifier of the template
    #[serde(rename = "templateID")]
    #[validate(custom(function = "check_xss_string"))]
    pub template_id: String,

    /// Whether the template is public or only accessible by the team
    #[serde(rename = "public")]
    pub public: bool,

    /// Names of the template (namespace/alias format when namespaced)
    #[serde(rename = "names")]
    #[validate(custom(function = "check_xss_vec_string"))]
    pub names: Vec<String>,

    /// Time when the template was created
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Time when the template was last updated
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// Time when the template was last used
    #[serde(rename = "lastSpawnedAt")]
    pub last_spawned_at: Nullable<chrono::DateTime<chrono::Utc>>,

    /// Number of times the template was used
    #[serde(rename = "spawnCount")]
    pub spawn_count: i64,

    /// List of builds for the template
    #[serde(rename = "builds")]
    #[validate(nested)]
    pub builds: Vec<models::TemplateBuild>,
}

impl TemplateWithBuilds {
    #[allow(clippy::new_without_default, clippy::too_many_arguments)]
    pub fn new(
        template_id: String,
        public: bool,
        names: Vec<String>,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
        last_spawned_at: Nullable<chrono::DateTime<chrono::Utc>>,
        spawn_count: i64,
        builds: Vec<models::TemplateBuild>,
    ) -> TemplateWithBuilds {
        TemplateWithBuilds {
            template_id,
            public,
            names,
            created_at,
            updated_at,
            last_spawned_at,
            spawn_count,
            builds,
        }
    }
}

/// Converts the TemplateWithBuilds value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::fmt::Display for TemplateWithBuilds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<Option<String>> = vec![
            Some("templateID".to_string()),
            Some(self.template_id.to_string()),
            Some("public".to_string()),
            Some(self.public.to_string()),
            Some("names".to_string()),
            Some(
                self.names
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            // Skipping createdAt in query parameter serialization

            // Skipping updatedAt in query parameter serialization

            // Skipping lastSpawnedAt in query parameter serialization
            Some("spawnCount".to_string()),
            Some(self.spawn_count.to_string()),
            // Skipping builds in query parameter serialization
        ];

        write!(
            f,
            "{}",
            params.into_iter().flatten().collect::<Vec<_>>().join(",")
        )
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a TemplateWithBuilds value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for TemplateWithBuilds {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub template_id: Vec<String>,
            pub public: Vec<bool>,
            pub names: Vec<Vec<String>>,
            pub created_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub updated_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub last_spawned_at: Vec<chrono::DateTime<chrono::Utc>>,
            pub spawn_count: Vec<i64>,
            pub builds: Vec<Vec<models::TemplateBuild>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing TemplateWithBuilds".to_string(),
                    );
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "templateID" => intermediate_rep.template_id.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "public" => intermediate_rep.public.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "names" => return std::result::Result::Err("Parsing a container in this style is not supported in TemplateWithBuilds".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "createdAt" => intermediate_rep.created_at.push(<chrono::DateTime::<chrono::Utc> as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "updatedAt" => intermediate_rep.updated_at.push(<chrono::DateTime::<chrono::Utc> as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "lastSpawnedAt" => return std::result::Result::Err("Parsing a nullable type in this style is not supported in TemplateWithBuilds".to_string()),
                    #[allow(clippy::redundant_clone)]
                    "spawnCount" => intermediate_rep.spawn_count.push(<i64 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "builds" => return std::result::Result::Err("Parsing a container in this style is not supported in TemplateWithBuilds".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing TemplateWithBuilds".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(TemplateWithBuilds {
            template_id: intermediate_rep
                .template_id
                .into_iter()
                .next()
                .ok_or_else(|| "templateID missing in TemplateWithBuilds".to_string())?,
            public: intermediate_rep
                .public
                .into_iter()
                .next()
                .ok_or_else(|| "public missing in TemplateWithBuilds".to_string())?,
            names: intermediate_rep
                .names
                .into_iter()
                .next()
                .ok_or_else(|| "names missing in TemplateWithBuilds".to_string())?,
            created_at: intermediate_rep
                .created_at
                .into_iter()
                .next()
                .ok_or_else(|| "createdAt missing in TemplateWithBuilds".to_string())?,
            updated_at: intermediate_rep
                .updated_at
                .into_iter()
                .next()
                .ok_or_else(|| "updatedAt missing in TemplateWithBuilds".to_string())?,
            last_spawned_at: std::result::Result::Err(
                "Nullable types not supported in TemplateWithBuilds".to_string(),
            )?,
            spawn_count: intermediate_rep
                .spawn_count
                .into_iter()
                .next()
                .ok_or_else(|| "spawnCount missing in TemplateWithBuilds".to_string())?,
            builds: intermediate_rep
                .builds
                .into_iter()
                .next()
                .ok_or_else(|| "builds missing in TemplateWithBuilds".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<TemplateWithBuilds> and HeaderValue

#[cfg(feature = "server")]
impl std::convert::TryFrom<header::IntoHeaderValue<TemplateWithBuilds>> for HeaderValue {
    type Error = String;

    fn try_from(
        hdr_value: header::IntoHeaderValue<TemplateWithBuilds>,
    ) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match HeaderValue::from_str(&hdr_value) {
            std::result::Result::Ok(value) => std::result::Result::Ok(value),
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Invalid header value for TemplateWithBuilds - value: {hdr_value} is invalid {e}"#
            )),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::TryFrom<HeaderValue> for header::IntoHeaderValue<TemplateWithBuilds> {
    type Error = String;

    fn try_from(hdr_value: HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
            std::result::Result::Ok(value) => {
                match <TemplateWithBuilds as std::str::FromStr>::from_str(value) {
                    std::result::Result::Ok(value) => {
                        std::result::Result::Ok(header::IntoHeaderValue(value))
                    }
                    std::result::Result::Err(err) => std::result::Result::Err(format!(
                        r#"Unable to convert header value '{value}' into TemplateWithBuilds - {err}"#
                    )),
                }
            }
            std::result::Result::Err(e) => std::result::Result::Err(format!(
                r#"Unable to convert header: {hdr_value:?} to string: {e}"#
            )),
        }
    }
}
