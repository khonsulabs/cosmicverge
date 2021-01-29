use crossbeam::channel::Sender;
use glam::IVec2;
use web_sys::{HtmlCanvasElement, MouseEvent, WheelEvent};
use yew::prelude::*;

use crate::{localize, redraw_loop};

#[cfg(name = "opengl")]
mod glspace;
mod space2d;

#[derive(Default)]
struct MouseButtons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub mouse_down_start: Option<IVec2>,
    pub last_mouse_location: Option<IVec2>,
}

pub struct Game {
    link: ComponentLink<Self>,
    props: Props,
    loop_sender: Sender<redraw_loop::Command>,
    space_sender: Sender<space2d::Command>,
    mouse_buttons: MouseButtons,
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
        };
        component.update_title();
        component
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Message::WheelEvent(event) => {
                let delta = event.delta_y() / 100.;
                let amount = match event.delta_mode() {
                    WheelEvent::DOM_DELTA_PIXEL => delta,
                    WheelEvent::DOM_DELTA_LINE => delta * 20.,
                    WheelEvent::DOM_DELTA_PAGE => delta * 50.,
                    other => {
                        error!("Unexpected mouse wheel event mode: {}", other);
                        return false;
                    }
                };
                let _ = self.space_sender.send(space2d::Command::Zoom(amount));
            }
            Message::MouseDown(event) => {
                self.update_mouse_buttons(event.button(), true);

                self.mouse_buttons.mouse_down_start =
                    Some(IVec2::new(event.client_x(), event.client_y()));
                self.mouse_buttons.last_mouse_location = None;
            }
            Message::MouseUp(event) => {
                self.update_mouse_buttons(event.button(), false);
            }
            Message::MouseMove(event) => {
                if let Some(start) = self.mouse_buttons.mouse_down_start {
                    let location = IVec2::new(event.client_x(), event.client_y());
                    let delta = match self.mouse_buttons.last_mouse_location {
                        Some(last_mouse_location) => location - last_mouse_location,
                        None => location - start,
                    };
                    self.mouse_buttons.last_mouse_location = Some(location);

                    if self.mouse_buttons.left {
                        let _ = self
                            .space_sender
                            .send(space2d::Command::Pan(delta.as_f64()));
                    }
                }
            }
            Message::MouseEnter(_) => {}
            Message::MouseLeave(_) => {}
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
            <canvas
                id="glcanvas"
                onwheel=self.link.callback(Message::WheelEvent)
                onmousedown=self.link.callback(Message::MouseDown)
                onmouseup=self.link.callback(Message::MouseUp)
                onmousemove=self.link.callback(Message::MouseMove)
                onmouseenter=self.link.callback(Message::MouseEnter)
                onmouseleave=self.link.callback(Message::MouseLeave)
            />
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
