use salvo::prelude::*;

use crate::AppResult;

#[handler]
pub async fn index(res: &mut Response) -> AppResult<()> {
    res.render("Hello world");
    Ok(())
}
