//! This module contains helpers for dal test macro expansion.
//!
//! _Caution:_ functions in this module may appear to be unused, but they are likely used during
//! macro expansion.

use dal::{ChangeSet, ChangeSetId, DalContext, UserClaim};
use telemetry::tracing::subscriber;
use tracing_subscriber::{fmt, EnvFilter, Registry};

use crate::helpers::{create_auth_token, generate_fake_name};
use crate::WorkspaceSignup;

/// This function is used during macro expansion for setting up a [`ChangeSet`] in an integration test.
pub async fn create_change_set_and_update_ctx(
    ctx: &mut DalContext,
    base_change_set_id: ChangeSetId,
) {
    let base_change_set = ChangeSet::find(ctx, base_change_set_id)
        .await
        .expect("could not perform find change set")
        .expect("no change set found");
    let mut change_set = ChangeSet::new(ctx, generate_fake_name(), Some(base_change_set_id))
        .await
        .expect("could not create change set");
    change_set
        .update_pointer(
            ctx,
            base_change_set
                .workspace_snapshot_address
                .expect("no workspace snapshot set on base change set"),
        )
        .await
        .expect("could not update pointer");
    ctx.update_visibility_and_snapshot_to_visibility(change_set.id)
        .await
        .expect("could not update visibility and snapshot");
}

/// This function is used during macro expansion for setting up tracing in an integration test.
pub fn tracing_init(span_events_env_var: &'static str, log_env_var: &'static str) {
    use std::thread;

    thread::spawn(move || {
        let tokio = ::tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime for Tracing for Testing");

        tokio.block_on(async move {
            tracing_init_inner(span_events_env_var, log_env_var);
            tokio::time::sleep(std::time::Duration::from_secs(10000000)).await
        });
    });
}

fn tracing_init_inner(span_events_env_var: &str, log_env_var: &str) {
    use opentelemetry_sdk::runtime;
    use tracing_subscriber::layer::SubscriberExt;

    let event_filter = {
        use fmt::format::FmtSpan;

        // Used exclusively in tests & prefixed with `SI_TEST_`
        #[allow(clippy::disallowed_methods)]
        match ::std::env::var(span_events_env_var) {
            Ok(value) => value
                .to_ascii_lowercase()
                .split(',')
                .map(|filter| match filter.trim() {
                    "new" => FmtSpan::NEW,
                    "enter" => FmtSpan::ENTER,
                    "exit" => FmtSpan::EXIT,
                    "close" => FmtSpan::CLOSE,
                    "active" => FmtSpan::ACTIVE,
                    "full" => FmtSpan::FULL,
                    _ => panic!(
                        "{} must contain filters separated by `,`.\n\t\
                            For example: `active` or `new,close`\n\t
                            Got: {}",
                        span_events_env_var, value,
                    ),
                })
                .fold(FmtSpan::NONE, |acc, filter| filter | acc),
            Err(::std::env::VarError::NotUnicode(_)) => {
                panic!("{} must contain a valid UTF-8 string", span_events_env_var,)
            }
            Err(::std::env::VarError::NotPresent) => FmtSpan::NONE,
        }
    };

    let env_filter = EnvFilter::from_env(log_env_var);
    let format_layer = fmt::layer()
        .with_thread_ids(true)
        .with_span_events(event_filter)
        .with_test_writer()
        .pretty();

    let otel_tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .install_batch(runtime::Tokio)
        .expect("Creating otel_tracer failed");

    let otel_layer = tracing_opentelemetry::layer().with_tracer(otel_tracer);

    let registry = Registry::default();

    let registry = registry
        .with(env_filter)
        .with(format_layer)
        .with(otel_layer);

    subscriber::set_global_default(registry).expect("failed to register global default");
}

/// This function is used during macro expansion for setting up the workspace for integration tests.
pub async fn workspace_signup(ctx: &DalContext) -> crate::Result<(WorkspaceSignup, String)> {
    use color_eyre::eyre::WrapErr;

    let mut ctx = ctx.clone_with_head().await?;

    let workspace_name = generate_fake_name();
    let user_name = format!("frank {workspace_name}");
    let user_email = format!("{workspace_name}@example.com");

    let nw = WorkspaceSignup::new(&mut ctx, &workspace_name, &user_name, &user_email)
        .await
        .wrap_err("cannot signup a new workspace")?;
    let auth_token = create_auth_token(UserClaim {
        user_pk: nw.user.pk(),
        workspace_pk: *nw.workspace.pk(),
    })
    .await;
    Ok((nw, auth_token))
}