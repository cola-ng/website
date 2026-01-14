pub mod stream {
    use diesel::prelude::*;
    use salvo::oapi::extract::*;
    use salvo::prelude::*;
    use serde::Serialize;

    use crate::models::interflow::*;
    use crate::permission::Accessible;
    use crate::db::schema::*;
    use crate::{AppResult, DepotExt, JsonResult, StatusInfo, db};

    #[endpoint(tags("account"))]
    pub fn watch(stream_id: PathParam<i64>, depot: &mut Depot) -> AppResult<StatusInfo> {
        let cuser = depot.current_user()?;
        let conn = &mut db::connect()?;
        let stream = interflow_streams::table
            .find(stream_id.into_inner())
            .first::<Stream>(conn)?
            .assign_to(cuser, "view", conn)?;
        stream.add_watcher(cuser.id, conn)?;
        Ok(StatusInfo::done())
    }
    #[endpoint(tags("account"))]
    pub fn unwatch(stream_id: PathParam<i64>, depot: &mut Depot) -> AppResult<StatusInfo> {
        let cuser = depot.current_user()?;
        let conn = &mut db::connect()?;
        diesel::delete(
            interflow_watchers::table
                .filter(interflow_watchers::user_id.eq(cuser.id))
                .filter(interflow_watchers::stream_id.eq(stream_id.into_inner())),
        )
        .execute(conn)?;
        Ok(StatusInfo::done())
    }

    #[derive(Serialize, ToSchema, Debug)]
    #[salvo(schema(inline))]
    struct HasWatchedOkData {
        has_watched: bool,
    }
    #[endpoint(tags("account"))]
    pub fn has_watched(stream_id: PathParam<i64>, depot: &mut Depot) -> JsonResult<HasWatchedOkData> {
        let cuser = depot.current_user()?;
        let conn = &mut db::connect()?;
        let query = interflow_watchers::table
            .filter(interflow_watchers::user_id.eq(cuser.id))
            .filter(interflow_watchers::stream_id.eq(stream_id.into_inner()));

        Ok(Json(HasWatchedOkData {
            has_watched: diesel_exists!(query, conn),
        }))
    }
    #[endpoint(tags("account"))]
    pub fn watched_ids(_req: &mut Request, depot: &mut Depot) -> JsonResult<Vec<i64>> {
        let cuser = depot.current_user()?;
        let conn = &mut db::connect()?;
        let stream_ids: Vec<i64> = interflow_watchers::table
            .filter(interflow_watchers::user_id.eq(cuser.id))
            .select(interflow_watchers::stream_id)
            .get_results(conn)
            .unwrap_or_default();
        Ok(Json(stream_ids))
    }
}
