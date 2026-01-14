use std::sync::Arc;
use salvo::prelude::*;
use crate::AppState;

#[derive(Clone)]
pub struct StateHoop {
    pub state: Arc<AppState>,
}

#[salvo::async_trait]
impl Handler for StateHoop {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        depot.insert("state", self.state.clone());
        ctrl.call_next(req, depot, res).await;
    }
}