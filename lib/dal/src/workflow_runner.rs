use serde::{Deserialize, Serialize};
use si_data::{NatsError, PgError};
use std::default::Default;
use telemetry::prelude::*;
use thiserror::Error;
use veritech::{FunctionResult, FunctionResultFailure};

use crate::workflow_runner::workflow_runner_state::WorkflowRunnerState;
use crate::workflow_runner::workflow_runner_state::WorkflowRunnerStatus;
use crate::{
    func::binding_return_value::{FuncBindingReturnValue, FuncBindingReturnValueError},
    func::execution::{FuncExecution, FuncExecutionError},
    func::{binding::FuncBindingId, FuncId},
    impl_standard_model, pk,
    standard_model::{self, objects_from_rows},
    standard_model_accessor, ChangeSetPk, ComponentId, Func, FuncBinding, FuncBindingError,
    HistoryEventError, SchemaId, SchemaVariantId, StandardModel, StandardModelError, SystemId,
    Timestamp, Visibility, WorkflowError, WorkflowPrototype, WorkflowPrototypeError,
    WorkflowPrototypeId, WorkflowResolverError, WorkflowResolverId, WriteTenancy, WsEvent,
    WsEventError, WsPayload,
};
use crate::{DalContext, FuncBackendError};

pub mod workflow_runner_state;

