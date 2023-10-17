use base64::Engine;
use content_store::ContentHash;
use serde::{Deserialize, Serialize};
use std::string::FromUtf8Error;
use strum::{EnumDiscriminants, IntoEnumIterator};
use telemetry::prelude::*;
use thiserror::Error;
use veritech_client::CycloneValueEncryptError;

use crate::workspace_snapshot::content_address::ContentAddress;
use crate::{
    pk, FuncBackendKind, FuncBackendResponseType, HistoryEventError, StandardModel, Timestamp,
};

#[remain::sorted]
#[derive(Error, Debug)]
pub enum FuncError {
    #[error("cyclone value encrypt error: {0}")]
    CycloneValueEncrypt(#[from] CycloneValueEncryptError),
    #[error("error decoding code_base64: {0}")]
    Decode(#[from] base64::DecodeError),
    #[error("utf8 encoding error: {0}")]
    FromUtf8(#[from] FromUtf8Error),
    #[error("func binding error: {0}")]
    FuncBinding(String),
    #[error("history event error: {0}")]
    HistoryEvent(#[from] HistoryEventError),
    /// Could not find [`FuncArgument`](crate::FuncArgument) corresponding to the identity [`Func`].
    #[error("identity func argument not found")]
    IdentityFuncArgumentNotFound,
    /// Could not find the identity [`Func`].
    #[error("identity func not found")]
    IdentityFuncNotFound,
    #[error("intrinsic parse error: {0} is not an intrinsic")]
    IntrinsicParse(String),
    #[error("intrinsic spec creation error {0}")]
    IntrinsicSpecCreation(String),
}

pub type FuncResult<T> = Result<T, FuncError>;

// pub mod argument;
pub mod backend;
// pub before;
// pub mod binding;
// pub mod binding_return_value;
// pub mod execution;
// pub mod identity;
pub mod intrinsics;

pk!(FuncId);

/// A `Func` is the declaration of the existence of a function. It has a name,
/// and corresponds to a given function backend (and its associated return types).
///
/// `handler` is the name of the entry point into the code in `code_base64`.
/// For example, if we had a code block of
/// `function myValidator(actual, expected) { return true; }` in `code_base64`,
/// the `handler` value should be `myValidator`.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Func {
    pub id: FuncId,
    #[serde(flatten)]
    pub timestamp: Timestamp,
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub link: Option<String>,
    pub hidden: bool,
    pub builtin: bool,
    pub backend_kind: FuncBackendKind,
    pub backend_response_type: FuncBackendResponseType,
    pub handler: Option<String>,
    pub code_base64: Option<String>,
    pub code_sha256: String,
}

impl Func {
    pub fn assemble(id: FuncId, inner: &FuncContentV1) -> Self {
        let inner = inner.to_owned();
        Self {
            id,
            timestamp: inner.timestamp,
            name: inner.name,
            display_name: inner.display_name,
            description: inner.description,
            link: inner.link,
            hidden: inner.hidden,
            builtin: inner.builtin,
            backend_kind: inner.backend_kind,
            backend_response_type: inner.backend_response_type,
            handler: inner.handler,
            code_base64: inner.code_base64,
            code_sha256: inner.code_sha256,
        }
    }
}

impl From<Func> for FuncContentV1 {
    fn from(value: Func) -> Self {
        Self {
            timestamp: value.timestamp,
            name: value.name,
            display_name: value.display_name,
            description: value.description,
            link: value.link,
            hidden: value.hidden,
            builtin: value.builtin,
            backend_kind: value.backend_kind,
            backend_response_type: value.backend_response_type,
            handler: value.handler,
            code_base64: value.code_base64,
            code_sha256: value.code_sha256,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FuncGraphNode {
    id: FuncId,
    name: String,
    content_address: ContentAddress,
    content: FuncContentV1,
}

#[derive(EnumDiscriminants, Serialize, Deserialize, PartialEq)]
#[serde(tag = "version")]
pub enum FuncContent {
    V1(FuncContentV1),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct FuncContentV1 {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub link: Option<String>,
    pub hidden: bool,
    pub builtin: bool,
    pub backend_kind: FuncBackendKind,
    pub backend_response_type: FuncBackendResponseType,
    pub handler: Option<String>,
    pub code_base64: Option<String>,
    pub code_sha256: String,
}

impl FuncGraphNode {
    pub fn assemble(
        id: impl Into<FuncId>,
        name: impl Into<String>,
        content_hash: ContentHash,
        content: FuncContentV1,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            content_address: ContentAddress::Func(content_hash),
            content,
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FuncMetadataView {
    pub display_name: String,
    pub description: Option<String>,
    pub link: Option<String>,
}

pub fn is_intrinsic(name: &str) -> bool {
    intrinsics::IntrinsicFunc::iter().any(|intrinsic| intrinsic.name() == name)
}

// impl Func {
//     #[instrument(skip_all)]
//     pub async fn new(
//         ctx: &DalContext,
//         name: impl AsRef<str>,
//         backend_kind: FuncBackendKind,
//         backend_response_type: FuncBackendResponseType,
//     ) -> FuncResult<Self> {
//         let name = name.as_ref();
//         let row = ctx
//             .txns()
//             .await?
//             .pg()
//             .query_one(
//                 "SELECT object FROM func_create_v1($1, $2, $3, $4, $5)",
//                 &[
//                     ctx.tenancy(),
//                     ctx.visibility(),
//                     &name,
//                     &backend_kind.as_ref(),
//                     &backend_response_type.as_ref(),
//                 ],
//             )
//             .await?;
//         let object = standard_model::finish_create_from_row(ctx, row).await?;
//         Ok(object)
//     }

//     /// Creates a new [`Func`] from [`self`](Func). All relevant fields are duplicated, but rows
//     /// existing on relationship tables (e.g. "belongs_to" or "many_to_many") are not.
//     pub async fn duplicate(&self, ctx: &DalContext) -> FuncResult<Self> {
//         // Generate a unique name and make sure it's not in use
//         let mut new_unique_name;
//         loop {
//             new_unique_name = format!("{}{}", self.name(), generate_unique_id(4));
//             if Self::find_by_name(ctx, &new_unique_name).await?.is_none() {
//                 break;
//             };
//         }

//         let mut new_func = Self::new(
//             ctx,
//             new_unique_name,
//             *self.backend_kind(),
//             *self.backend_response_type(),
//         )
//         .await?;

//         // Duplicate all fields on the func that do not come in through the constructor.
//         new_func.set_display_name(ctx, self.display_name()).await?;
//         new_func.set_description(ctx, self.description()).await?;
//         new_func.set_link(ctx, self.link()).await?;
//         new_func.set_hidden(ctx, self.hidden).await?;
//         new_func.set_builtin(ctx, self.builtin).await?;
//         new_func.set_handler(ctx, self.handler()).await?;
//         new_func.set_code_base64(ctx, self.code_base64()).await?;

//         Ok(new_func)
//     }

//     #[allow(clippy::result_large_err)]
//     pub fn code_plaintext(&self) -> FuncResult<Option<String>> {
//         Ok(match self.code_base64() {
//             Some(base64_code) => Some(String::from_utf8(
//                 general_purpose::STANDARD_NO_PAD.decode(base64_code)?,
//             )?),
//             None => None,
//         })
//     }

//     pub async fn is_builtin(&self, ctx: &DalContext) -> FuncResult<bool> {
//         let row = ctx
//             .txns()
//             .await?
//             .pg()
//             .query_opt(
//                 "SELECT id FROM funcs WHERE id = $1 and tenancy_workspace_pk = $2 LIMIT 1",
//                 &[self.id(), &WorkspacePk::NONE],
//             )
//             .await?;

//         Ok(row.is_some())
//     }

//     pub async fn set_code_plaintext(
//         &mut self,
//         ctx: &DalContext,
//         code: Option<&'_ str>,
//     ) -> FuncResult<()> {
//         self.set_code_base64(
//             ctx,
//             code.as_ref()
//                 .map(|code| general_purpose::STANDARD_NO_PAD.encode(code)),
//         )
//         .await
//     }

//     pub fn metadata_view(&self) -> FuncMetadataView {
//         FuncMetadataView {
//             display_name: self.display_name().unwrap_or_else(|| self.name()).into(),
//             description: self.description().map(Into::into),
//             link: self.description().map(Into::into),
//         }
//     }

//     pub async fn for_binding(ctx: &DalContext, func_binding: &FuncBinding) -> FuncResult<Self> {
//         let row = ctx
//             .txns()
//             .await?
//             .pg()
//             .query_one(
//                 "SELECT row_to_json(funcs.*) AS object
//                 FROM funcs_v1($1, $2) AS funcs
//                 INNER JOIN func_binding_belongs_to_func_v1($1, $2) AS func_binding_belongs_to_func
//                     ON funcs.id = func_binding_belongs_to_func.belongs_to_id
//                 WHERE func_binding_belongs_to_func.object_id = $3",
//                 &[ctx.tenancy(), ctx.visibility(), func_binding.id()],
//             )
//             .await?;
//         let object = standard_model::finish_create_from_row(ctx, row).await?;
//         Ok(object)
//     }

//     pub async fn find_by_name(ctx: &DalContext, name: &str) -> FuncResult<Option<Self>> {
//         Ok(Self::find_by_attr(ctx, "name", &name).await?.pop())
//     }

//     /// Returns `true` if this function is one handled internally by the `dal`, `false` if the
//     /// function is one that will be executed by `veritech`
//     pub fn is_intrinsic(&self) -> bool {
//         is_intrinsic(self.name())
//     }

//     standard_model_accessor!(name, String, FuncResult);
//     standard_model_accessor!(display_name, Option<String>, FuncResult);
//     standard_model_accessor!(description, Option<String>, FuncResult);
//     standard_model_accessor!(link, Option<String>, FuncResult);
//     standard_model_accessor!(hidden, bool, FuncResult);
//     standard_model_accessor!(builtin, bool, FuncResult);
//     standard_model_accessor!(backend_kind, Enum(FuncBackendKind), FuncResult);
//     standard_model_accessor!(
//         backend_response_type,
//         Enum(FuncBackendResponseType),
//         FuncResult
//     );
//     standard_model_accessor!(handler, Option<String>, FuncResult);
//     standard_model_accessor!(code_base64, Option<String>, FuncResult);
//     standard_model_accessor_ro!(code_sha256, String);
// }
