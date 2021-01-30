use cosmicverge_shared::{CosmicVergeResponse, UserProfile};
use wasm_bindgen::__rt::std::sync::Arc;
use yew::prelude::*;
use yew_bulma::static_page::StaticPage;
use yew_router::prelude::*;

use crate::{
    app::game::Game,
    client_api::{AgentMessage, AgentResponse, ApiAgent, ApiBridge},
    localize, localize_html,
};

mod game;
mod home_page;

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
    api: ApiBridge,
    rendering: bool,
    user: Option<Arc<LoggedInUser>>,
    navbar_expanded: bool,
    connected: Option<bool>,
}

#[derive(PartialEq, Debug)]
pub struct LoggedInUser {
    pub profile: UserProfile,
}

pub enum Message {
    WsMessage(AgentResponse),
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
        let callback = link.callback(Message::WsMessage);
        let api = ApiAgent::bridge(callback);
        set_document_title(&localize!("cosmic-verge"));

        Self {
            link,
            api,
            rendering: true,
            navbar_expanded: false,
            user: None,
            connected: None,
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
            Message::WsMessage(message) => match message {
                AgentResponse::Disconnected => {
                    self.user = None;
                    self.connected = Some(false);
                    true
                }
                AgentResponse::Connected => {
                    self.user = None;
                    self.connected = Some(true);
                    true
                }
                AgentResponse::Response(response) => match response {
                    CosmicVergeResponse::Authenticated(user) => {
                        self.user = Some(Arc::new(LoggedInUser {
                            profile: user.profile,
                        }));
                        true
                    }
                    _ => false,
                },
                _ => false,
            },
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let link = self.link.clone();
        let user = self.user.clone();
        let rendering = self.rendering;
        let navbar_expanded = self.navbar_expanded;
        let connected = self.connected;

        html! {
            <Router<AppRoute>
                render = Router::render(move |route: AppRoute| {
                    let app = AppRouteRenderer {
                        link: link.clone(),
                        route,
                        rendering,
                        navbar_expanded,
                        connected,
                        user: user.clone(),
                    };
                    app.render()
                })
            />
        }
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            self.api.send(AgentMessage::RegisterBroadcastHandler);
            self.api.send(AgentMessage::Initialize);
        }
    }
}

struct AppRouteRenderer {
    user: Option<Arc<LoggedInUser>>,
    connected: Option<bool>,
    route: AppRoute,
    link: ComponentLink<App>,
    rendering: bool,
    navbar_expanded: bool,
}

impl AppRouteRenderer {
    fn render(&self) -> Html {
        let set_title = self.link.callback(Message::SetTitle);

        if let Some(connected) = self.connected {
            let (game_foregrounded, contents) = match &self.route {
                AppRoute::Game => {
                    // Reveal the canvas
                    (true, Html::default())
                }
                other => (
                    false,
                    html! {
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
                    },
                ),
            };
            let app_class = if game_foregrounded {
                "in-game"
            } else {
                "out-of-game"
            };
            html! {
                <body>
                    { self.navbar() }
                    <div id="app" class=app_class>
                        { contents }
                    </div>
                    <Game set_title=set_title.clone() foregrounded=game_foregrounded rendering=self.rendering />
                </body>
            }
        } else {
            // TODO show a loading image
            html! { <body /> }
        }
    }

    fn render_content(&self, route: &AppRoute) -> Html {
        let set_title = self.link.callback(Message::SetTitle);
        match route {
            AppRoute::Game => unreachable!(),
            AppRoute::Index => {
                html! {<home_page::HomePage set_title=set_title.clone() user=self.user.clone() />}
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
