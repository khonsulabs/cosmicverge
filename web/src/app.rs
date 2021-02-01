use cosmicverge_shared::{CosmicVergeResponse, Pilot};
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
    connected_pilots: Option<usize>,
}

#[derive(PartialEq, Debug)]
pub struct LoggedInUser {
    pub user_id: i64,
    pub pilot: PilotingState,
}

impl LoggedInUser {
    fn with_pilot(&self, pilot: Pilot) -> Option<Arc<Self>> {
        Some(Arc::new(Self {
            user_id: self.user_id,
            pilot: PilotingState::Selected(pilot),
        }))
    }
}

#[derive(PartialEq, Debug)]
pub enum PilotingState {
    Unselected { available: Vec<Pilot> },
    Selected(Pilot),
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
            connected_pilots: None,
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
                    CosmicVergeResponse::ServerStatus { connected_pilots } => {
                        self.connected_pilots = Some(connected_pilots);
                        true
                    }
                    CosmicVergeResponse::Authenticated { user_id, pilots } => {
                        self.user = Some(Arc::new(LoggedInUser {
                            user_id,
                            pilot: PilotingState::Unselected { available: pilots },
                        }));
                        true
                    }
                    CosmicVergeResponse::PilotChanged(pilot) => {
                        let user = self.user.as_ref().expect("The server should never send this without us being Authenticated first");

                        self.user = user.with_pilot(pilot);
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
        let connected_pilots = self.connected_pilots;
        let redirect = Router::redirect(|_| AppRoute::Index);
        html! {
            <Router<AppRoute>
                render = Router::render(move |route: AppRoute| {
                    let app = AppRouteRenderer {
                        link: link.clone(),
                        route,
                        rendering,
                        navbar_expanded,
                        connected,
                        connected_pilots,
                        user: user.clone(),
                    };
                    app.render()
                })
                redirect = redirect
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
    connected_pilots: Option<usize>,
}

impl AppRouteRenderer {
    fn render(&self) -> Html {
        let set_title = self.link.callback(Message::SetTitle);

        if self.connected.unwrap_or_default() {
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
            html! {
                <body>
                    <div class="container">
                        <h1 class="is-size-1 has-text-centered">{ localize!("connecting") }</h1>
                        <progress class="progress is-large is-info" max="100" aria-hidden="true"></progress>
                    </div>
                </body>
            }
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
        let connected_pilots = if let Some(connected_pilots) = self.connected_pilots {
            html! {
                <div class="navbar-item">
                    { localize!("connected-pilots", "count" => connected_pilots) }
                </div>
            }
        } else {
            Default::default()
        };
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
                        { connected_pilots }
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
