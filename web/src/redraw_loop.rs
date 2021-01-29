use crossbeam::channel::{self, Receiver, Sender};
use wasm_bindgen::{__rt::core::time::Duration, prelude::*, JsCast};
use web_sys::Performance;

pub trait Drawable: 'static {
    fn initialize(&mut self) -> anyhow::Result<()>;
    fn render_frame(&mut self) -> anyhow::Result<()>;
    fn cleanup(&mut self) -> anyhow::Result<()>;
}

pub enum Command {
    SetFramerateTarget(Option<f64>),
    Pause,
    Resume,
    Stop,
}

struct RedrawLoopConfiguration {
    performance: Performance,
    last_frame_time: Option<f64>,
    receiver: Receiver<Command>,
    should_render: bool,
    exit: bool,
    framerate_target: Option<f64>,
    initialized: bool,
}

pub struct RedrawLoop<D> {
    drawable: D,
    config: RedrawLoopConfiguration,
}

pub struct Configuration {
    pub wait_to_render: bool,
    pub framerate_target: Option<f64>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            wait_to_render: false,
            framerate_target: None,
        }
    }
}

impl<D> RedrawLoop<D>
where
    D: Drawable,
{
    pub fn launch(drawable: D, config: Configuration) -> Sender<Command> {
        let (sender, receiver) = channel::unbounded();

        let render_loop = Self {
            drawable,
            config: RedrawLoopConfiguration {
                performance: web_sys::window().unwrap().performance().unwrap(),
                receiver,
                should_render: !config.wait_to_render,
                last_frame_time: None,
                initialized: false,
                exit: false,
                framerate_target: config.framerate_target,
            },
        };
        render_loop.run();
        sender
    }

    fn run(self) {
        if self.config.should_render {
            self.request_animation_frame();
        } else {
            self.wait_for_resume();
        }
    }

    fn receive_commands(&mut self) {
        while let Ok(command) = self.config.receiver.try_recv() {
            self.handle_command(command);
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::SetFramerateTarget(target) => {
                self.config.framerate_target = target;
            }
            Command::Resume => {
                self.config.should_render = true;
            }
            Command::Pause => {
                self.config.should_render = false;
            }
            Command::Stop => {
                self.config.exit = true;
            }
        }
    }

    fn request_animation_frame(self) {
        let closure = Closure::once_into_js(move || self.next_frame());
        web_sys::window()
            .unwrap()
            .request_animation_frame(closure.as_ref().unchecked_ref())
            .unwrap();
    }

    fn sleep_before_frame(self, sleep_duration: Duration) {
        let closure = Closure::once_into_js(move || self.request_animation_frame());
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                sleep_duration.as_millis() as i32,
            )
            .unwrap();
    }

    fn wait_for_resume(mut self) {
        let closure = Closure::once_into_js(move || {
            self.receive_commands();
            if self.config.should_render {
                self.request_animation_frame()
            } else {
                self.wait_for_resume()
            }
        });
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                150,
            )
            .unwrap();
    }

    fn next_frame(mut self) {
        let frame_start = self.config.performance.now();
        self.receive_commands();

        if !self.config.should_render {
            return self.wait_for_resume();
        }

        if !self.config.initialized {
            self.config.initialized = true;
            self.drawable.initialize().unwrap();
        }

        self.drawable.render_frame().unwrap();

        let now = self.config.performance.now();
        let frame_duration = Duration::from_millis(
            (now - self.config.last_frame_time.unwrap_or(frame_start)) as u64,
        );
        self.config.last_frame_time = Some(frame_start);

        if let Some(framerate_target) = self.config.framerate_target {
            let target_duration = Duration::from_secs_f64(1.0 / framerate_target);
            match target_duration.checked_sub(frame_duration) {
                Some(sleep_amount) => {
                    // Only sleep if we know we have more than a few ms to spare
                    if sleep_amount.as_millis() > 3 {
                        return self.sleep_before_frame(sleep_amount);
                    }
                }
                None => {}
            }
        }
        self.request_animation_frame()
    }
}
