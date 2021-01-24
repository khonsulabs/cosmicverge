use crate::strings::localize;
use yew::prelude::*;
use yew_bulma::static_page::StaticPage;
use yew_router::prelude::*;

#[derive(Switch, Clone, Debug)]
pub enum AppRoute {
    // #[to = "/login!"]
    // LogIn,
    // #[to = "/backoffice/users"]
    // #[rest]
    // BackOfficeUserEdit(EditingId),
    // #[to = "/backoffice/users!"]
    // BackOfficeUsersList,
    // #[to = "/backoffice/roles/{id}/permissions"]
    // #[rest]
    // BackOfficeRolePermissionStatementEdit(i64, EditingId),
    // #[to = "/backoffice/roles"]
    // #[rest]
    // BackOfficeRoleEdit(EditingId),
    // #[to = "/backoffice/roles!"]
    // BackOfficeRolesList,
    // #[to = "/backoffice!"]
    // BackOfficeDashboard,
    #[to = "/!"]
    Index,
    #[to = "/"]
    NotFound,
}

impl AppRoute {
    pub fn render(&self, set_title: Callback<String>) -> Html {
        match self {
            AppRoute::Index => {
                html! {<StaticPage title="Welcome" content=localize("home-page") set_title=set_title.clone() />}
            }
            AppRoute::NotFound => {
                html! {<StaticPage title="Not Found" content=localize("not-found") set_title=set_title.clone() />}
            }
        }
    }
}
