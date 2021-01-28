use crossbeam::channel::Sender;
use yew::prelude::*;

use crate::{localize, redraw_loop};
use web_sys::{HtmlCanvasElement, WheelEvent};

mod glspace;
mod space2d;

pub struct Game {
    link: ComponentLink<Self>,
    props: Props,
    loop_sender: Sender<redraw_loop::Command>,
    space_sender: Sender<space2d::Command>,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub set_title: Callback<String>,
    pub foregrounded: bool,
    pub rendering: bool,
}

#[derive(Debug)]
pub enum Message {
    WheelEvent(WheelEvent)
}

impl Component for Game {
    type Properties = Props;
    type Message = Message;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let (view, space_sender) = space2d::SpaceView::new();
        let loop_sender = redraw_loop::RedrawLoop::launch(view, redraw_loop::Configuration::default());
        let component = Self { link, props, loop_sender, space_sender };
        component.update_title();
        component
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        let Message::WheelEvent(event) = msg;
        let delta = event.delta_y() / 100.;
        // let amount = match event.delta_mode() {
        //     WheelEvent::DOM_DELTA_PIXEL => delta,
        //     WheelEvent::
        // };
        let _ = self.space_sender.send(space2d::Command::Zoom(delta));
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
                let _ = self.loop_sender.send(redraw_loop::Command::SetFramerateTarget(60.));
            } else {
                let _ = self.loop_sender.send(redraw_loop::Command::SetFramerateTarget(10.));
            }
        }

        self.update_title();
        redraw
    }

    fn view(&self) -> Html {
        html! {
            <canvas id="glcanvas" onwheel=self.link.callback(Message::WheelEvent)></canvas>
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