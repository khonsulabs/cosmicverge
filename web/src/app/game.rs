use std::{collections::HashMap, time::Duration};

use cosmicverge_shared::{
    euclid::Point2D,
    protocol::CosmicVergeResponse,
    solar_systems::{universe, Named, Pixels, SolarSystem, SolarSystemId},
};
use crossbeam::channel::Sender;
use web_sys::{HtmlCanvasElement, MouseEvent, WheelEvent};
use yew::{
    prelude::*,
    services::{timeout::TimeoutTask, TimeoutService},
};

use crate::{
    client_api::{AgentMessage, AgentResponse, ApiAgent, ApiBridge},
    localize, redraw_loop,
};

const DOUBLE_CLICK_MS: i64 = 400;
const DOUBLE_CLICK_MAX_PIXEL_DISTANCE: f64 = 5.;

#[cfg(name = "opengl")]
mod glspace;
mod space2d;

#[derive(Default, Debug)]
struct MouseButtons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub mouse_down_start: Option<Point2D<i32, Pixels>>,
    pub last_mouse_location: Option<Point2D<i32, Pixels>>,

    pub sequential_click_state: Option<SequentialClickState>,
}

impl MouseButtons {
    fn is_down(&self, button: Button) -> bool {
        match button {
            Button::Left => self.left,
            Button::Right => self.right,
            Button::Middle => self.middle,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Button {
    Left,
    Right,
    Middle,
}

#[derive(Debug)]
struct SequentialClickState {
    button: Button,
    location: Point2D<i32, Pixels>,
    // from performance::now()
    first_click: f64,
    click_count: i64,
}

pub struct Game {
    _api: ApiBridge,
    link: ComponentLink<Self>,
    props: Props,
    solar_system: &'static SolarSystem,
    loop_sender: Sender<redraw_loop::Command>,
    space_sender: Sender<space2d::Command>,
    mouse_buttons: MouseButtons,
    mouse_location: Option<Point2D<i32, Pixels>>,
    touches: HashMap<i32, TouchState>,
    performance: web_sys::Performance,
    click_handler_timer: Option<TimeoutTask>,
}

struct TouchState {
    start: Point2D<i32, Pixels>,
    last_location: Option<Point2D<i32, Pixels>>,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub set_title: Callback<String>,
    pub foregrounded: bool,
    pub rendering: bool,
}

#[derive(Debug)]
pub enum Message {
    WheelEvent(WheelEvent),
    TouchStart(TouchEvent),
    TouchEnd(TouchEvent),
    TouchMove(TouchEvent),
    TouchCancel(TouchEvent),
    MouseDown(MouseEvent),
    MouseUp(MouseEvent),
    MouseMove(MouseEvent),
    MouseEnter(MouseEvent),
    MouseLeave(MouseEvent),
    ApiMessage(AgentResponse),
    CheckHandleClick,
}

impl Game {
    fn update_mouse_buttons(&mut self, button: i16, state: bool, location: Point2D<i32, Pixels>) {
        let button = match button {
            0 => {
                self.mouse_buttons.left = state;
                Button::Left
            }
            1 => {
                self.mouse_buttons.middle = state;
                Button::Middle
            }
            2 => {
                self.mouse_buttons.right = state;
                Button::Right
            }
            other => {
                error!("Unexpected mouse button: {}", other);
                return;
            }
        };

        // For a new mouse button reset the click handler timeout
        if state {
            self.click_handler_timer = Some(TimeoutService::spawn(
                Duration::from_millis(DOUBLE_CLICK_MS as u64),
                self.link.callback(|_| Message::CheckHandleClick),
            ));

            let now = self.performance.now();
            if let Some(state) = &mut self.mouse_buttons.sequential_click_state {
                if state.button == button {
                    let distance = self
                        .mouse_buttons
                        .last_mouse_location
                        .map(|l| l.to_f64().distance_to(location.to_f64()));
                    let elapsed = (now - state.first_click) as i64;
                    if (distance.is_none() || distance.unwrap() < DOUBLE_CLICK_MAX_PIXEL_DISTANCE)
                        && elapsed < state.click_count * DOUBLE_CLICK_MS
                    {
                        state.click_count += 1;
                        return;
                    }
                }
            }

            self.mouse_buttons.sequential_click_state = Some(SequentialClickState {
                button,
                location,
                first_click: now,
                click_count: 1,
            });
        }
    }
}

impl Component for Game {
    type Properties = Props;
    type Message = Message;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut api = ApiAgent::bridge(link.callback(Message::ApiMessage));
        api.send(AgentMessage::RegisterBroadcastHandler);
        let (view, space_sender) = space2d::SpaceView::new();
        let loop_sender =
            redraw_loop::RedrawLoop::launch(view, redraw_loop::Configuration::default());

        let component = Self {
            _api: api,
            link,
            props,
            loop_sender,
            space_sender,
            performance: web_sys::window().unwrap().performance().unwrap(),
            mouse_buttons: Default::default(),
            touches: Default::default(),
            solar_system: universe().get(&SolarSystemId::SM0A9F4),
            click_handler_timer: None,
            mouse_location: None,
        };
        component
            .space_sender
            .send(space2d::Command::SetSolarSystem(Some(
                component.solar_system,
            )))
            .unwrap();
        component.update_title();
        component
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Message::CheckHandleClick => {
                if let Some(state) = &self.mouse_buttons.sequential_click_state {
                    if !self.mouse_buttons.is_down(state.button) {
                        let _ = self.space_sender.send(space2d::Command::HandleClick {
                            button: state.button,
                            count: state.click_count,
                            location: state.location,
                        });
                    }
                }

                self.mouse_buttons.sequential_click_state = None;
            }
            Message::WheelEvent(event) => {
                event.prevent_default();
                let delta = event.delta_y();
                let amount = match event.delta_mode() {
                    WheelEvent::DOM_DELTA_PIXEL => delta,
                    WheelEvent::DOM_DELTA_LINE => delta * 20.,
                    WheelEvent::DOM_DELTA_PAGE => delta * 50.,
                    other => {
                        error!("Unexpected mouse wheel event mode: {}", other);
                        return false;
                    }
                };
                let amount = (amount / 1000.).min(1.).max(-1.);
                let focus = Point2D::new(event.client_x(), event.client_y());
                let _ = self
                    .space_sender
                    .send(space2d::Command::Zoom(amount, focus.to_f64()));
            }
            Message::MouseDown(event) => {
                event.prevent_default();
                let location = Point2D::new(event.client_x(), event.client_y());
                self.update_mouse_buttons(event.button(), true, location);

                self.mouse_buttons.mouse_down_start = Some(location);
                self.mouse_buttons.last_mouse_location = None;
            }
            Message::MouseUp(event) => {
                event.prevent_default();
                let location = Point2D::new(event.client_x(), event.client_y());
                self.update_mouse_buttons(event.button(), false, location);
            }
            Message::MouseMove(event) => {
                let location = Point2D::<i32, Pixels>::new(event.client_x(), event.client_y());
                self.mouse_location = Some(location);
                if let Some(start) = self.mouse_buttons.mouse_down_start {
                    event.prevent_default();
                    let delta = match self.mouse_buttons.last_mouse_location {
                        Some(last_mouse_location) => location - last_mouse_location,
                        None => location - start,
                    };
                    self.mouse_buttons.last_mouse_location = Some(location);

                    if self.mouse_buttons.left {
                        let _ = self
                            .space_sender
                            .send(space2d::Command::Pan(delta.to_f64()));
                    }
                }
            }
            Message::MouseEnter(event) => {
                let location = Point2D::<i32, Pixels>::new(event.client_x(), event.client_y());
                self.mouse_location = Some(location);
            }
            Message::MouseLeave(_) => {
                self.mouse_location = None;
            }
            Message::TouchStart(event) => {
                event.prevent_default();
                let touches = event.changed_touches();
                for i in 0..touches.length() {
                    let touch = touches.get(i).unwrap();
                    let start = Point2D::new(touch.client_x(), touch.client_y());
                    self.touches.insert(
                        touch.identifier(),
                        TouchState {
                            start,
                            last_location: None,
                        },
                    );
                }
            }
            Message::TouchCancel(event) | Message::TouchEnd(event) => {
                event.prevent_default();
                let touches = event.changed_touches();
                for i in 0..touches.length() {
                    let touch = touches.get(i).unwrap();
                    self.touches.remove(&touch.identifier());
                }
            }
            Message::TouchMove(event) => {
                event.prevent_default();
                let touches = event.touches();
                if touches.length() == 1 {
                    // Pan
                    let touch = touches.get(0).unwrap();
                    if let Some(touch_state) = self.touches.get_mut(&touch.identifier()) {
                        let location =
                            Point2D::<i32, Pixels>::new(touch.client_x(), touch.client_y());
                        let delta = match touch_state.last_location {
                            Some(last_mouse_location) => location - last_mouse_location,
                            None => location - touch_state.start,
                        };
                        touch_state.last_location = Some(location);

                        let _ = self
                            .space_sender
                            .send(space2d::Command::Pan(delta.to_f64()));
                    }
                } else if touches.length() == 2 {
                    // Zoom
                    let touch1 = touches.get(0).unwrap();
                    let touch1_location =
                        Point2D::<i32, Pixels>::new(touch1.client_x(), touch1.client_y());
                    if let Some(old_touch1) = self.touches.get(&touch1.identifier()) {
                        let touch2 = touches.get(1).unwrap();
                        let touch2_location =
                            Point2D::<i32, Pixels>::new(touch2.client_x(), touch2.client_y());
                        if let Some(old_touch2) = self.touches.get(&touch2.identifier()) {
                            let touch1_last_location =
                                old_touch1.last_location.unwrap_or(old_touch1.start);
                            let touch2_last_location =
                                old_touch2.last_location.unwrap_or(old_touch2.start);
                            let current_midpoint = (touch1_location.to_vector()
                                + touch2_location.to_vector())
                            .to_f64()
                                / 2.;
                            let old_midpoint = (touch1_last_location.to_vector()
                                + touch2_last_location.to_vector())
                            .to_f64()
                                / 2.;

                            let _ = self
                                .space_sender
                                .send(space2d::Command::Pan(current_midpoint - old_midpoint));

                            let current_distance = touch1_location
                                .to_f64()
                                .distance_to(touch2_location.to_f64());
                            let old_distance = touch1_last_location
                                .to_f64()
                                .distance_to(touch2_last_location.to_f64());
                            let ratio = current_distance / old_distance - 1.;

                            let _ = self
                                .space_sender
                                .send(space2d::Command::Zoom(ratio, current_midpoint.to_point()));
                        }
                    }
                } else {
                    error!("Only one or two fingers handled. Touch this less.")
                }

                let touches = event.changed_touches();
                for i in 0..touches.length() {
                    let touch = touches.get(i).unwrap();
                    let location = Point2D::<i32, Pixels>::new(touch.client_x(), touch.client_y());
                    if let Some(state) = self.touches.get_mut(&touch.identifier()) {
                        state.last_location = Some(location);
                    }
                }
            }
            Message::ApiMessage(message) => match message {
                AgentResponse::Response(response) => match response {
                    CosmicVergeResponse::PilotChanged(active_pilot) => {
                        let _ = self
                            .space_sender
                            .send(space2d::Command::SetPilot(active_pilot));
                    }
                    CosmicVergeResponse::SpaceUpdate {
                        ships, location, ..
                    } => {
                        let _ = self.space_sender.send(space2d::Command::UpdateSolarSystem {
                            ships,
                            solar_system: location.system,
                        });
                    }
                    _ => {}
                },
                _ => {}
            },
        }

        false
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        let mut redraw = false;
        if props.rendering != self.props.rendering {
            redraw = true;
            self.props.rendering = props.rendering;
            if props.rendering {
                let _ = self.loop_sender.send(redraw_loop::Command::Resume);
            } else {
                let _ = self.loop_sender.send(redraw_loop::Command::Pause);
            }
        }

        if props.foregrounded != self.props.foregrounded {
            redraw = true;
            self.props.foregrounded = props.foregrounded;
            if props.foregrounded {
                let _ = self
                    .loop_sender
                    .send(redraw_loop::Command::SetFramerateTarget(None));
            } else {
                let _ = self
                    .loop_sender
                    .send(redraw_loop::Command::SetFramerateTarget(Some(10.)));
            }
        }

        self.update_title();
        redraw
    }

    fn view(&self) -> Html {
        html! {
            <div id="game"
                    onwheel=self.link.callback(Message::WheelEvent)
                    onmousedown=self.link.callback(Message::MouseDown)
                    onmouseup=self.link.callback(Message::MouseUp)
                    onmousemove=self.link.callback(Message::MouseMove)
                    onmouseenter=self.link.callback(Message::MouseEnter)
                    onmouseleave=self.link.callback(Message::MouseLeave)
                    ontouchstart=self.link.callback(Message::TouchStart)
                    ontouchmove=self.link.callback(Message::TouchMove)
                    ontouchend=self.link.callback(Message::TouchEnd)
                    ontouchcancel=self.link.callback(Message::TouchCancel)>
                <div id="hud">
                    <div id="solar-system">
                        <label>{ localize!("current-system") }</label>
                        <div id="solar-system-name">{ &self.solar_system.id.name() }</div>
                    </div>
                </div>
                <canvas id="glcanvas" />
            </div>
        }
    }

    fn destroy(&mut self) {
        let _ = self.loop_sender.send(redraw_loop::Command::Stop);
    }
}

impl Game {
    fn update_title(&self) {
        self.props.set_title.emit(localize!("cosmic-verge"));
    }
}

fn check_canvas_size(canvas: &HtmlCanvasElement) -> bool {
    let width_attr = canvas.attributes().get_with_name("width");
    let height_attr = canvas.attributes().get_with_name("height");
    let actual_width: Option<i32> = width_attr
        .as_ref()
        .map(|w| w.value().parse().ok())
        .flatten();
    let actual_height: Option<i32> = height_attr
        .as_ref()
        .map(|h| h.value().parse().ok())
        .flatten();
    let mut changed = false;
    if actual_width.is_none() || actual_width.unwrap() != canvas.client_width() {
        changed = true;
        if let Some(attr) = width_attr {
            attr.set_value(&canvas.client_width().to_string());
        } else {
            let _ = canvas.set_attribute("width", &canvas.client_width().to_string());
        }
    }

    if actual_height.is_none() || actual_height.unwrap() != canvas.client_height() {
        changed = true;
        if let Some(attr) = height_attr {
            attr.set_value(&canvas.client_height().to_string());
        } else {
            let _ = canvas.set_attribute("height", &canvas.client_height().to_string());
        }
    }

    changed
}
