use crate::{
    render::Renderer,
    ui::{
        self,
        img_ids::{ImageGraphic, VoxelGraphic},
        ScaleMode, Ui,
    },
    GlobalState,
};
use conrod_core::{
    color,
    color::TRANSPARENT,
    position::Relative,
    widget::{text_box::Event as TextBoxEvent, Button, Image, List, Rectangle, Text, TextBox},
    widget_ids, Borderable, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};

widget_ids! {
    struct Ids {
        // Background and logo
        bg,
        v_logo,
        alpha_version,
        // Disclaimer
        disc_window,
        disc_text_1,
        disc_text_2,
        disc_button,
        disc_scrollbar,
        // Login, Singleplayer
        login_button,
        login_text,
        login_error,
        login_error_bg,
        address_text,
        address_bg,
        address_field,
        username_text,
        username_bg,
        username_field,
        singleplayer_button,
        singleplayer_text,
        usrnm_bg,
        srvr_bg,
        // Server list
        servers_button,
        servers_frame,
        servers_text,
        servers_close,
        // Buttons
        settings_button,
        quit_button,
        // Error
        error_frame,
        button_ok,
        version,
    }
}

image_ids! {
    struct Imgs {
        <VoxelGraphic>
        v_logo: "voxygen/element/v_logo.vox",
        input_bg: "voxygen/element/misc_bg/textbox.vox",
        button: "voxygen/element/buttons/button.vox",
        button_hover: "voxygen/element/buttons/button_hover.vox",
        button_press: "voxygen/element/buttons/button_press.vox",
        disclaimer: "voxygen/element/frames/disclaimer.vox",

        <ImageGraphic>
        bg: "voxygen/background/bg_main.png",
        error_frame: "voxygen/element/frames/window_2.png",
        nothing: "voxygen/element/nothing.png",
    }

}

font_ids! {
    pub struct Fonts {
        opensans: "voxygen/font/OpenSans-Regular.ttf",
        metamorph: "voxygen/font/Metamorphous-Regular.ttf",
    }
}

pub enum Event {
    LoginAttempt {
        username: String,
        server_address: String,
    },
    StartSingleplayer,
    Quit,
    Settings,
    DisclaimerClosed,
}

pub struct MainMenuUi {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    fonts: Fonts,
    username: String,
    server_address: String,
    login_error: Option<String>,
    connecting: Option<std::time::Instant>,
    show_servers: bool,
    show_disclaimer: bool,
}

impl MainMenuUi {
    pub fn new(global_state: &mut GlobalState) -> Self {
        let window = &mut global_state.window;
        let networking = &global_state.settings.networking;
        let mut ui = Ui::new(window).unwrap();
        // TODO: adjust/remove this, right now it is used to demonstrate window scaling functionality
        ui.scaling_mode(ScaleMode::RelativeToWindow([1920.0, 1080.0].into()));
        // Generate ids
        let ids = Ids::new(ui.id_generator());
        // Load images
        let imgs = Imgs::load(&mut ui).expect("Failed to load images");
        // Load fonts
        let fonts = Fonts::load(&mut ui).expect("Failed to load fonts");

        Self {
            ui,
            ids,
            imgs,
            fonts,
            username: networking.username.clone(),
            server_address: networking.servers[networking.default_server].clone(),
            login_error: None,
            connecting: None,
            show_servers: false,
            show_disclaimer: global_state.settings.show_disclaimer,
        }
    }

