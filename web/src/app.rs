use crate::routes::AppRoute;
use yew::prelude::*;
use yew_router::prelude::*;
use crate::{space_bridge, localize};

pub struct App {
    link: ComponentLink<Self>,
    rendering: bool,
    navbar_expanded: bool,
}

pub enum Message {
    SetTitle(String),
    ToggleRendering,
    ToggleNavbar,
}

fn set_document_title(title: &str) {
    web_sys::window().unwrap().document().unwrap().set_title(title);
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
            Message::ToggleRendering => {
                self.rendering = !self.rendering;

                space_bridge::emit_command( if self.rendering {
                    space_bridge::BridgeCommand::ResumeRendering
                } else {
                    space_bridge::BridgeCommand::PauseRendering
                }).unwrap();

                true
            }
            Message::ToggleNavbar => {
                self.navbar_expanded = !self.navbar_expanded;
                true
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let set_title = self.link.callback(Message::SetTitle);
        // let user = self.user.clone();
        html! {
            <div>
                { self.navbar() }
                <p>{ self.rendering }</p>
                <button onclick=self.link.callback(|_| Message::ToggleRendering)>{ "Toggle" }</button>

                <section class="section content">
                    <div class="columns is-centered">
                        <div class="column is-half">
                            <p class="notification is-danger is-light">
                  //              { localize("early-warning") }
                            </p>
                        </div>
                    </div>
                    <Router<AppRoute>
                        render = Router::render(move |switch: AppRoute| {
                            switch.render(set_title.clone())
                        })
                    />
                </section>

                //{ self.footer() }
            </div>
        }
    }
}

impl App {
    fn navbar(&self) -> Html {
        html! {
            <nav class=format!("navbar {}", self.navbar_menu_expanded_class()) role="navigation" aria-label=localize!("navbar-label")>
                <div class="navbar-brand">
                    <a class="navbar-item" href="/">
                        { localize!("cosmic-verge") }
                    </a>

                    <a role="button" class="navbar-burger" aria-label=localize!("navbar-menu-label") aria-expanded=self.navbar_expanded data-target="navbar-contents" onclick=self.link.callback(|_| Message::ToggleNavbar)>
                        <span aria-hidden="true"></span>
                        <span aria-hidden="true"></span>
                        <span aria-hidden="true"></span>
                    </a>
                </div>

                <div id="navbar-contents" class=format!("navbar-menu {}", self.navbar_menu_expanded_class())>
                    <div class="navbar-start">
                        <a class="navbar-item">
                            { "Home" }
                        </a>
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
}