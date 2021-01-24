use crate::routes::AppRoute;
use yew::prelude::*;
use yew_router::prelude::*;

pub struct App {
    link: ComponentLink<Self>,
    title: String,
}

pub enum Message {
    SetTitle(String),
}

impl Component for App {
    type Message = Message;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            title: Default::default(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        let Message::SetTitle(title) = msg;
        self.title = title;
        false
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let set_title = self.link.callback(Message::SetTitle);
        // let user = self.user.clone();
        html! {
            <div>
                //{ self.nav_bar() }

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
