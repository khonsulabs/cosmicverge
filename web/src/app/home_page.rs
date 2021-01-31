use std::sync::Arc;

use cosmicverge_shared::{CosmicVergeRequest, CosmicVergeResponse, OAuthProvider, Pilot};
use wasm_bindgen::__rt::std::borrow::Cow;
use yew::prelude::*;
use yew_bulma::prelude::*;

use crate::app::PilotingState;
use crate::strings::translate_error;
use crate::{
    app::LoggedInUser,
    client_api::{AgentMessage, AgentResponse, ApiAgent, ApiBridge},
    localize, localize_html,
};

pub struct HomePage {
    link: ComponentLink<Self>,
    api: ApiBridge,
    props: Props,
    current_storage_status: bool,
    error_message: Option<String>,

    pilot_state: PilotLoginState,
}

enum PilotLoginState {
    Selecting,
    Creating {
        sent_request: bool,
        name: FormStorage<Option<String>>,
    },
    WaitingForServer,
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
    SelectPilot(i64),
    NewPilot,
    CreatePilot,
    ListPilots,
    FormChanged,
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
            pilot_state: PilotLoginState::Selecting,
            error_message: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> bool {
        match msg {
            Message::LogInWith(provider) => {
                self.api.send(AgentMessage::Request(
                    CosmicVergeRequest::AuthenticationUrl(provider),
                ));
                false
            }
            Message::ApiMessage(msg) => match msg {
                AgentResponse::StorageStatus(status) => {
                    self.current_storage_status = status;
                    true
                }
                AgentResponse::Response(response) => match response {
                    CosmicVergeResponse::Error { message } => {
                        self.error_message = message;
                        match &self.pilot_state {
                            PilotLoginState::WaitingForServer => {
                                self.pilot_state = PilotLoginState::Selecting;
                            }
                            PilotLoginState::Creating { name, .. } => {
                                self.pilot_state = PilotLoginState::Creating {
                                    name: name.clone(),
                                    sent_request: false,
                                };
                            }
                            PilotLoginState::Selecting => {}
                        }

                        true
                    }
                    _ => false,
                },
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
            Message::SelectPilot(pilot_id) => {
                self.api
                    .send(AgentMessage::Request(CosmicVergeRequest::SelectPilot(
                        pilot_id,
                    )));
                self.pilot_state = PilotLoginState::WaitingForServer;
                true
            }
            Message::NewPilot => {
                self.pilot_state = PilotLoginState::Creating {
                    sent_request: false,
                    name: FormStorage::default(),
                };
                true
            }
            Message::CreatePilot => {
                let name = {
                    if let PilotLoginState::Creating { name, .. } = &self.pilot_state {
                        name.clone()
                    } else {
                        unreachable!()
                    }
                };

                self.pilot_state = PilotLoginState::Creating {
                    sent_request: true,
                    name: name.clone(),
                };
                let name = name.unchecked_value().unwrap();
                self.api
                    .send(AgentMessage::Request(CosmicVergeRequest::CreatePilot {
                        name,
                    }));

                true
            }
            Message::ListPilots => {
                self.pilot_state = PilotLoginState::Selecting;
                true
            }
            Message::FormChanged => true,
        }
    }

    fn change(&mut self, props: Self::Properties) -> bool {
        self.props = props;
        true
    }

    fn view(&self) -> Html {
        if let Some(user) = &self.props.user {
            match &user.pilot {
                PilotingState::Unselected { available } => match &self.pilot_state {
                    PilotLoginState::Selecting => self.select_pilot(available),
                    PilotLoginState::Creating { sent_request, name } => {
                        self.create_pilot(*sent_request, name)
                    }
                    PilotLoginState::WaitingForServer => self.waiting_for_server(),
                },
                PilotingState::Selected(pilot) => {
                    // TODO player dashboard? Not sure.
                    html! {
                        <p>{"Welcome Back "}{ &pilot.name }</p>
                    }
                }
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

impl HomePage {
    fn select_pilot(&self, available_pilots: &Vec<Pilot>) -> Html {
        let pilots = if available_pilots.is_empty() {
            html! {
                <div class="notification is-info">
                    { localize!("no-pilots") }
                </div>
            }
        } else {
            available_pilots
                .iter()
                .map(|p| self.pilot_button(p))
                .collect::<Html>()
        };

        html! {
            <div class="container content">
                <Title>{ localize!("select-pilot") }</Title>
                { self.error_message() }
                { pilots }
                <div class="columns is-centered">
                    <button class="button is-primary" onclick=self.link.callback(|_| Message::NewPilot)>
                        { localize!("create-new-pilot") }
                    </button>
                </div>
            </div>
        }
    }

    fn error_message(&self) -> Html {
        if let Some(message) = self.error_message.as_deref() {
            html! {
                <div class="notification is-danger">
                    { localize_html!(message) }
                </div>
            }
        } else {
            Default::default()
        }
    }

    fn create_pilot(&self, sent_request: bool, name: &FormStorage<Option<String>>) -> Html {
        let errors = if let Some(errors) = self.validate_pilot_name(name) {
            Some(errors.translate(translate_error))
        } else {
            None
        };

        html! {
            <div class="container content">
                <Title>{ localize!("create-new-pilot") }</Title>
                <form>
                    { self.error_message() }
                    <Field<PilotFields> field=PilotFields::Name errors=errors.clone()>
                        <Label<PilotFields> text=localize!("pilot-name") field=PilotFields::Name />
                        <TextInput<PilotFields,String>
                            field=PilotFields::Name
                            errors=errors.clone()
                            storage=name.clone()
                            readonly=sent_request
                            on_value_changed=self.link.callback(|_| Message::FormChanged)
                            />
                    </Field<PilotFields>>
                    <div class="field is-grouped is-grouped-right">
                        <Button
                            label=localize!("cancel")
                            css_class="is-light"
                            action=self.link.callback(|e: web_sys::MouseEvent| {e.prevent_default(); Message::ListPilots})
                        />
                        <Button
                            label=localize!("create-new-pilot")
                            css_class="is-primary"
                            action=self.link.callback(|e: web_sys::MouseEvent| {e.prevent_default(); Message::CreatePilot})
                            processing=sent_request
                        />
                    </div>
                </form>
            </div>
        }
    }

    fn waiting_for_server(&self) -> Html {
        html! {
            <div class="container">
                <h1 class="is-size-1 has-text-centered">{ localize!("connecting") }</h1>
                <progress class="progress is-large is-info" max="100" aria-hidden="true"></progress>
            </div>
        }
    }

    fn pilot_button(&self, pilot: &Pilot) -> Html {
        let pilot_id = pilot.id;
        html! {
            <div class="columns is-centered">
                <button class="button" onclick=self.link.callback(move |_| Message::SelectPilot(pilot_id))>
                    { &pilot.name }
                </button>
            </div>
        }
    }

    fn validate_pilot_name(
        &self,
        name: &FormStorage<Option<String>>,
    ) -> Option<Rc<ErrorSet<PilotFields>>> {
        ModelValidator::default()
            .with_custom(PilotFields::Name, PilotNameValidator { name: name.clone() })
            .validate()
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum PilotFields {
    Name,
}

impl FormField for PilotFields {
    fn form_id(&self) -> Cow<'static, str> {
        match self {
            Self::Name => Cow::from("name"),
        }
    }
}

#[derive(Debug)]
struct PilotNameValidator {
    name: FormStorage<Option<String>>,
}

impl Validator for PilotNameValidator {
    fn validate(&self) -> Result<(), ValidationError> {
        match self.name.unchecked_value() {
            Some(name) => match Pilot::cleanup_name(&name) {
                Ok(name) => {
                    if name.is_empty() {
                        Err(ValidationError::NotPresent)
                    } else {
                        Ok(())
                    }
                }
                Err(_) => Err(ValidationError::Custom("pilot-error-invalid-name")),
            },
            None => Err(ValidationError::NotPresent),
        }
    }
}
