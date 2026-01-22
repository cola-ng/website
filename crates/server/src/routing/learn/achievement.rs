use diesel::prelude::*;
use salvo::prelude::*;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{AppResult, DepotExt};

#[handler]
pub async fn list_achievements(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let achievements: Vec<UserAchievement> = with_conn(move |conn| {
        learn_achievements::table
            .filter(learn_achievements::user_id.eq(user_id))
            .order(learn_achievements::earned_at.desc())
            .load::<UserAchievement>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list achievements"))?;

    res.render(Json(achievements));
    Ok(())
}
