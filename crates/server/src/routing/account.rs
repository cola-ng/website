use salvo::prelude::*;

use crate::hoops::require_auth;
use crate::routing::auth;

pub fn router() -> Router {
    Router::new()
}
