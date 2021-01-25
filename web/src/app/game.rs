use crate::{localize, space_bridge};
use yew::prelude::*;

pub struct Game {
    props: Props,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub set_title: Callback<String>,
}

impl Component for Game {
    type Properties = Props;
    type Message = ();

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        let component = Self { props };
        component.update_title();
        let _ = space_bridge::emit_command(space_bridge::BridgeCommand::IncreaseFramerate);
        component
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        false
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props = props;
        self.update_title();
        false
    }

    fn view(&self) -> Html {
        html! {
            <div class="intentionally-blank" />
        }
    }

    fn destroy(&mut self) {
        let _ = space_bridge::emit_command(space_bridge::BridgeCommand::ReduceFramerate);
    }
}

impl Game {
    fn update_title(&self) {
        self.props.set_title.emit(localize!("cosmic-verge"));
    }
}