#[derive(Error, Debug)]
pub enum WorkflowRunnerError {
    #[error(transparent)]
    WsEvent(#[from] WsEventError),
    #[error(transparent)]
    Workflow(#[from] WorkflowError),
    #[error(transparent)]
    FuncBindingReturnValue(#[from] FuncBindingReturnValueError),
    #[error(transparent)]
    FuncExecution(#[from] FuncExecutionError),
    #[error("error serializing/deserializing json: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("pg error: {0}")]
    Pg(#[from] PgError),
    #[error("nats txn error: {0}")]
    Nats(#[from] NatsError),
    #[error("history event error: {0}")]
    HistoryEvent(#[from] HistoryEventError),
    #[error("standard model error: {0}")]
    StandardModel(#[from] StandardModelError),
    #[error(transparent)]
    WorkflowResolver(#[from] WorkflowResolverError),
    #[error(transparent)]
    WorkflowPrototype(#[from] WorkflowPrototypeError),
    #[error(transparent)]
    FuncBinding(#[from] FuncBindingError),
    #[error("prototype not found {0}")]
    PrototypeNotFound(WorkflowPrototypeId),
    #[error("missing workflow {0}")]
    MissingWorkflow(String),
}

pub type WorkflowRunnerResult<T> = Result<T, WorkflowRunnerError>;

const FIND_FOR_PROTOTYPE: &str = include_str!("./queries/workflow_runner_find_for_prototype.sql");

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunnerContext {
    component_id: ComponentId,
    schema_id: SchemaId,
    schema_variant_id: SchemaVariantId,
    system_id: SystemId,
}

// Hrm - is this a universal runner context? -- Adam
impl Default for WorkflowRunnerContext {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowRunnerContext {
    pub fn new() -> Self {
        WorkflowRunnerContext {
            component_id: ComponentId::NONE,
            schema_id: SchemaId::NONE,
            schema_variant_id: SchemaVariantId::NONE,
            system_id: SystemId::NONE,
        }
    }

    pub fn component_id(&self) -> ComponentId {
        self.component_id
    }

    pub fn set_component_id(&mut self, component_id: ComponentId) {
        self.component_id = component_id;
    }

    pub fn schema_id(&self) -> SchemaId {
        self.schema_id
    }

    pub fn set_schema_id(&mut self, schema_id: SchemaId) {
        self.schema_id = schema_id;
    }

    pub fn schema_variant_id(&self) -> SchemaVariantId {
        self.schema_variant_id
    }

    pub fn set_schema_variant_id(&mut self, schema_variant_id: SchemaVariantId) {
        self.schema_variant_id = schema_variant_id;
    }

    pub fn system_id(&self) -> SystemId {
        self.system_id
    }

    pub fn set_system_id(&mut self, system_id: SystemId) {
        self.system_id = system_id;
    }
}

pk!(WorkflowRunnerPk);
pk!(WorkflowRunnerId);

// An WorkflowRunner joins a `FuncBinding` to the context in which
// its corresponding `FuncBindingResultValue` is consumed.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunner {
    pk: WorkflowRunnerPk,
    id: WorkflowRunnerId,
    workflow_prototype_id: WorkflowPrototypeId,
    workflow_resolver_id: WorkflowResolverId,
    func_id: FuncId,
    func_binding_id: FuncBindingId,
    #[serde(flatten)]
    context: WorkflowRunnerContext,
    #[serde(flatten)]
    tenancy: WriteTenancy,
    #[serde(flatten)]
    timestamp: Timestamp,
    #[serde(flatten)]
    visibility: Visibility,
}

impl_standard_model! {
    model: WorkflowRunner,
    pk: WorkflowRunnerPk,
    id: WorkflowRunnerId,
    table_name: "workflow_runners",
    history_event_label_base: "workflow_runner",
    history_event_message_name: "Workflow Runner"
}

impl WorkflowRunner {
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    pub async fn new(
        ctx: &DalContext<'_, '_, '_>,
        workflow_prototype_id: WorkflowPrototypeId,
        workflow_resolver_id: WorkflowResolverId,
        func_id: FuncId,
        func_binding_id: FuncBindingId,
        context: WorkflowRunnerContext,
    ) -> WorkflowRunnerResult<Self> {
        let row = ctx.txns().pg().query_one(
            "SELECT object FROM workflow_runner_create_v1($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            &[
                ctx.write_tenancy(),
                ctx.visibility(),
                &workflow_prototype_id,
                &workflow_resolver_id,
                &func_id,
                &func_binding_id,
                &context.component_id(),
                &context.schema_id(),
                &context.schema_variant_id(),
                &context.system_id(),
            ],
        )
            .await?;
        let object = standard_model::finish_create_from_row(ctx, row).await?;
        WsEvent::new(ctx, WsPayload::ChangeSetApplied(ChangeSetPk::NONE))
            .publish(ctx)
            .await?;
        Ok(object)
    }

    /// Create a [`WorkflowRunner`](Self) and "run" it immediately. This not only creates the
    /// runner, but also a corresponding, _terminating_
    /// [`WorkflowRunnerState`](crate::workflow_runner::workflow_runner_state::WorkflowRunnerState).
    pub async fn run(
        ctx: &DalContext<'_, '_, '_>,
        prototype_id: WorkflowPrototypeId,
    ) -> WorkflowRunnerResult<(Self, WorkflowRunnerState, Vec<FuncBindingReturnValue>)> {
        let prototype = WorkflowPrototype::get_by_id(ctx, &prototype_id)
            .await?
            .ok_or(WorkflowRunnerError::PrototypeNotFound(prototype_id))?;

        // NOTE(nick,wendy): it seems like we already generate the tree inside
        // "prototype.resolve()". What is the value of doing this in two steps? It is possible
        // this is better for multiple runs because "prototype.resolve()" will only return the
        // resolver rather than generate a tree and build the resolver. Maybe, we could return
        // a boolean called "created" and only run "resolver.tree()" if "created" is false?
        let resolver = prototype.resolve(ctx).await?;
        let tree = resolver.tree(ctx).await?;

        // Perform the workflow runner "run" by running the workflow tree.
        let (func_binding_return_values, func_id, func_binding_id, maybe_failure): (
            Vec<FuncBindingReturnValue>,
            FuncId,
            FuncBindingId,
            Option<FunctionResultFailure>,
        ) = match tree.run(ctx).await {
            Ok(func_binding_return_values) => {
                let (func_id, func_binding_id) =
                    Self::process_successful_workflow_run(ctx, &func_binding_return_values).await?;
                (func_binding_return_values, func_id, func_binding_id, None)
            }
            Err(maybe_failure) => {
                // The only kind of error we should "allow" is a function failure. All other errors
                // will be returned.
                if let WorkflowError::FuncBinding(FuncBindingError::FuncBackend(
                    FuncBackendError::FunctionResultCommandRun(FunctionResult::Failure(failure)),
                )) = &maybe_failure
                {
                    // We need to collect the FuncId if the workflow was not successful.
                    let identity = Func::find_by_attr(ctx, "name", &"si:identity")
                        .await?
                        .pop()
                        .ok_or_else(|| {
                            WorkflowRunnerError::MissingWorkflow("si:identity".to_owned())
                        })?;
                    (
                        Vec::new(),
                        *identity.id(),
                        FuncBindingId::NONE,
                        Some(failure.clone()),
                    )
                } else {
                    return Err(WorkflowRunnerError::Workflow(maybe_failure));
                }
            }
        };

        // Destructure the underlying function failure, if a function failure occurred.
        let (workflow_runner_status, execution_id, error_kind, error_message) = match maybe_failure
        {
            Some(failure) => (
                WorkflowRunnerStatus::Failure,
                Some(failure.execution_id),
                Some(failure.error.kind),
                Some(failure.error.message),
            ),
            None => (WorkflowRunnerStatus::Success, None, None, None),
        };

        let mut context = WorkflowRunnerContext::new();
        context.set_component_id(prototype.context().component_id);
        context.set_schema_id(prototype.context().schema_id);
        context.set_schema_variant_id(prototype.context().schema_variant_id);
        context.set_system_id(prototype.context().system_id);

        // TODO(nick,wendy,paulo): create the runner independent of it being ran (either at the
        // beginning of this function or outside of it).
        let runner = Self::new(
            ctx,
            *prototype.id(),
            *resolver.id(),
            func_id,
            func_binding_id,
            context,
        )
        .await?;

        let runner_state = WorkflowRunnerState::new(
            ctx,
            *runner.id(),
            workflow_runner_status,
            execution_id,
            error_kind,
            error_message,
        )
        .await?;

        Ok((runner, runner_state, func_binding_return_values))
    }

    /// Upon a successful workflow runner "run" (within [`Self::run()`]), process the result
    /// as desired (e.g. processing logs).
    async fn process_successful_workflow_run(
        ctx: &DalContext<'_, '_, '_>,
        func_binding_return_values: &Vec<FuncBindingReturnValue>,
    ) -> WorkflowRunnerResult<(FuncId, FuncBindingId)> {
        let identity = Func::find_by_attr(ctx, "name", &"si:identity")
            .await?
            .pop()
            .ok_or_else(|| WorkflowRunnerError::MissingWorkflow("si:identity".to_owned()))?;

        let (func_binding, mut func_binding_return_value) = FuncBinding::find_or_create_and_execute(
            ctx,
            serde_json::json!({ "identity": serde_json::to_value(func_binding_return_values)? }),
            *identity.id(),
        )
            .await?;

        let mut logs = Vec::new();
        for return_value in func_binding_return_values {
            for stream in return_value
                .get_output_stream(ctx)
                .await?
                .unwrap_or_default()
            {
                logs.push(stream);
            }
        }
        logs.sort_by_key(|log| log.timestamp);
        if func_binding_return_value.func_execution_pk().is_none() {
            let pk = FuncExecution::new(ctx, &identity, &func_binding)
                .await?
                .pk();
            func_binding_return_value
                .set_func_execution_pk(ctx, pk)
                .await?;
        }

        let mut func_execution =
            FuncExecution::get_by_pk(ctx, &func_binding_return_value.func_execution_pk()).await?;
        func_execution.set_output_stream(ctx, logs).await?;

        Ok((*identity.id(), *func_binding.id()))
    }

    standard_model_accessor!(
        workflow_prototype_id,
        Pk(WorkflowPrototypeId),
        WorkflowRunnerResult
    );
    standard_model_accessor!(func_id, Pk(FuncId), WorkflowRunnerResult);
    standard_model_accessor!(func_binding_id, Pk(FuncBindingId), WorkflowRunnerResult);

    pub async fn find_for_prototype(
        ctx: &DalContext<'_, '_, '_>,
        workflow_prototype_id: &WorkflowPrototypeId,
        context: WorkflowRunnerContext,
    ) -> WorkflowRunnerResult<Vec<Self>> {
        let rows = ctx
            .txns()
            .pg()
            .query(
                FIND_FOR_PROTOTYPE,
                &[
                    ctx.read_tenancy(),
                    ctx.visibility(),
                    workflow_prototype_id,
                    &context.component_id(),
                    &context.schema_id(),
                    &context.schema_variant_id(),
                    &context.system_id(),
                ],
            )
            .await?;
        let object = objects_from_rows(rows)?;
        Ok(object)
    }
}

#[cfg(test)]
mod test {
    use super::WorkflowRunnerContext;

    #[test]
    fn context_builder() {
        let mut c = WorkflowRunnerContext::new();
        c.set_component_id(15.into());
        assert_eq!(c.component_id(), 15.into());
    }
}
