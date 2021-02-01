use crossbeam::channel::Sender;
use euclid::Point2D;
use web_sys::{HtmlCanvasElement, MouseEvent, WheelEvent};
use yew::prelude::*;

pub struct Pixels;
pub struct Solar;

use std::collections::HashMap;

use crate::{localize, redraw_loop};

#[derive(Debug, Clone)]
pub struct SolarSystem {
    pub name: String,
    pub background: String,
    pub locations: Vec<SolarSystemLocation>,
}

#[derive(Debug, Clone)]
pub struct SolarSystemLocation {
    pub id: i64,
    pub name: String,
    pub image: String,
    pub size: f64,
    pub location: Point2D<f64, Solar>,
    pub owned_by: Option<i64>,
}

pub fn fake_solar_system() -> SolarSystem {
    SolarSystem {
        name: String::from("SM-0-A9F4"),
        background: String::from("/helianthusgames/Backgrounds/BlueStars.png"),
        locations: vec![
            SolarSystemLocation {
                id: 1,
                name: String::from("Sun"),
                image: String::from("/helianthusgames/Suns/2.png"),
                size: 128.,
                location: Point2D::zero(),
                owned_by: None,
            },
            SolarSystemLocation {
                id: 2,
                name: String::from("Earth"),
                image: String::from("/helianthusgames/Terran_or_Earth-like/1.png"),
                size: 32.,
                location: Point2D::new(600., 0.),
                owned_by: Some(1),
            },
            SolarSystemLocation {
                id: 3,
                name: String::from("Earth"),
                image: String::from("/helianthusgames/Rocky/1.png"),
                size: 24.,
                location: Point2D::new(200., 200.),
                owned_by: Some(1),
            },
        ],
    }
}

#[cfg(name = "opengl")]
mod glspace;
mod space2d;

#[derive(Default)]
struct MouseButtons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub mouse_down_start: Option<Point2D<i32, Pixels>>,
    pub last_mouse_location: Option<Point2D<i32, Pixels>>,
}

pub struct Game {
    link: ComponentLink<Self>,
    props: Props,
    solar_system: SolarSystem,
    loop_sender: Sender<redraw_loop::Command>,
    space_sender: Sender<space2d::Command>,
    mouse_buttons: MouseButtons,
    touches: HashMap<i32, TouchState>,
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
}

impl Game {
    fn update_mouse_buttons(&mut self, button: i16, state: bool) {
        match button {
            0 => {
                self.mouse_buttons.left = state;
            }
            1 => {
                self.mouse_buttons.middle = state;
            }
            2 => {
                self.mouse_buttons.right = state;
            }
            other => error!("Unexpected mouse button: {}", other),
        }
    }
}

impl Component for Game {
    type Properties = Props;
    type Message = Message;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let (view, space_sender) = space2d::SpaceView::new();
        let loop_sender =
            redraw_loop::RedrawLoop::launch(view, redraw_loop::Configuration::default());
        let component = Self {
            link,
            props,
            loop_sender,
            space_sender,
            mouse_buttons: Default::default(),
            touches: Default::default(),
            solar_system: fake_solar_system(),
        };
        component
            .space_sender
            .send(space2d::Command::SetSolarSystem(Some(
                component.solar_system.clone(),
            )))
            .unwrap();
        component.update_title();
        component
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
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
                self.update_mouse_buttons(event.button(), true);

                self.mouse_buttons.mouse_down_start =
                    Some(Point2D::new(event.client_x(), event.client_y()));
                self.mouse_buttons.last_mouse_location = None;
            }
            Message::MouseUp(event) => {
                event.prevent_default();
                self.update_mouse_buttons(event.button(), false);
            }
            Message::MouseMove(event) => {
                if let Some(start) = self.mouse_buttons.mouse_down_start {
                    event.prevent_default();
                    let location = Point2D::<i32, Pixels>::new(event.client_x(), event.client_y());
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
            Message::MouseEnter(_) => {}
            Message::MouseLeave(_) => {}
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
                        <div id="solar-system-name">{ &self.solar_system.name }</div>
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
