use chrono::{TimeDelta, Utc};
use diesel::prelude::*;
use serde::Serialize;
use serde_json::Value;
use ulid::Ulid;

use crate::models::trade::*;
use crate::models::wallet::*;
use crate::models::*;
use crate::schema::*;
use crate::utils::email::disable_track_img;
use crate::{AppResult, DeployStage, db};

#[derive(Serialize, Debug)]
struct CouponNotifyContext<'a> {
    recipient: &'a User,
    coupon: &'a Coupon,
    obtained_coupon: &'a ObtainedCoupon,
    track_flag: &'a str,
    coupon_url: &'a str,
    front_url: &'a str,
}

pub async fn supervise_job() {
    loop {
        if let Err(e) = supervise().await {
            tracing::error!( error = ?e, "notification supervise error")
        }
        if crate::deploy_stage() != DeployStage::Prod {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        } else {
            tokio::time::sleep(tokio::time::Duration::from_secs(8 * 3600)).await;
        }
    }
}
pub async fn supervise() -> AppResult<()> {
    let mut conn = db::connect()?;

    let obtained_coupons = if crate::deploy_stage() != DeployStage::Prod {
        wallet_obtained_coupons::table
            .filter(wallet_obtained_coupons::is_used.eq(false))
            .filter(wallet_obtained_coupons::expires_at.lt(Utc::now() + TimeDelta::try_days(15).unwrap()))
            .filter(wallet_obtained_coupons::expires_at.gt(Utc::now() + TimeDelta::try_days(14).unwrap()))
            .order_by(wallet_obtained_coupons::id.asc())
            .get_results::<ObtainedCoupon>(&mut conn)?
    } else {
        wallet_obtained_coupons::table
            .filter(wallet_obtained_coupons::is_used.eq(false))
            .filter(wallet_obtained_coupons::expires_at.lt(Utc::now() + TimeDelta::try_days(15).unwrap()))
            .filter(wallet_obtained_coupons::expires_at.gt(Utc::now() + TimeDelta::try_days(14).unwrap()))
            .order_by(wallet_obtained_coupons::id.asc())
            .get_results::<ObtainedCoupon>(&mut conn)?
    };

    for obtained_coupon in obtained_coupons {
        let Ok(coupon) = trade_coupons::table.find(obtained_coupon.coupon_id).first::<Coupon>(&mut conn) else {
            continue;
        };
        let Ok(recipient_ids) = realm_users::table
            .filter(realm_users::realm_id.eq(obtained_coupon.realm_id))
            .filter(realm_users::is_root.eq(true))
            .select(realm_users::user_id)
            .load::<i64>(&mut conn)
        else {
            continue;
        };
        let recipients = users::table
            .filter(users::id.eq_any(&recipient_ids))
            .filter(users::is_disabled.eq(false))
            .load::<User>(&mut conn)?;
        for recipient in recipients {
            let email_kind = format!("obtained_coupon:{}:expire_notify:{}", obtained_coupon.id, recipient.id);
            let query = email_messages::table.filter(email_messages::kind.eq(&email_kind));
            if diesel_exists!(query, &mut conn) {
                continue;
            }

            let Ok(email) = emails::table
                .filter(emails::user_id.eq(recipient.id))
                .filter(emails::is_master.eq(true))
                .first::<Email>(&mut conn)
            else {
                continue;
            };
            let track_flag = crate::utils::uuid_string();

            let front_url = if recipient.in_kernel {
                crate::manage_front_url()
            } else {
                crate::console_front_url()
            };
            let coupon_url = format!("{}/realms/{}/wallet", front_url, obtained_coupon.realm_id);
            let data = CouponNotifyContext {
                track_flag: &track_flag,
                recipient: &recipient,
                obtained_coupon: &obtained_coupon,
                coupon: &coupon,
                coupon_url: &coupon_url,
                front_url: &front_url,
            };
            let view_token = Ulid::new().to_string().to_lowercase();
            let html_body = crate::email::HANDLEBARS.render("wallet_obtained_coupon_will_expire", &data)?;
            let new_msg = NewEmailMessage {
                kind: &email_kind,
                thread_id: None,
                recipient_id: Some(recipient.id),
                recipient_email: &email.value,
                reply_token: None,
                view_token: &view_token,
                subject: "Your Coupon is About to Expire in 15 Days! Don’t Miss Out!",
                text_body: None,
                html_body: Some(disable_track_img(&html_body)),
                attachments: Value::Null,
                track_flag: Some(&*track_flag),
                sent_in: if crate::deploy_stage() != DeployStage::Prod {
                    Some(Utc::now() + TimeDelta::try_seconds(60).unwrap())
                } else {
                    Some(Utc::now() + TimeDelta::try_hours(1).unwrap())
                },
                sent_status: "delay",
                sent_error: None,
            };
            diesel::insert_into(email_messages::table).values(&new_msg).execute(&mut conn)?;
        }
    }
    drop(conn);
    Ok(())
}
