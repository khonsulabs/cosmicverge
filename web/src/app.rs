use yew::prelude::*;
use yew_bulma::static_page::StaticPage;
use yew_router::prelude::*;

use crate::{app::{game::Game}, localize, localize_html};

mod game;


#[derive(Switch, Clone, Debug, Eq, PartialEq)]
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

pub struct App {
    link: ComponentLink<Self>,
    rendering: bool,
    navbar_expanded: bool,
}

pub enum Message {
    SetTitle(String),
    ToggleNavbar,
    ToggleRendering,
}

fn set_document_title(title: &str) {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .set_title(title);
}

impl Component for App {
    type Message = Message;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        set_document_title(&localize!("cosmic-verge"));

        Self {
            link,
            rendering: true,
            navbar_expanded: false,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Message::SetTitle(title) => {
                set_document_title(&title);
                false
            }
            Message::ToggleNavbar => {
                self.navbar_expanded = !self.navbar_expanded;
                true
            }
            Message::ToggleRendering => {
                self.rendering = !self.rendering;
                true
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let link = self.link.clone();
        let rendering = self.rendering;
        let navbar_expanded = self.navbar_expanded;

        html! {
            <Router<AppRoute>
                render = Router::render(move |route: AppRoute| {
                    let app = AppRouteRenderer {
                        link: link.clone(),
                        route,
                        rendering,
                        navbar_expanded,
                    };
                    app.render()
                })
            />
        }
    }
}

struct AppRouteRenderer {
    route: AppRoute,
    link: ComponentLink<App>,
    rendering: bool,
    navbar_expanded: bool,
}

impl AppRouteRenderer {
    fn render(&self) -> Html {
        let set_title = self.link.callback(Message::SetTitle);


        let (game_foregrounded, contents) = match &self.route {
            AppRoute::Game => {
                // Reveal the canvas
                (true, Html::default())
            }
            other => {
                (false, html! {
                    <section class="section content">
                        <div class="columns is-centered">
                            <div class="column is-half">
                                <p class="notification is-danger is-light">
                                    { localize!("early-warning") }
                                </p>
                            </div>
                        </div>

                        { self.render_content(other) }
                    </section>
                })
            }
        };
        let app_class = if game_foregrounded {
            "in-game"
        } else {
            "out-of-game"
        };
        html! {
            <div>
                <div id="app" class=app_class>
                    { self.navbar() }
                    { contents }
                </div>
                <Game set_title=set_title.clone() foregrounded=game_foregrounded rendering=self.rendering />
            </div>
        }
    }

    fn render_content(&self, route: &AppRoute) -> Html {
        let set_title = self.link.callback(Message::SetTitle);
        match route {
            AppRoute::Game => unreachable!(),
            AppRoute::Index => {
                html! {<p>{"This is the home page. Cool aint it?"}</p>}
            }
            AppRoute::NotFound => {
                html! {<StaticPage title="Not Found" content=localize_html!("not-found") set_title=set_title.clone() />}
            }
        }
    }

    fn navbar(&self) -> Html {
        html! {
            <nav class=format!("navbar is-fixed-top {}", self.navbar_menu_expanded_class()) role="navigation" aria-label=localize!("navbar-label")>
                <div class="navbar-brand">
                    <RouterAnchor<AppRoute> classes="navbar-item" route=AppRoute::Index>
                        { localize!("cosmic-verge") }
                    </RouterAnchor<AppRoute>>

                    <a role="button" class="navbar-burger" aria-label=localize!("navbar-menu-label") aria-expanded=self.navbar_expanded data-target="navbar-contents" onclick=self.link.callback(|_| Message::ToggleNavbar)>
                        <span aria-hidden="true"></span>
                        <span aria-hidden="true"></span>
                        <span aria-hidden="true"></span>
                    </a>
                </div>

                <div id="navbar-contents" class=format!("navbar-menu {}", self.navbar_menu_expanded_class())>
                    <div class="navbar-start">
                        <RouterAnchor<AppRoute> classes=self.navbar_item_class(AppRoute::Index) route=AppRoute::Index>
                            { localize!("home") }
                        </RouterAnchor<AppRoute>>
                        <RouterAnchor<AppRoute> classes=self.navbar_item_class(AppRoute::Game) route=AppRoute::Game>
                            { localize!("space") }
                        </RouterAnchor<AppRoute>>
                    </div>
                    <div class="navbar-end">
                        <div class="navbar-item">
                            <button class="button" onclick=self.link.callback(|_| Message::ToggleRendering)>{ self.rendering_icon() }</button>
                        </div>
                    </div>
                </div>
            </nav>
        }
    }

    fn navbar_menu_expanded_class(&self) -> &'static str {
        if self.navbar_expanded {
            "is-active"
        } else {
            ""
        }
    }

    fn rendering_icon(&self) -> Html {
        if self.rendering {
            html! { <i class="icofont-ui-pause"></i> }
        } else {
            html! { <i class="icofont-ui-play"></i> }
        }
    }

    fn navbar_item_class(&self, check_route: AppRoute) -> &'static str {
        if self.route == check_route {
            "navbar-item is-active"
        } else {
            "navbar-item"
        }
    }
}