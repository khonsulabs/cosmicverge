use crossbeam::channel::Sender;
use yew::prelude::*;

use crate::{localize, redraw_loop};

mod space;

pub struct Game {
    props: Props,
    loop_sender: Sender<redraw_loop::Command>,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub set_title: Callback<String>,
    pub foregrounded: bool,
    pub rendering: bool,
}

impl Component for Game {
    type Properties = Props;
    type Message = ();

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        let loop_sender = redraw_loop::RedrawLoop::launch(space::SpaceView::new(), redraw_loop::Configuration::default());
        let component = Self { props, loop_sender };
        component.update_title();
        component
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
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
            <canvas id="glcanvas"></canvas>
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