use axum::{routing::get, Router};

use crate::server::state::AppState;
pub mod func;
pub mod variant;

pub fn routes() -> Router<AppState> {
    const PREFIX: &str = "/workspaces/:workspace_id/change-sets/:change_set_id";

    Router::new()
        .route(
            &format!("{PREFIX}/schema-variants"),
            get(variant::list_schema_variants),
        )
        .route(
            &format!("{PREFIX}/schema-variants/:schema_variant_id"),
            get(variant::get_variant),
        )
}

// pub async fn list_schema_variants(
//     HandlerContext(builder): HandlerContext,
//     AccessBuilder(access_builder): AccessBuilder,
//     PosthogClient(posthog_client): PosthogClient,
//     OriginalUri(original_uri): OriginalUri,
//     Path((_workspace_pk, change_set_id)): Path<(WorkspacePk, ChangeSetId)>,
// ) -> Result<Json<Vec<frontend_types::SchemaVariant>>, SchemaVariantsAPIError> {
//     let ctx = builder
//         .build(access_builder.build(change_set_id.into()))
//         .await?;

//     let mut schema_variants = Vec::new();

//     // NOTE(victor): This is not optimized, since it loops twice through the defaults, but it'll get the job done for now
//     // determining the default should change soon, and then we can get rid of SchemaVariant::get_default_for_schema over here
//     for schema_id in Schema::list_ids(&ctx).await? {
//         let default_schema_variant = SchemaVariant::get_default_for_schema(&ctx, schema_id).await?;
//         if !default_schema_variant.ui_hidden() {
//             schema_variants.push(
//                 default_schema_variant
//                     .into_frontend_type(&ctx, schema_id)
//                     .await?,
//             )
//         }

//         if let Some(unlocked) = SchemaVariant::get_unlocked_for_schema(&ctx, schema_id).await? {
//             if !unlocked.ui_hidden() {
//                 schema_variants.push(unlocked.into_frontend_type(&ctx, schema_id).await?)
//             }
//         }
//     }

//     track(
//         &posthog_client,
//         &ctx,
//         &original_uri,
//         "list_schema_variants",
//         serde_json::json!({}),
//     );

//     Ok(Json(schema_variants))
// }

// pub async fn get_variant(
//     HandlerContext(builder): HandlerContext,
//     AccessBuilder(access_builder): AccessBuilder,
//     PosthogClient(posthog_client): PosthogClient,
//     OriginalUri(original_uri): OriginalUri,
//     Path((_workspace_pk, change_set_id, schema_variant_id)): Path<(
//         WorkspacePk,
//         ChangeSetId,
//         SchemaVariantId,
//     )>,
// ) -> Result<Json<frontend_types::SchemaVariant>, SchemaVariantsAPIError> {
//     let ctx = builder
//         .build(access_builder.build(change_set_id.into()))
//         .await?;

//     let schema_variant = SchemaVariant::get_by_id(&ctx, schema_variant_id).await?;
//     let schema_id = SchemaVariant::schema_id_for_schema_variant_id(&ctx, schema_variant_id).await?;
//     let schema_variant = schema_variant.into_frontend_type(&ctx, schema_id).await?;

//     // Ported from `lib/sdf-server/src/server/service/variant/get_variant.rs`, so changes may be
//     // desired here...

//     track(
//         &posthog_client,
//         &ctx,
//         &original_uri,
//         "get_variant",
//         serde_json::json!({
//                     "schema_name": &schema_variant.schema_name,
//                     "variant_category": &schema_variant.category,
//                     "variant_menu_name": schema_variant.display_name,
//                     "variant_id": schema_variant.schema_variant_id,
//                     "schema_id": schema_variant.schema_id,
//                     "variant_component_type": schema_variant.component_type,
//         }),
//     );

//     Ok(Json(schema_variant))
// }

// #[remain::sorted]
// #[derive(Debug, Error)]
// pub enum SchemaVariantsAPIError {
//     #[error("schema error: {0}")]
//     Schema(#[from] dal::SchemaError),
//     #[error("schema error: {0}")]
//     SchemaVariant(#[from] dal::SchemaVariantError),
//     #[error("transactions error: {0}")]
//     Transactions(#[from] dal::TransactionsError),
// }

// impl IntoResponse for SchemaVariantsAPIError {
//     fn into_response(self) -> Response {
//         let status_code = match &self {
//             Self::Transactions(dal::TransactionsError::BadWorkspaceAndChangeSet) => {
//                 StatusCode::FORBIDDEN
//             }
//             // When a graph node cannot be found for a schema variant, it is not found
//             Self::SchemaVariant(dal::SchemaVariantError::NotFound(_)) => StatusCode::NOT_FOUND,
//             _ => ApiError::DEFAULT_ERROR_STATUS_CODE,
//         };

//         ApiError::new(status_code, self).into_response()
//     }
// }
