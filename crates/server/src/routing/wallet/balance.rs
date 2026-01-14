use bigdecimal::{BigDecimal, Zero};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use salvo::oapi::extract::*;
use salvo::prelude::*;

use crate::models::wallet::*;
use crate::models::*;
use crate::permission::Accessible;
use crate::db::schema::*;
use crate::{AppError, AppResult, DepotExt, PagedResult, StatusInfo, db};

#[endpoint(tags("wallet"))]
pub async fn list(req: &mut Request, depot: &mut Depot) -> PagedResult<Balance> {
    let _cuser = depot.current_user()?.must_in_kernel()?;
    let conn = &mut db::connect()?;
    let data = query_pagation_data!(
        req,
        res,
        Balance,
        wallet_balances::table, //.permit(cuser, "view")?,
        "created_at desc",
        BALANCE_FILTER_FIELDS.clone(),
        BALANCE_JOINED_OPTIONS.clone(),
        ID_NAME_SEARCH_TMPL,
        conn
    );
    Ok(Json(data))
}

#[derive(Deserialize, ToSchema, Debug)]
#[salvo(schema(inline))]
struct CreateBalanceInData {
    realm_id: i64,
    #[salvo(schema(value_type = String))]
    current_amount: BigDecimal,
    #[salvo(schema(value_type = String))]
    initial_amount: BigDecimal,
    parent_id: Option<i64>,
    payment_id: Option<i64>,
    payment_intent_id: Option<i64>,
    stripe_intent_id: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    description: String,
}
#[endpoint(tags("wallet"))]
pub async fn create(pdata: JsonBody<CreateBalanceInData>, depot: &mut Depot) -> AppResult<StatusInfo> {
    let cuser = depot.current_user()?.must_in_kernel()?;
    let pdata = pdata.into_inner();
    if pdata.initial_amount <= 0.into() {
        return Err(StatusError::bad_request().brief("The initial_amount must be greater than 0.").into());
    }
    if pdata.current_amount <= 0.into() {
        return Err(StatusError::bad_request().brief("The current_amount must be greater than 0.").into());
    }
    let conn = &mut db::connect()?;
    realms::table
        .find(pdata.realm_id)
        .get_result::<Realm>(conn)?
        .assign_to(cuser, "wallet.balance_manage", conn)?;
    let stripe_intent_id = if let Some(payment_intent_id) = pdata.payment_intent_id {
        trade_payment_intents::table
            .find(payment_intent_id)
            .select(trade_payment_intents::stripe_intent_id)
            .first::<Option<String>>(conn)?
    } else {
        None
    };
    conn.transaction::<_, AppError, _>(|conn| {
        let balance = diesel::insert_into(wallet_balances::table)
            .values(&NewBalance {
                realm_id: pdata.realm_id,
                kind: "manage.balance",
                initial_amount: pdata.initial_amount.clone(),
                current_amount: pdata.current_amount.clone(),
                currency: "usd",
                parent_id: None,
                payment_id: pdata.payment_id,
                payment_intent_id: pdata.payment_intent_id,
                stripe_intent_id: stripe_intent_id.as_deref(),
                refund_id: None,
                refund_item_id: None,
                expires_at: pdata.expires_at,
                description: Some(&pdata.description),
                updated_by: Some(cuser.id),
                created_by: Some(cuser.id),
            })
            .get_result::<Balance>(conn)?;

        let _ = diesel::insert_into(action_histories::table)
            .values(NewActionHistory {
                entity: "wallet_balance",
                action: "create",
                record_id: balance.id,
                user_id: cuser.id,
                old_data: None,
                new_data: serde_json::to_value(&balance).ok(),
                keywords: vec![],
                description: Some("kernel user create balance"),
            })
            .execute(conn);

        diesel::insert_into(wallet_histories::table)
            .values(&NewHistory {
                realm_id: pdata.realm_id,
                kind: "manage.balance",
                payment_id: None,
                refund_id: None,
                balance_ids: vec![balance.id],
                happend_amount: pdata.current_amount.clone(),
                balance_change: pdata.current_amount.clone(),
                currency: "usd",

                created_by: Some(cuser.id),
                description: Some(&*pdata.description),
            })
            .execute(conn)?;
        // diesel::insert_into(wallet_balance_changes::table)
        //     .values(&NewBalanceChange {
        //         realm_id: pdata.realm_id,
        //         history_id,
        //         balance_id,
        //         payment_id: None,
        //         refund_id: None,
        //         initial_amount: change_amount.clone(),
        //         changed_amount: change_amount.clone(),
        //         currency: &change.currency,
        //     })
        //     .execute(conn)?;
        Ok(())
    })?;

    Ok(StatusInfo::done())
}

#[derive(AsChangeset, Deserialize, ToSchema, Debug)]
#[diesel(table_name = wallet_balances)]
#[salvo(schema(inline))]
struct UpdateBalanceInData {
    kind: Option<String>,
    #[salvo(schema(value_type = Option<String>))]
    current_amount: Option<BigDecimal>,
    #[salvo(schema(value_type = Option<String>))]
    initial_amount: Option<BigDecimal>,
    currency: Option<String>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    parent_id: Option<Option<i64>>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    payment_id: Option<Option<i64>>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    expires_at: Option<Option<DateTime<Utc>>>,

    #[serde(default, with = "::serde_with::rust::double_option")]
    description: Option<Option<String>>,
}
#[endpoint(tags("wallet"))]
pub async fn update(balance_id: PathParam<i64>, pdata: JsonBody<UpdateBalanceInData>, depot: &mut Depot) -> AppResult<StatusInfo> {
    let cuser = depot.current_user()?.must_in_kernel()?;
    let pdata = pdata.into_inner();
    if let Some(initial_amount) = &pdata.initial_amount {
        if initial_amount <= &BigDecimal::zero() {
            return Err(StatusError::bad_request().brief("The initial_amount must be greater than 0.").into());
        }
    }
    if let Some(current_amount) = &pdata.current_amount {
        if current_amount <= &BigDecimal::zero() {
            return Err(StatusError::bad_request().brief("The current_amount must be greater than 0.").into());
        }
    }
    let conn = &mut db::connect()?;
    let balance = wallet_balances::table.find(balance_id.into_inner()).first::<Balance>(conn)?;
    let _realm = realms::table
        .find(balance.realm_id)
        .get_result::<Realm>(conn)?
        .assign_to(cuser, "wallet.balance_manage", conn)?;
    conn.transaction::<_, AppError, _>(|conn| {
        let updated = diesel::update(&balance).set(&pdata).get_result::<Balance>(conn)?;
        let _ = diesel::insert_into(action_histories::table)
            .values(NewActionHistory {
                entity: "wallet_balance",
                action: "update",
                record_id: balance.id,
                user_id: cuser.id,
                old_data: serde_json::to_value(&balance).ok(),
                new_data: serde_json::to_value(&updated).ok(),
                keywords: vec![],
                description: Some("kernel user update balance"),
            })
            .execute(conn);
        // diesel::insert_into(wallet_histories::table)
        //     .values(&NewHistory {
        //         realm_id,
        //         kind: "balance.manage",
        //         payment_id: pdata.payment_id,
        //         refund_id: None,
        //         balance_ids: vec![balance_id],
        //         happend_amount: pdata.amount.clone(),
        //         balance_change: pdata.amount.clone(),
        //         currency: "usd",

        //         created_by: Some(cuser.id),
        //         description: Some(&*pdata.description),
        //     })
        //     .execute(conn)?;
        Ok(())
    })?;

    Ok(StatusInfo::done())
}
