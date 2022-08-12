use std::thread;
use std::num::Wrapping;
use std::collections::VecDeque;
use derivative::Derivative;

use sdl2::{
    rect::{
        Rect,
        Point,
    },
    render::{Canvas, Texture},
    pixels::Color,
    event::{Event, EventSender},
    keyboard::Keycode,
    mouse::MouseButton,
};

#[inline]
fn remap(x: i32, min: i32, max: i32, outmin: i32, outmax: i32) -> i32 {
    (Wrapping(x - min) * Wrapping(outmax - outmin) / Wrapping(max - min) + Wrapping(outmin)).0
}

fn clamp<T: std::cmp::PartialOrd>(x: T, min: T, max: T) -> T {
    if x < min { return min }
    if x > max { return max }
    return x
}

trait Buttonish {
    fn draw(&mut self, canvas: &mut Canvas<sdl2::video::Window>, selected: bool, offset_y: i32);
    fn captures_input(&self) -> bool;
    fn action(&mut self, ev: &InternalTkEvent) -> Option<TkEvent>;
    fn name(&self) -> &str;
    fn rect(&self) -> Rect;
}

impl core::fmt::Debug for dyn Buttonish {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("dyn Buttonish")
            .field("captures_input", &self.captures_input())
            .field("name", &self.name())
            .finish()
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct Slider {
    name: &'static str,
    #[derivative(Debug="ignore")]
    text: Option<Texture>,
    rect: Option<Rect>,
    outline_rect: Rect,
    min: i32,
    max: i32,
    level: i32,
    grabbed: bool,
}

impl Slider {
    fn new(name: &'static str, text: Texture, line: usize, init: i32, min: i32, max: i32) -> Slider {
        let attr = text.query();
        let rect = Rect::new(0, (line as u32 * attr.height) as i32, attr.width, attr.height);
        Slider {
            name,
            text: Some(text),
            rect: Some(rect),
            outline_rect: Rect::new((attr.width + 5) as i32, (line as u32 * attr.height) as i32, attr.height*5, attr.height),
            level: init,
            min, max,
            grabbed: false,
        }
    }
}

impl Buttonish for Slider {
    fn draw(&mut self, canvas: &mut Canvas<sdl2::video::Window>, selected: bool, offset_y: i32) {
        if selected {
            self.text.as_mut().unwrap().set_color_mod(255, 0, 0);
        } else {
            self.text.as_mut().unwrap().set_color_mod(255, 255, 255);
        }

        let mut rect = self.rect.unwrap().clone();
        rect.set_y(rect.y() + offset_y);
        canvas.copy(self.text.as_ref().unwrap(), None, rect).unwrap();


        self.outline_rect.set_y(rect.y());
        if self.grabbed {
            canvas.set_draw_color(Color::RGB(255, 0, 0));
        } else {
            canvas.set_draw_color(Color::RGB(255, 255, 255));
        }
        canvas.draw_rect(self.outline_rect);

        if self.level != self.min {
            let mut content_rect = self.outline_rect.clone();
            content_rect.set_width(remap(self.level, self.min, self.max, 0, self.outline_rect.width() as i32) as u32);
            content_rect.set_y(rect.y());
            canvas.fill_rect(content_rect);
        }
    }
    fn captures_input(&self) -> bool { true }
    fn action(&mut self, ev: &InternalTkEvent) -> Option<TkEvent> {
        self.grabbed = true;
        return match ev {
            InternalTkEvent::ChangeTabPos(d) => {
                let new = clamp(self.level + d, self.min, self.max);
                if new != self.level {
                    self.level = new;
                    Some(TkEvent::SliderChange(self.name().to_string(), self.level, self.min, self.max))
                } else {
                    Some(TkEvent::None)
                }
            },
            InternalTkEvent::Press => {
                self.grabbed = false;
                None
            },
            _ => Some(TkEvent::None),
        }
    }
    fn name(&self) -> &str { self.name }
    fn rect(&self) -> Rect { self.rect.unwrap() }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct Toggle {
    name: &'static str,
    #[derivative(Debug="ignore")]
    text: Option<Texture>,
    rect: Option<Rect>,
    state_rect: Rect,
    state: bool,
}

impl Buttonish for Toggle {
    fn draw(&mut self, canvas: &mut Canvas<sdl2::video::Window>, selected: bool, offset_y: i32) {
        if selected {
            self.text.as_mut().unwrap().set_color_mod(255, 0, 0);
        } else {
            self.text.as_mut().unwrap().set_color_mod(255, 255, 255);
        }

        let mut rect = self.rect.unwrap().clone();
        rect.set_y(rect.y() + offset_y);

        canvas.copy(self.text.as_ref().unwrap(), None, rect).unwrap();

        canvas.set_draw_color(Color::RGB(255, 255, 255));
        self.state_rect.set_y(rect.y());
        if self.state == true {
            canvas.fill_rect(self.state_rect);
        } else {
            canvas.draw_rect(self.state_rect);
        }
    }
    fn captures_input(&self) -> bool { false }
    fn action(&mut self, _: &InternalTkEvent) -> Option<TkEvent> {
        self.state = !self.state;
        Some(TkEvent::ToggleChange(self.name().to_string(), self.state))
    }
    fn name(&self) -> &str { self.name }
    fn rect(&self) -> Rect { self.rect.unwrap() }
}

impl Toggle {
    fn new(name: &'static str, line: usize, text: Texture) -> Toggle {
        let attr = text.query();
        let rect = Rect::new(0, (line as u32 * attr.height) as i32, attr.width, attr.height);
        let state_rect = Rect::new((attr.width + 5) as i32, (line as u32 * attr.height) as i32, attr.height, attr.height);
        Toggle {
            name,
            text: Some(text),
            rect: Some(rect),
            state_rect,
            state: false,
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct Button {
    name: &'static str,
    #[derivative(Debug="ignore")]
    text: Option<Texture>,
    rect: Option<Rect>,
}

impl Buttonish for Button {
    fn draw(&mut self, canvas: &mut Canvas<sdl2::video::Window>, selected: bool, offset_y: i32) {
        if selected {
            self.text.as_mut().unwrap().set_color_mod(255, 0, 0);
        } else {
            self.text.as_mut().unwrap().set_color_mod(255, 255, 255);
        }

        let mut rect = self.rect.unwrap().clone();
        rect.set_y(rect.y() + offset_y);
        canvas.copy(self.text.as_ref().unwrap(), None, rect).unwrap();
    }
    fn captures_input(&self) -> bool { false }
    fn action(&mut self, _: &InternalTkEvent) -> Option<TkEvent> {
        Some(TkEvent::ButtonPress(self.name().to_string()))
    }
    fn name(&self) -> &str { self.name }
    fn rect(&self) -> Rect { self.rect.unwrap() }
}

impl Button {
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
    buttons: Vec<Box<dyn Buttonish>>,
    btn_pos: usize,
    max_btn_pos: usize,
    #[derivative(Debug="ignore")]
    text: Option<Texture>,
    rect: Option<Rect>,
}

impl Tab {
    fn draw(&mut self, canvas: &mut Canvas<sdl2::video::Window>, selected: bool, y_offset: i32) {
        if selected {
            let bottom = self.rect.unwrap().height() as i32;

            canvas.set_draw_color(Color::RGB(255, 255, 255));
            canvas.draw_line((0, bottom), (640, bottom)).unwrap();
            let old = canvas.viewport();
            let new = Rect::new(0, bottom, 640, (480 - bottom) as u32);
            canvas.set_viewport(new);

            for (i, btn) in self.buttons.iter_mut().enumerate() {
                btn.draw(canvas, self.btn_pos == i, y_offset);
            }
            canvas.set_viewport(old);

            self.text.as_mut().unwrap().set_color_mod(255, 0, 0);
        } else {
            self.text.as_mut().unwrap().set_color_mod(255, 255, 255);
        }
        canvas.copy(self.text.as_ref().unwrap(), None, self.rect);
    }
    fn cur_btn(&self) -> Option<&Box<dyn Buttonish>> {
        self.buttons.get(self.btn_pos)
    }
    fn cur_mut_btn(&mut self) -> Option<&mut Box<dyn Buttonish>> {
        self.buttons.get_mut(self.btn_pos)
    }
}

#[derive(Debug, PartialEq)]
enum InternalTkEvent {
    ChangeTabPos(i32),
    ChangeBtnPos(i32),
    Press,
    TouchPress(i32, i32),
    SetOffsetY(i32),
    AppendOffsetY(i32),
    Quit,
    Dummy,
}

#[derive(Debug, PartialEq)]
pub enum TkEvent {
    ButtonSelect(String),
    ButtonPress(String),
    SliderChange(String, i32, i32, i32),
    ToggleChange(String, bool),
    TabChange(String),
    None,
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

    old_xy: Option<(i32, i32)>,
    y_offset: i32,
    y_velocity: i32,
    tk_event_queue: VecDeque<TkEvent>,
    redirect_input: bool,
    line_height: i32,
}

impl Toolkit {
    pub fn tick(&mut self) -> bool {
        let mut redraw = false;
        // blocking!
        //let ev = self.event_pump.wait_event();
        let mut events: Vec<InternalTkEvent> = Vec::new();
        for ev in self.event_pump.poll_iter() {
            if ev.is_user_event() {
                events.push(ev.as_user_event_type::<InternalTkEvent>().unwrap());
            } else {
                let out = match ev {
                    Event::Quit {..} => InternalTkEvent::Quit,
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
                    Event::MouseMotion {x, y, yrel, mousestate, ..} => {
                        if mousestate.left() {
                            if yrel == 0 {
                                InternalTkEvent::TouchPress(x, y)
                            } else {
                                InternalTkEvent::AppendOffsetY(yrel)
                            }
                        } else {
                            InternalTkEvent::Dummy
                        }
                    }
                    Event::MouseButtonDown{x, y, mouse_btn, ..} => {
                        if mouse_btn == MouseButton::Left {
                            InternalTkEvent::TouchPress(x, y)
                        } else {
                            InternalTkEvent::Dummy
                        }
                    }
                    _ => InternalTkEvent::Dummy,
                };

                if out != InternalTkEvent::Dummy {
                    events.push(out);
                }
            }
        }

        for tk_ev in events {
            if self.redirect_input && tk_ev != InternalTkEvent::Quit {
                if let Some(btn) = self.cur_mut_btn() {
                    if let Some(new_ev) = btn.action(&tk_ev) {
                        self.tk_event_queue.push_back(new_ev);
                    } else {
                        self.redirect_input = false;
                    }
                    redraw = true;
                }
            } else {
                match tk_ev {
                    InternalTkEvent::ChangeTabPos(p) => {
                        let new_pos = clamp(self.tab_pos as i32 + p, 0, self.max_tab_pos as i32) as usize;
                        if new_pos != self.tab_pos {
                            self.y_offset = 0;
                            self.tab_pos = new_pos;
                            self.tk_event_queue.push_back(TkEvent::TabChange(self.cur_tab().unwrap().name.to_string()));
                            redraw = true;
                        };
                    },
                    InternalTkEvent::ChangeBtnPos(p) => {
                        if let Some(mut tab) = self.cur_mut_tab() {
                            let new_pos = clamp(tab.btn_pos as i32 + p, 0, tab.max_btn_pos as i32) as usize;
                            if new_pos != tab.btn_pos {
                                tab.btn_pos = new_pos;
                                let bottom = if let Some(btn) = self.cur_tab().unwrap().buttons.last() {
                                    btn.rect().bottom()
                                } else { unreachable!(); };

                                self.y_offset = clamp(self.y_offset - self.line_height * p, -1*(bottom - (480 - self.line_height)), 0);
                                self.tk_event_queue.push_back(TkEvent::ButtonSelect(self.cur_btn().unwrap().name().to_string()));
                                redraw = true;
                            }
                        };
                    },
                    InternalTkEvent::Press => {
                        if let Some(btn) = self.cur_mut_btn() {
                            if btn.captures_input() {
                                btn.action(&InternalTkEvent::Dummy);
                                self.redirect_input = true;
                            } else {
                                if let Some(new_ev) = btn.action(&tk_ev) {
                                    self.tk_event_queue.push_back(new_ev);
                                }
                            }
                            redraw = true;
                        };
                    },
                    InternalTkEvent::Quit => self.run = false,
                    InternalTkEvent::SetOffsetY(y) => {
                        self.y_offset = y;
                        redraw = true;
                    },
                    InternalTkEvent::AppendOffsetY(y) => {
                        self.y_offset += y;
                        self.y_velocity = y*15;
                        redraw = true;
                    },
                    InternalTkEvent::TouchPress(x, y) => {
                        let adj_y = y - (self.line_height + self.y_offset);
                        if y < self.line_height {
                            let mut new_tab: Option<usize> = None;
                            for (i, candidate) in self.tabs.iter().enumerate() {
                                println!("considering {}", candidate.name);
                                if candidate.rect.unwrap().contains_point(Point::new(x, y)) {
                                    new_tab = Some(i);
                                    break;
                                }
                            }
                            if let Some(id) = new_tab {
                                self.tab_pos = id;
                                self.tk_event_queue.push_back(TkEvent::TabChange(self.cur_tab().unwrap().name.to_string()));
                                redraw = true;
                            }
                        } else {
                            let h = self.line_height;
                            if let Some(mut tab) = self.cur_mut_tab() {
                                let new_pos = clamp(adj_y / h, 0, tab.max_btn_pos as i32) as usize;
                                if new_pos == tab.btn_pos {
                                    if let Some(btn) = self.cur_mut_btn() {
                                        if btn.captures_input() {
                                            self.redirect_input = true;
                                        } else {
                                            if let Some(new_ev) = btn.action(&tk_ev) {
                                                self.tk_event_queue.push_back(new_ev);
                                                redraw = true;
                                            }
                                        }
                                    };
                                } else {
                                    tab.btn_pos = new_pos;
                                    self.tk_event_queue.push_back(TkEvent::ButtonSelect(self.cur_btn().unwrap().name().to_string()));
                                    redraw = true;
                                }
                            }
                        }
                    }
                    InternalTkEvent::Dummy => (),
                }
            }
        }

        if self.y_velocity != 0 {
            self.y_offset += self.y_velocity/15;
            if self.y_velocity > 0 {
                self.y_velocity -= 1;
            } else {
                self.y_velocity += 1;
            }
            redraw = true;
        }

        if self.y_offset > 0 {
            self.y_offset /= 2;
            redraw = true;
        }


        if let Some(btn) = self.cur_tab().unwrap().buttons.last() {
            let bottom = btn.rect().bottom();
            let diff = (480 - self.line_height) - (bottom + self.y_offset);
            if diff > 0 && self.y_offset < -480 {
                self.y_offset += diff/2;
                redraw = true;
            } else if (480 - self.line_height) - bottom > 0 && self.y_offset != 0 {
                self.y_offset = 0;
                redraw = true;
            }
        }

        if redraw {
            self.canvas.set_draw_color(Color::RGB(0, 0, 0));
            self.canvas.clear();
            for (i, tab) in self.tabs.iter_mut().enumerate() {
                tab.draw(&mut self.canvas, self.tab_pos == i, self.y_offset);
            }
            self.canvas.present();
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
    fn cur_mut_btn(&mut self) -> Option<&mut Box<dyn Buttonish>> {
        if let Some(tab) = self.cur_mut_tab() {
            tab.cur_mut_btn()
        } else {
            None
        }
    }
    fn cur_btn(&self) -> Option<&Box<dyn Buttonish>> {
        if let Some(tab) = self.tabs.get(self.tab_pos) {
            tab.cur_btn()
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
    buttons: Vec<Box<dyn Buttonish>>,
    builder: ToolkitBuilder,
}

impl TabBuilder {
    pub fn button(mut self, name: &'static str) -> TabBuilder {
        let text = self.builder.render_text(name);
        self.buttons.push(Box::new(Button::new(name, self.buttons.len(), text)));
        self
    }
    pub fn toggle(mut self, name: &'static str) -> TabBuilder {
        let text = self.builder.render_text(name);
        self.buttons.push(Box::new(Toggle::new(name, self.buttons.len(), text)));
        self
    }
    pub fn slider(mut self, name: &'static str, cur: i32, min: i32, max: i32) -> TabBuilder {
        let text = self.builder.render_text(name);
        self.buttons.push(Box::new(Slider::new(name, text, self.buttons.len(), cur, min, max)));
        self
    }
    pub fn buttons_vec(mut self, names: Vec<&'static str>) -> TabBuilder {
        for name in names {
            let text = self.builder.render_text(name);
            self.buttons.push(Box::new(Button::new(name, self.buttons.len(), text)));
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

        self.builder.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.builder.canvas.clear();
        for (i, tab) in self.builder.tabs.iter_mut().enumerate() {
            tab.draw(&mut self.builder.canvas, i == 0, 0);
        }
        self.builder.canvas.present();

        Toolkit {
            run: true,
            canvas: self.builder.canvas,
            tabs: self.builder.tabs,
            event_pump: self.builder.event_pump,
            event_sender: self.builder.event_sender,
            tab_pos: 0,
            max_tab_pos,
            redirect_input: false,
            tk_event_queue: VecDeque::new(),
            old_xy: None,
            y_offset: 0,
            y_velocity: 0,
            line_height: attr.height as i32,
        }
    }
}