    fn update_layout(&mut self, global_state: &mut GlobalState) -> Vec<Event> {
        let mut events = Vec::new();
        let ref mut ui_widgets = self.ui.set_widgets();
        let version = env!("CARGO_PKG_VERSION");
        const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
        const TEXT_COLOR_2: Color = Color::Rgba(1.0, 1.0, 1.0, 0.2);
        // Background image, Veloren logo, Alpha-Version Label
        Image::new(self.imgs.bg)
            .middle_of(ui_widgets.window)
            .set(self.ids.bg, ui_widgets);
        Image::new(self.imgs.v_logo)
            .w_h(123.0 * 3.0, 35.0 * 3.0)
            .top_left_with_margins(30.0, 30.0)
            .set(self.ids.v_logo, ui_widgets);
        Text::new(version)
            .top_left_with_margins_on(ui_widgets.window, 5.0, 5.0)
            .font_size(14)
            .font_id(self.fonts.opensans)
            .color(TEXT_COLOR)
            .set(self.ids.version, ui_widgets);

        if self.show_disclaimer {
            Image::new(self.imgs.disclaimer)
                .w_h(1800.0, 800.0)
                .middle_of(ui_widgets.window)
                .scroll_kids()
                .scroll_kids_vertically()
                .set(self.ids.disc_window, ui_widgets);

            Text::new("Disclaimer")
                .top_left_with_margins_on(self.ids.disc_window, 30.0, 40.0)
                .font_id(self.fonts.metamorph)
                .font_size(35)
                .font_id(self.fonts.metamorph)
                .color(TEXT_COLOR)
                .set(self.ids.disc_text_1, ui_widgets);
            Text::new(
            "Welcome to the alpha version of Veloren!\n\
            \n\
            \n\
            Before you dive into the fun, please keep a few things in mind:\n\
            \n\
            - This is a very early alpha. Expect bugs, extremely unfinished gameplay, unpolished mechanics, and missing features. \n\
            If you are a reviewer, please DO NOT review this version.\n\
            \n\
            - If you have constructive feedback or bug reports, you can contact us via Reddit, GitLab, or our community Discord server.\n\
            \n\
            - Veloren is licensed under the GPL 3 open-source licence. That means you're free to play, modify, and redistribute the game however you wish \n\
            (provided derived work is also under GPL 3).
            \n\
            - Veloren is a non-profit community project, and everybody working on it is a volunteer.\n\
            If you like what you see, you're welcome to join the development or art teams!
            \n\
            - 'Voxel RPG' is a genre in its own right. First-person shooters used to be called Doom clones.\n\
            Like them, we're trying to build a niche. This game is not a clone, and its development will diverge from existing games in the future.\n\
            \n\
            Thanks for taking the time to read this notice, we hope you enjoy the game!\n\
            \n\
            ~ The Veloren Devs")
            .top_left_with_margins_on(self.ids.disc_window, 110.0, 40.0)
            .font_id(self.fonts.metamorph)
            .font_size(26)
            .font_id(self.fonts.opensans)
            .color(TEXT_COLOR)
            .set(self.ids.disc_text_2, ui_widgets);
            if Button::image(self.imgs.button)
                .w_h(300.0, 50.0)
                .mid_bottom_with_margin_on(self.ids.disc_window, 30.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label_y(Relative::Scalar(2.0))
                .label("Accept")
                .label_font_size(18)
                .label_color(TEXT_COLOR)
                .set(self.ids.disc_button, ui_widgets)
                .was_clicked()
            {
                self.show_disclaimer = false;
                events.push(Event::DisclaimerClosed);
            }
        } else {
            // TODO: Don't use macros for this?
            // Input fields
            // Used when the login button is pressed, or enter is pressed within input field
            macro_rules! login {
                () => {
                    self.login_error = None;
                    self.connecting = Some(std::time::Instant::now());
                    events.push(Event::LoginAttempt {
                        username: self.username.clone(),
                        server_address: self.server_address.clone(),
                    });
                };
            }

            // Singleplayer
            // Used when the singleplayer button is pressed
            macro_rules! singleplayer {
                () => {
                    self.login_error = None;
                    events.push(Event::StartSingleplayer);
                    events.push(Event::LoginAttempt {
                        username: "singleplayer".to_string(),
                        server_address: "localhost".to_string(),
                    });
                };
            }

            // Username
            Rectangle::fill_with([320.0, 50.0], color::rgba(0.0, 0.0, 0.0, 0.97))
                .middle_of(ui_widgets.window)
                .set(self.ids.usrnm_bg, ui_widgets);
            Image::new(self.imgs.input_bg)
                .w_h(337.0, 67.0)
                .middle_of(self.ids.usrnm_bg)
                .set(self.ids.username_bg, ui_widgets);
            for event in TextBox::new(&self.username)
                .w_h(290.0, 30.0)
                .mid_bottom_with_margin_on(self.ids.username_bg, 44.0 / 2.0)
                .font_size(22)
                .font_id(self.fonts.opensans)
                .text_color(TEXT_COLOR)
                // transparent background
                .color(TRANSPARENT)
                .border_color(TRANSPARENT)
                .set(self.ids.username_field, ui_widgets)
            {
                match event {
                    TextBoxEvent::Update(username) => {
                        // Note: TextBox limits the input string length to what fits in it
                        self.username = username.to_string();
                    }
                    TextBoxEvent::Enter => {
                        login!();
                    }
                }
            }
            // Login error
            if let Some(msg) = &self.login_error {
                let text = Text::new(&msg)
                    .rgba(1.0, 1.0, 1.0, 1.0)
                    .font_size(30)
                    .font_id(self.fonts.opensans);
                Rectangle::fill_with([400.0, 100.0], color::TRANSPARENT)
                    .rgba(0.1, 0.1, 0.1, 1.0)
                    .parent(ui_widgets.window)
                    .mid_top_with_margin_on(self.ids.username_bg, -35.0)
                    .set(self.ids.login_error_bg, ui_widgets);
                Image::new(self.imgs.error_frame)
                    .w_h(400.0, 100.0)
                    .middle_of(self.ids.login_error_bg)
                    .set(self.ids.error_frame, ui_widgets);
                text.mid_top_with_margin_on(self.ids.error_frame, 10.0)
                    .set(self.ids.login_error, ui_widgets);
                if Button::image(self.imgs.button)
                    .w_h(100.0, 30.0)
                    .mid_bottom_with_margin_on(self.ids.login_error_bg, 5.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label_y(Relative::Scalar(2.0))
                    .label("Okay")
                    .label_font_size(10)
                    .label_color(TEXT_COLOR)
                    .set(self.ids.button_ok, ui_widgets)
                    .was_clicked()
                {
                    self.login_error = None
                };
            }
            if self.show_servers {
                Image::new(self.imgs.error_frame)
                    .mid_top_with_margin_on(self.ids.username_bg, -320.0)
                    .w_h(400.0, 300.0)
                    .set(self.ids.servers_frame, ui_widgets);

                let net_settings = &global_state.settings.networking;

                // TODO: Draw scroll bar or remove it.
                let (mut items, _scrollbar) = List::flow_down(net_settings.servers.len())
                    .top_left_with_margins_on(self.ids.servers_frame, 0.0, 5.0)
                    .w_h(400.0, 300.0)
                    .scrollbar_next_to()
                    .scrollbar_thickness(18.0)
                    .scrollbar_color(TEXT_COLOR)
                    .set(self.ids.servers_text, ui_widgets);

                while let Some(item) = items.next(ui_widgets) {
                    let mut text = "".to_string();
                    if &net_settings.servers[item.i] == &self.server_address {
                        text.push_str("-> ")
                    } else {
                        text.push_str("  ")
                    }
                    text.push_str(&net_settings.servers[item.i]);

                    if item
                        .set(
                            Button::image(self.imgs.nothing)
                                .w_h(100.0, 50.0)
                                .mid_top_with_margin_on(self.ids.servers_frame, 10.0)
                                //.hover_image(self.imgs.button_hover)
                                //.press_image(self.imgs.button_press)
                                .label_y(Relative::Scalar(2.0))
                                .label(&text)
                                .label_font_size(20)
                                .label_color(TEXT_COLOR),
                            ui_widgets,
                        )
                        .was_clicked()
                    {
                        // TODO: Set as current server address
                        self.server_address = net_settings.servers[item.i].clone();
                    }
                }

                if Button::image(self.imgs.button)
                    .w_h(200.0, 53.0)
                    .mid_bottom_with_margin_on(self.ids.servers_frame, 5.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label_y(Relative::Scalar(2.0))
                    .label("Close")
                    .label_font_size(20)
                    .label_color(TEXT_COLOR)
                    .set(self.ids.servers_close, ui_widgets)
                    .was_clicked()
                {
                    self.show_servers = false
                };
            }
            // Server address
            Rectangle::fill_with([320.0, 50.0], color::rgba(0.0, 0.0, 0.0, 0.97))
                .down_from(self.ids.usrnm_bg, 30.0)
                .set(self.ids.srvr_bg, ui_widgets);
            Image::new(self.imgs.input_bg)
                .w_h(337.0, 67.0)
                .middle_of(self.ids.srvr_bg)
                .set(self.ids.address_bg, ui_widgets);
            for event in TextBox::new(&self.server_address)
                .w_h(290.0, 30.0)
                .mid_bottom_with_margin_on(self.ids.address_bg, 44.0 / 2.0)
                .font_size(22)
                .font_id(self.fonts.opensans)
                .text_color(TEXT_COLOR)
                // transparent background
                .color(TRANSPARENT)
                .border_color(TRANSPARENT)
                .set(self.ids.address_field, ui_widgets)
            {
                match event {
                    TextBoxEvent::Update(server_address) => {
                        self.server_address = server_address.to_string();
                    }
                    TextBoxEvent::Enter => {
                        login!();
                    }
                }
            }
            // Login button
            // Change button text and remove hover/press images if a connection is in progress
            if let Some(start) = self.connecting {
                Button::image(self.imgs.button)
                    .w_h(258.0, 55.0)
                    .down_from(self.ids.address_bg, 20.0)
                    .align_middle_x_of(self.ids.address_bg)
                    .label("Connecting...")
                    .label_color({
                        let pulse =
                            ((start.elapsed().as_millis() as f32 * 0.008).sin() + 1.0) / 2.0;
                        Color::Rgba(
                            TEXT_COLOR.red() * (pulse / 2.0 + 0.5),
                            TEXT_COLOR.green() * (pulse / 2.0 + 0.5),
                            TEXT_COLOR.blue() * (pulse / 2.0 + 0.5),
                            pulse / 4.0 + 0.75,
                        )
                    })
                    .label_font_size(22)
                    .label_y(Relative::Scalar(5.0))
                    .set(self.ids.login_button, ui_widgets);
            } else {
                if Button::image(self.imgs.button)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .w_h(258.0, 55.0)
                    .down_from(self.ids.address_bg, 20.0)
                    .align_middle_x_of(self.ids.address_bg)
                    .label("Login")
                    .label_color(TEXT_COLOR)
                    .label_font_size(24)
                    .label_y(Relative::Scalar(5.0))
                    .set(self.ids.login_button, ui_widgets)
                    .was_clicked()
                {
                    login!();
                }
            };

            // Singleplayer button
            if Button::image(self.imgs.button)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .w_h(258.0, 55.0)
                .down_from(self.ids.login_button, 20.0)
                .align_middle_x_of(self.ids.address_bg)
                .label("Singleplayer")
                .label_color(TEXT_COLOR)
                .label_font_size(22)
                .label_y(Relative::Scalar(5.0))
                .label_x(Relative::Scalar(2.0))
                .set(self.ids.singleplayer_button, ui_widgets)
                .was_clicked()
            {
                singleplayer!();
            }
            // Quit
            if Button::image(self.imgs.button)
                .w_h(190.0, 40.0)
                .bottom_left_with_margins_on(ui_widgets.window, 60.0, 30.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label("Quit")
                .label_color(TEXT_COLOR)
                .label_font_size(20)
                .label_y(Relative::Scalar(3.0))
                .set(self.ids.quit_button, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Quit);
            }

            // Settings
            if Button::image(self.imgs.button)
                .w_h(190.0, 40.0)
                .up_from(self.ids.quit_button, 8.0)
                //.hover_image(self.imgs.button_hover)
                //.press_image(self.imgs.button_press)
                .label("Settings")
                .label_color(TEXT_COLOR_2)
                .label_font_size(20)
                .label_y(Relative::Scalar(3.0))
                .set(self.ids.settings_button, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Settings);
            }

            // Servers
            if Button::image(self.imgs.button)
                .w_h(190.0, 40.0)
                .up_from(self.ids.settings_button, 8.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label("Servers")
                .label_color(TEXT_COLOR)
                .label_font_size(20)
                .label_y(Relative::Scalar(3.0))
                .set(self.ids.servers_button, ui_widgets)
                .was_clicked()
            {
                self.show_servers = true;
            };
        }

        events
    }

    pub fn login_error(&mut self, msg: String) {
        self.login_error = Some(msg);
        self.connecting = None;
    }

    pub fn connected(&mut self) {
        self.connecting = None;
    }

    pub fn handle_event(&mut self, event: ui::Event) {
        self.ui.handle_event(event);
    }

    pub fn maintain(&mut self, global_state: &mut GlobalState) -> Vec<Event> {
        let events = self.update_layout(global_state);
        self.ui.maintain(global_state.window.renderer_mut(), None);
        events
    }

    pub fn render(&self, renderer: &mut Renderer) {
        self.ui.render(renderer, None);
    }
}
