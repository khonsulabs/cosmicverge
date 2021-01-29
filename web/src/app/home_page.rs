use yew::prelude::*;
use crate::{app::LoggedInUser, localize, localize_html};
use std::sync::Arc;
use crate::client_api::{AgentMessage, AgentResponse, ApiAgent, ApiBridge};
use cosmicverge_shared::{OAuthProvider, CosmicVergeRequest};

pub struct HomePage {
    link: ComponentLink<Self>,
    api: ApiBridge,
    props: Props,
    current_storage_status: bool,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub set_title: Callback<String>,
    pub user: Option<Arc<LoggedInUser>>,
}

pub enum Message {
    LogInWith(OAuthProvider),
    ApiMessage(AgentResponse),
    ToggleStatus,
}

impl Component for HomePage {
    type Message = Message;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut api = ApiAgent::bridge(link.callback(Message::ApiMessage));
        api.send(AgentMessage::QueryStorageStatus);
        Self {
            link,
            props,
            api,
            current_storage_status: false,
        }
    }

    fn update(&mut self, msg: Self::Message) -> bool {
        match msg {
            Message::LogInWith(provider) => {
                self.api
                    .send(AgentMessage::Request(CosmicVergeRequest::AuthenticationUrl(
                        provider,
                    )));
                false
            }
            Message::ApiMessage(msg) => match msg {
                AgentResponse::StorageStatus(status) => {
                    self.current_storage_status = status;
                    true
                }
                _ => false,
            },
            Message::ToggleStatus => {
                self.api.send(if self.current_storage_status {
                    AgentMessage::DisableStorage
                } else {
                    AgentMessage::EnableStorage
                });
                false
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> bool {
        self.props = props;
        true
    }

    fn view(&self) -> Html {
        if let Some(user) = &self.props.user {
            html! {
                <p>{"You're in"}</p>
            }
        } else {
            html! {
                <div class="login columns is-centered">
                    <div class="column is-half">
                        <h1>{localize!("log-in")}</h1>
                        <p>{localize!("login-intro")}</p>
                        <div class="notification is-info has-text-left">
                            <label class="checkbox">
                                {localize_html!("storage-agreement")}
                                <br />
                                <br />
                                <input type="checkbox" checked=self.current_storage_status onclick=self.link.callback(|_| Message::ToggleStatus) />
                                {localize!("i-agree")}
                            </label>
                        </div>
                        <button class="button twitch-button" disabled=!self.current_storage_status onclick=self.link.callback(|_| Message::LogInWith(OAuthProvider::Twitch))>
                            {localize_html!("log-in-with-twitch")}
                        </button>
                    </div>
                </div>
            }
        }
    }
}