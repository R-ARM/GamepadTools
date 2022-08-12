use std::thread;
use std::collections::VecDeque;
use derivative::Derivative;

use sdl2::{
    rect::Rect,
    render::{Canvas, Texture},
    pixels::Color,
    event::{Event, EventSender},
    keyboard::Keycode,
};

fn clamp<T: std::cmp::PartialOrd>(x: T, min: T, max: T) -> T {
    if x < min { return min }
    if x > max { return max }
    return x
}

#[derive(Derivative)]
#[derivative(Debug)]
struct Button {
    name: &'static str,
    #[derivative(Debug="ignore")]
    text: Option<Texture>,
    rect: Option<Rect>,
}

impl Button {
    fn draw(&mut self, canvas: &mut Canvas<sdl2::video::Window>, selected: bool) {
        if selected {
            self.text.as_mut().unwrap().set_color_mod(255, 0, 0);
        } else {
            self.text.as_mut().unwrap().set_color_mod(255, 255, 255);
        }
        canvas.copy(self.text.as_ref().unwrap(), None, self.rect).unwrap();
    }
    fn new(name: &'static str, line: usize, text: Texture) -> Button {
        let attr = text.query();
        let rect = Rect::new(0, (line as u32 * attr.height) as i32, attr.width, attr.height);
        Button {
            name,
            text: Some(text),
            rect: Some(rect),
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct Tab {
    name: &'static str,
    buttons: Vec<Button>,
    btn_pos: usize,
    max_btn_pos: usize,
    #[derivative(Debug="ignore")]
    text: Option<Texture>,
    rect: Option<Rect>,
}

impl Tab {
    fn draw(&mut self, canvas: &mut Canvas<sdl2::video::Window>, selected: bool) {
        if selected {
            let bottom = self.rect.unwrap().height() as i32;

            canvas.set_draw_color(Color::RGB(255, 255, 255));
            canvas.draw_line((0, bottom), (640, bottom)).unwrap();
            let old = canvas.viewport();
            let new = Rect::new(0, bottom, 640, (480 - bottom) as u32);
            canvas.set_viewport(new);

            for (i, btn) in self.buttons.iter_mut().enumerate() {
                btn.draw(canvas, self.btn_pos == i);
            }
            canvas.set_viewport(old);

            self.text.as_mut().unwrap().set_color_mod(255, 0, 0);
        } else {
            self.text.as_mut().unwrap().set_color_mod(255, 255, 255);
        }
        canvas.copy(self.text.as_ref().unwrap(), None, self.rect);
    }
}

#[derive(Debug, PartialEq)]
enum InternalTkEvent {
    ChangeTabPos(i32),
    ChangeBtnPos(i32),
    Press,
    AnimationUpdate,
    Quit,
    Dummy,
}

#[derive(Debug, PartialEq)]
pub enum TkEvent {
    ButtonSelect(String),
    ButtonPress(String),
    TabChange(String),
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Toolkit {
    tabs: Vec<Tab>,
    tab_pos: usize,
    max_tab_pos: usize,
    #[derivative(Debug="ignore")]
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    run: bool,

    #[derivative(Debug="ignore")]
    event_pump: sdl2::EventPump,
    #[derivative(Debug="ignore")]
    event_sender: EventSender,

    tk_event_queue: VecDeque<TkEvent>,
}

impl Toolkit {
    pub fn tick(&mut self) -> bool {
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();
        for (i, tab) in self.tabs.iter_mut().enumerate() {
            tab.draw(&mut self.canvas, self.tab_pos == i);
        }
        self.canvas.present();

        // blocking!
        let ev = self.event_pump.wait_event();

        if ev.is_user_event() {
            let tk_ev = ev.as_user_event_type::<InternalTkEvent>().unwrap();
            match tk_ev {
                InternalTkEvent::ChangeTabPos(p) => {
                    let new_pos = clamp(self.tab_pos as i32 + p, 0, self.max_tab_pos as i32) as usize;
                    if new_pos != self.tab_pos {
                        self.tab_pos = new_pos;
                        self.tk_event_queue.push_back(TkEvent::TabChange(self.cur_tab().unwrap().name.to_string()));
                    }
                }
                InternalTkEvent::ChangeBtnPos(p) => 
                    if let Some(mut tab) = self.cur_mut_tab() {
                        let new_pos = clamp(tab.btn_pos as i32 + p, 0, tab.max_btn_pos as i32) as usize;
                        if new_pos != tab.btn_pos {
                            tab.btn_pos = new_pos;
                            self.tk_event_queue.push_back(TkEvent::ButtonSelect(self.cur_btn().unwrap().name.to_string()));
                        }
                    },
                InternalTkEvent::Press =>
                    if let Some(btn) = self.cur_btn() {
                        self.tk_event_queue.push_back(TkEvent::ButtonPress(btn.name.to_string()));
                    }
                InternalTkEvent::AnimationUpdate => todo!("animations"),
                InternalTkEvent::Quit =>            self.run = false,
                InternalTkEvent::Dummy => (),
            }
        } else {
            let out = match ev {
                Event::Quit{..} => InternalTkEvent::Quit,
                Event::KeyDown {keycode, ..} => {
                    match keycode {
                        Some(Keycode::Escape) =>    InternalTkEvent::Quit,
                        Some(Keycode::Up) =>        InternalTkEvent::ChangeBtnPos(-1),
                        Some(Keycode::Down) =>      InternalTkEvent::ChangeBtnPos(1),
                        Some(Keycode::Left) =>      InternalTkEvent::ChangeTabPos(-1),
                        Some(Keycode::Right) =>     InternalTkEvent::ChangeTabPos(1),
                        Some(Keycode::Return) =>    InternalTkEvent::Press,
                        _ => InternalTkEvent::Dummy,
                    }
                }
                _ => InternalTkEvent::Dummy,
            };

            if out != InternalTkEvent::Dummy {
                self.event_sender.push_custom_event(out).unwrap();
            }
        }

        self.run
    }

    pub fn builder(name: &'static str) -> ToolkitBuilder {
        ToolkitBuilder::new(name)
    }

    pub fn poll_events(&mut self) -> Option<TkEvent> {
        self.tk_event_queue.pop_front()
    }

    fn cur_mut_tab(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.tab_pos)
    }
    fn cur_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.tab_pos)
    }
    fn cur_btn(&self) -> Option<&Button> {
        if let Some(tab) = self.tabs.get(self.tab_pos) {
            tab.buttons.get(tab.btn_pos)
        } else {
            None
        }
    }
}

// Input:
fn handle_inputs(sdl_tx: EventSender) {
    loop {
        thread::sleep(std::time::Duration::from_secs(1));
        sdl_tx.push_custom_event(InternalTkEvent::Dummy).unwrap();
    }
    // it's much faster to use raw linux evdev apis
    // than sdl2 gamecontroller api
    // find rinputer(or close enough) device:
    // open it and convert events into sdl input events:
}


// Initialization:

pub struct ToolkitBuilder {
    name: &'static str,
    tabs: Vec<Tab>,
    ttf: sdl2::ttf::Sdl2TtfContext,
    canvas: Canvas<sdl2::video::Window>,
    text_creator: sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    event_pump: sdl2::EventPump,
    event_sender: EventSender,
    event_sender_2: Option<EventSender>,
    newtab_offset: u32,
}

impl ToolkitBuilder {
    pub fn new(name: &'static str) -> ToolkitBuilder {
        let sdl2_ctx = sdl2::init().unwrap();
        let video = sdl2_ctx.video().unwrap();
        let window = video.window(name, 640, 480).build().unwrap();
        let canvas = window.into_canvas().present_vsync().build().unwrap();
        let text_creator = canvas.texture_creator();

        let ev = sdl2_ctx.event().unwrap();
        ev.register_custom_event::<InternalTkEvent>().unwrap();
        let event_sender = ev.event_sender();
        let event_sender_2 = Some(ev.event_sender());
        let event_pump = sdl2_ctx.event_pump().unwrap();

        ToolkitBuilder {
            ttf: sdl2::ttf::init().unwrap(),
            canvas,
            text_creator,
            name,
            event_pump,
            event_sender,
            event_sender_2,
            newtab_offset: 0,
            tabs: Vec::new(),
        }
    }
    pub fn tab(self, name: &'static str) -> TabBuilder {
        TabBuilder {
            name,
            buttons: Vec::new(),
            builder: self,
        }
    }
    fn render_text(&mut self, input: &'static str) -> Texture {
        let font = self.ttf.load_font("/usr/share/fonts/liberation/LiberationSans-Regular.ttf", 28).unwrap();
        let surface = font.render(input).blended(Color::RGBA(255, 255, 255, 255)).unwrap();
        self.text_creator.create_texture_from_surface(&surface).unwrap()
    }
}

pub struct TabBuilder {
    name: &'static str,
    buttons: Vec<Button>,
    builder: ToolkitBuilder,
}

impl TabBuilder {
    pub fn button(mut self, name: &'static str) -> TabBuilder {
        let text = self.builder.render_text(name);
        self.buttons.push(Button::new(name, self.buttons.len(), text));
        self
    }
    pub fn buttons_vec(mut self, names: Vec<&'static str>) -> TabBuilder {
        for name in names {
            let text = self.builder.render_text(name);
            self.buttons.push(Button::new(name, self.buttons.len(), text));
        }
        self
    }
    pub fn tab(mut self, name: &'static str) -> TabBuilder {
        let max_btn_pos = self.buttons.len() - 1;
        let text = self.builder.render_text(self.name);
        let attr = text.query();
        let rect = Rect::new(self.builder.newtab_offset as i32, 0, attr.width, attr.height);
        self.builder.newtab_offset += attr.width + 10;
        self.builder.tabs.push(Tab {
            name: self.name,
            buttons: self.buttons,
            btn_pos: 0,
            max_btn_pos,
            text: Some(text),
            rect: Some(rect),
        });
        self.builder.tab(name)
    }
    pub fn build(mut self) -> Toolkit {
        let max_btn_pos = self.buttons.len() - 1;
        let text = self.builder.render_text(self.name);
        let attr = text.query();
        let rect = Rect::new(self.builder.newtab_offset as i32, 0, attr.width, attr.height);
        self.builder.newtab_offset += attr.width + 10;

        self.builder.tabs.push(Tab {
            name: self.name,
            buttons: self.buttons,
            btn_pos: 0,
            max_btn_pos,
            text: Some(text),
            rect: Some(rect),
        });

        if let Some(sender) = self.builder.event_sender_2.take() {
            thread::spawn(move || handle_inputs(sender));
        } else {
            unreachable!();
        }

        let max_tab_pos = self.builder.tabs.len() - 1;
        Toolkit {
            run: true,
            canvas: self.builder.canvas,
            tabs: self.builder.tabs,
            event_pump: self.builder.event_pump,
            event_sender: self.builder.event_sender,
            tab_pos: 0,
            max_tab_pos,
            tk_event_queue: VecDeque::new(),
        }
    }
}
