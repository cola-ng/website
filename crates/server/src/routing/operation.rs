use diesel::prelude::*;
use salvo::oapi::extract::*;
use salvo::prelude::*;

use crate::models::*;
use crate::db::schema::*;
use crate::{JsonResult, db};

pub fn authed_root(path: impl Into<String>) -> Router {
    Router::with_path(path).push(Router::with_path("{operation_id:u64}").get(show))
}

#[endpoint(tags("operation"))]
pub async fn show(operation_id: PathParam<i64>) -> JsonResult<Operation> {
    let operation_id = operation_id.into_inner();
    if operation_id > 0 {
        let operation = operations::table.find(operation_id).first::<Operation>(&mut db::conn()?)?;
        Ok(Json(operation))
    } else {
        Err(StatusError::bad_request().brief("parse id param error").into())
    }
}
