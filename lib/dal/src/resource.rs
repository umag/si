use crate::DalContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use si_data::PgError;
use strum_macros::Display;
use telemetry::prelude::*;
use thiserror::Error;

use crate::{
    impl_standard_model, pk, standard_model, standard_model_accessor, standard_model_belongs_to,
    ws_event::{WsEvent, WsPayload},
    Component, ComponentId, HistoryEventError, StandardModel, StandardModelError, System, SystemId,
    Timestamp, Visibility, WorkflowPrototype, WorkflowPrototypeError, WorkflowPrototypeId,
    WorkflowRunner, WorkflowRunnerError, WriteTenancy, WsEventError,
};

#[derive(Error, Debug)]
pub enum ResourceError {
    #[error(transparent)]
    HistoryEvent(#[from] HistoryEventError),
    #[error(transparent)]
    Pg(#[from] PgError),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    WsEvent(#[from] WsEventError),
    #[error(transparent)]
    StandardModel(#[from] StandardModelError),
    #[error(transparent)]
    WorkflowPrototype(#[from] WorkflowPrototypeError),
    #[error(transparent)]
    WorkflowRunner(#[from] WorkflowRunnerError),
    #[error("system id is required: -1 was used")]
    SystemIdRequired,
    #[error("no component set for resource {0}")]
    NoComponent(ResourceId),
    #[error("prototype not found {0}")]
    PrototypeNotFound(WorkflowPrototypeId),
}

pub type ResourceResult<T> = Result<T, ResourceError>;

const FIND_BY_KEY: &str = include_str!("./queries/resource_find_by_key.sql");
const LIST_BY_COMPONENT_AND_SYSTEM: &str =
    include_str!("./queries/resource_list_by_component_and_system.sql");

pk!(ResourcePk);
pk!(ResourceId);

impl From<Resource> for veritech::ResourceView {
    fn from(res: Resource) -> Self {
        Self {
            key: res.key,
            data: res.data,
        }
    }
}

/// A Resource is the "real-world" representation of a specific [`Component`],
/// as it exists in the world, where the [`Component`] is the representation of
/// what we think it should look like.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pk: ResourcePk,
    id: ResourceId,
    key: String,
    data: serde_json::Value,
    refresh_workflow_id: WorkflowPrototypeId,
    #[serde(flatten)]
    tenancy: WriteTenancy,
    #[serde(flatten)]
    timestamp: Timestamp,
    #[serde(flatten)]
    visibility: Visibility,
}

impl_standard_model! {
    model: Resource,
    pk: ResourcePk,
    id: ResourceId,
    table_name: "resources",
    history_event_label_base: "resource",
    history_event_message_name: "Resource"
}

impl Resource {
    /// For a [`Resource`] to be uniquely identified, we need to know both
    /// which [`Component`] it is the "real world" representation of, and also
    /// the [`System`] in which that component being referred to.
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    pub async fn new(
        ctx: &DalContext,
        component_id: ComponentId,
        system_id: SystemId,
        key: String,
        data: serde_json::Value,
        refresh_workflow_id: WorkflowPrototypeId,
    ) -> ResourceResult<Self> {
        let row = ctx
            .txns()
            .pg()
            .query_one(
                "SELECT object FROM resource_create_v1($1, $2, $3, $4, $5)",
                &[
                    ctx.write_tenancy(),
                    ctx.visibility(),
                    &key,
                    &data,
                    &refresh_workflow_id,
                ],
            )
            .await?;
        let object: Self = standard_model::finish_create_from_row(ctx, row).await?;

        object.set_component(ctx, &component_id).await?;
        if system_id.is_some() {
            object.set_system(ctx, &system_id).await?;
        }

        Ok(object)
    }

    standard_model_accessor!(key, String, ResourceResult);
    standard_model_accessor!(data, Json<JsonValue>, ResourceResult);
    standard_model_accessor!(refresh_workflow_id, Pk(WorkflowPrototypeId), ResourceResult);

    standard_model_belongs_to!(
        lookup_fn: component,
        set_fn: set_component,
        unset_fn: unset_component,
        table: "resource_belongs_to_component",
        model_table: "components",
        belongs_to_id: ComponentId,
        returns: Component,
        result: ResourceResult,
    );

    standard_model_belongs_to!(
        lookup_fn: system,
        set_fn: set_system,
        unset_fn: unset_system,
        table: "resource_belongs_to_system",
        model_table: "systems",
        belongs_to_id: SystemId,
        returns: System,
        result: ResourceResult,
    );

    pub async fn upsert(
        ctx: &DalContext,
        component_id: ComponentId,
        system_id: SystemId,
        key: String,
        data: serde_json::Value,
        refresh_workflow_id: WorkflowPrototypeId,
    ) -> ResourceResult<Self> {
        let resource = Self::find_by_key(ctx, component_id, &key).await?;
        if let Some(mut resource) = resource {
            resource.set_data(ctx, data).await?;
            resource
                .set_refresh_workflow_id(ctx, refresh_workflow_id)
                .await?;
            Ok(resource)
        } else {
            Ok(Self::new(ctx, component_id, system_id, key, data, refresh_workflow_id).await?)
        }
    }

    pub async fn find_by_key(
        ctx: &DalContext,
        component_id: ComponentId,
        key: &str,
    ) -> ResourceResult<Option<Self>> {
        let row = ctx
            .txns()
            .pg()
            .query_opt(
                FIND_BY_KEY,
                &[ctx.read_tenancy(), ctx.visibility(), &component_id, &key],
            )
            .await?;
        let objects = standard_model::option_object_from_row(row)?;
        Ok(objects)
    }

    pub async fn refresh(&mut self, ctx: &DalContext) -> ResourceResult<()> {
        let component = self
            .component(ctx)
            .await?
            .ok_or(ResourceError::NoComponent(self.id))?;
        let prototype = WorkflowPrototype::get_by_id(ctx, &self.refresh_workflow_id)
            .await?
            .ok_or(ResourceError::PrototypeNotFound(self.refresh_workflow_id))?;
        let run_id: usize = rand::random();
        let (_runner, _state, _func_binding_return_values, _created_resources, _updated_resources) =
            WorkflowRunner::run(ctx, run_id, *prototype.id(), *component.id()).await?;

        let system_id = self
            .system(ctx)
            .await?
            .map_or(SystemId::NONE, |system| *system.id());
        WsEvent::resource_refreshed(ctx, *component.id(), system_id)
            .publish(ctx)
            .await?;
        Ok(())
    }

    pub async fn list_by_component(
        ctx: &DalContext,
        component_id: ComponentId,
        system_id: SystemId,
    ) -> ResourceResult<Vec<Self>> {
        let rows = ctx
            .txns()
            .pg()
            .query(
                LIST_BY_COMPONENT_AND_SYSTEM,
                &[
                    ctx.read_tenancy(),
                    ctx.visibility(),
                    &component_id,
                    &system_id,
                ],
            )
            .await?;
        let objects = standard_model::objects_from_rows(rows)?;
        Ok(objects)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Display, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum ResourceHealth {
    Ok,
    Warning,
    Error,
    Unknown,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceView {
    pub id: ResourceId,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub error: Option<String>,
    pub key: String,
    pub data: serde_json::Value,
    pub health: ResourceHealth,
    pub entity_type: String,
}

impl ResourceView {
    pub fn new(resource: Resource) -> Self {
        // TODO: actually fill all of the data dynamically, most fields don't make much sense for now

        Self {
            id: *resource.id(),
            created_at: resource.timestamp().created_at,
            updated_at: resource.timestamp().updated_at,
            error: None,
            key: resource.key,
            data: resource.data,
            health: ResourceHealth::Error,
            entity_type: "idk bro".to_owned(),
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRefreshId {
    component_id: ComponentId,
    system_id: SystemId,
}

impl WsEvent {
    pub fn resource_refreshed(
        ctx: &DalContext,
        component_id: ComponentId,
        system_id: SystemId,
    ) -> Self {
        WsEvent::new(
            ctx,
            WsPayload::ResourceRefreshed(ResourceRefreshId {
                component_id,
                system_id,
            }),
        )
    }
}
