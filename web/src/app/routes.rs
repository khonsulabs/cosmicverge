use crate::{app::game::Game, localize, localize_html};
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
    #[to = "/game!"]
    Game,
    #[to = "/!"]
    Index,

    #[to = "/"]
    NotFound,
}

impl AppRoute {
    pub fn render(&self, set_title: Callback<String>) -> Html {
        match self {
            AppRoute::Game => {
                // Reveal the canvas
                html! { <Game set_title=set_title.clone() /> }
            }
            other => {
                html! {
                    <section class="section content">
                        <div class="columns is-centered">
                            <div class="column is-half">
                                <p class="notification is-danger is-light">
                                    { localize!("early-warning") }
                                </p>
                            </div>
                        </div>

                        { other.render_content(set_title) }
                    </section>
                }
            }
        }
    }

    fn render_content(&self, set_title: Callback<String>) -> Html {
        match self {
            AppRoute::Game => unreachable!(),
            AppRoute::Index => {
                html! {<p>{"This is the home page. Cool aint it?"}</p>}
            }
            AppRoute::NotFound => {
                html! {<StaticPage title="Not Found" content=localize_html!("not-found") set_title=set_title.clone() />}
            }
        }
    }
}
