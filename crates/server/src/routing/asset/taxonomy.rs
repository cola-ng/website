use diesel::prelude::*;
use salvo::prelude::*;

use crate::AppResult;
use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::asset::*;

#[handler]
pub async fn list_domains(_req: &mut Request, res: &mut Response) -> AppResult<()> {
    let domains: Vec<TaxonDomain> =
        with_conn(move |conn| taxon_domains::table.load::<TaxonDomain>(conn))
            .await
            .map_err(|_| StatusError::internal_server_error().brief("failed to list domains"))?;

    res.render(Json(domains));
    Ok(())
}

#[handler]
pub async fn list_categories(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let domain_id = req.query::<i64>("domain_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let categories: Vec<TaxonCategory> = with_conn(move |conn| {
        let mut query = taxon_categories::table.limit(limit).into_boxed();

        if let Some(did) = domain_id {
            query = query.filter(taxon_categories::domain_id.eq(did));
        }

        query.load::<TaxonCategory>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list categories"))?;

    res.render(Json(categories));
    Ok(())
}

#[handler]
pub async fn get_categories_by_domain(req: &mut Request, res: &mut Response) -> AppResult<()> {
    let domain_id: i64 = req
        .param::<i64>("domain_id")
        .ok_or_else(|| StatusError::bad_request().brief("missing domain_id"))?;

    let categories: Vec<TaxonCategory> = with_conn(move |conn| {
        taxon_categories::table
            .filter(taxon_categories::domain_id.eq(domain_id))
            .load::<TaxonCategory>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list categories"))?;

    res.render(Json(categories));
    Ok(())
}
