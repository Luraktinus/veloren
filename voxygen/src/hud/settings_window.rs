use super::{img_ids::Imgs, Fonts, Show, TEXT_COLOR};
use crate::{
    ui::{ImageSlider, ToggleButton},
    GlobalState,
};
use conrod_core::{
    color,
    widget::{self, Button, DropDownList, Image, Rectangle, Scrollbar, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

const FPS_CHOICES: [u32; 11] = [15, 30, 40, 50, 60, 90, 120, 144, 240, 300, 500];

widget_ids! {
    struct Ids {
        settings_content,
        settings_icon,
        settings_button_mo,
        settings_close,
        settings_title,
        settings_r,
        settings_l,
        settings_scrollbar,
        controls_text,
        controls_controls,
        button_help,
        button_help2,
        show_help_label,
        gameplay,
        controls,
        rectangle,
        debug_button,
        debug_button_label,
        interface,
        inventory_test_button,
        inventory_test_button_label,
        mouse_pan_slider,
        mouse_pan_label,
        mouse_pan_value,
        mouse_zoom_slider,
        mouse_zoom_label,
        mouse_zoom_value,
        settings_bg,
        sound,
        test,
        video,
        vd_slider,
        vd_text,
        vd_value,
        max_fps_slider,
        max_fps_text,
        max_fps_value,
        audio_volume_slider,
        audio_volume_text,
        audio_device_list,
        audio_device_text,
    }
}

pub enum SettingsTab {
    Interface,
    Video,
    Sound,
    Gameplay,
    Controls,
}

#[derive(WidgetCommon)]
pub struct SettingsWindow<'a> {
    global_state: &'a GlobalState,

    show: &'a Show,

    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> SettingsWindow<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        show: &'a Show,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
    ) -> Self {
        Self {
            global_state,
            show,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    ToggleHelp,
    ToggleInventoryTestButton,
    ToggleDebug,
    ChangeTab(SettingsTab),
    Close,
    AdjustMousePan(u32),
    AdjustMouseZoom(u32),
    AdjustViewDistance(u32),
    AdjustVolume(f32),
    ChangeAudioDevice(String),
    MaximumFPS(u32),
}

impl<'a> Widget for SettingsWindow<'a> {
    type State = State;
    type Style = ();
    type Event = Vec<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();

        // Frame Alignment
        Rectangle::fill_with([824.0, 488.0], color::TRANSPARENT)
            .middle_of(ui.window)
            .set(state.ids.settings_bg, ui);
        // Frame
        Image::new(self.imgs.settings_frame_l)
            .top_left_with_margins_on(state.ids.settings_bg, 0.0, 0.0)
            .w_h(412.0, 488.0)
            .set(state.ids.settings_l, ui);
        Image::new(self.imgs.settings_frame_r)
            .right_from(state.ids.settings_l, 0.0)
            .parent(state.ids.settings_bg)
            .w_h(412.0, 488.0)
            .set(state.ids.settings_r, ui);
        // Content Alignment
        Rectangle::fill_with([198.0 * 4.0, 97.0 * 4.0], color::TRANSPARENT)
            .top_right_with_margins_on(state.ids.settings_r, 21.0 * 4.0, 4.0 * 4.0)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.settings_content, ui);
        Scrollbar::y_axis(state.ids.settings_content)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(state.ids.settings_scrollbar, ui);
        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(28.0, 28.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.settings_r, 0.0, 0.0)
            .set(state.ids.settings_close, ui)
            .was_clicked()
        {
            events.push(Event::Close);
        }

        // Title
        Text::new("Settings")
            .mid_top_with_margin_on(state.ids.settings_bg, 5.0)
            .font_size(14)
            .color(TEXT_COLOR)
            .set(state.ids.settings_title, ui);

        // 1) Interface Tab -------------------------------
        if Button::image(if let SettingsTab::Interface = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Interface = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Interface = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_press
        })
        .top_left_with_margins_on(state.ids.settings_l, 8.0 * 4.0, 2.0 * 4.0)
        .label("Interface")
        .label_font_size(14)
        .label_color(TEXT_COLOR)
        .set(state.ids.interface, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Interface));
        }

        // Contents
        if let SettingsTab::Interface = self.show.settings_tab {
            // Help
            let show_help =
                ToggleButton::new(self.show.help, self.imgs.check, self.imgs.check_checked)
                    .w_h(288.0 / 24.0, 288.0 / 24.0)
                    .top_left_with_margins_on(state.ids.settings_content, 5.0, 5.0)
                    .hover_images(self.imgs.check_checked_mo, self.imgs.check_mo)
                    .press_images(self.imgs.check_press, self.imgs.check_press)
                    .set(state.ids.button_help, ui);

            if self.show.help != show_help {
                events.push(Event::ToggleHelp);
            }

            Text::new("Show Help")
                .right_from(state.ids.button_help, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.button_help)
                .color(TEXT_COLOR)
                .set(state.ids.show_help_label, ui);

            // Inventory test
            let inventory_test_button = ToggleButton::new(
                self.show.inventory_test_button,
                self.imgs.check,
                self.imgs.check_checked,
            )
            .w_h(288.0 / 24.0, 288.0 / 24.0)
            .down_from(state.ids.button_help, 7.0)
            .hover_images(self.imgs.check_checked_mo, self.imgs.check_mo)
            .press_images(self.imgs.check_press, self.imgs.check_press)
            .set(state.ids.inventory_test_button, ui);

            if self.show.inventory_test_button != inventory_test_button {
                events.push(Event::ToggleInventoryTestButton);
            }

            Text::new("Show Inventory Test Button")
                .right_from(state.ids.inventory_test_button, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.inventory_test_button)
                .color(TEXT_COLOR)
                .set(state.ids.inventory_test_button_label, ui);

            // Debug
            let show_debug =
                ToggleButton::new(self.show.debug, self.imgs.check, self.imgs.check_checked)
                    .w_h(288.0 / 24.0, 288.0 / 24.0)
                    .down_from(state.ids.inventory_test_button, 7.0)
                    .hover_images(self.imgs.check_checked_mo, self.imgs.check_mo)
                    .press_images(self.imgs.check_press, self.imgs.check_press)
                    .set(state.ids.debug_button, ui);

            if self.show.debug != show_debug {
                events.push(Event::ToggleDebug);
            }

            Text::new("Show Debug Window")
                .right_from(state.ids.debug_button, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.debug_button)
                .color(TEXT_COLOR)
                .set(state.ids.debug_button_label, ui);
        }

        // 2) Gameplay Tab --------------------------------
        if Button::image(if let SettingsTab::Gameplay = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Gameplay = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Gameplay = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_press
        })
        .right_from(state.ids.interface, 0.0)
        .label("Gameplay")
        .label_font_size(14)
        .label_color(TEXT_COLOR)
        .set(state.ids.gameplay, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Gameplay));
        }

        // Contents
        if let SettingsTab::Gameplay = self.show.settings_tab {
            let display_pan = self.global_state.settings.gameplay.pan_sensitivity;
            let display_zoom = self.global_state.settings.gameplay.zoom_sensitivity;

            // Mouse Pan Sensitivity
            Text::new("Pan Sensitivity")
                .top_left_with_margins_on(state.ids.settings_content, 10.0, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.mouse_pan_label, ui);

            if let Some(new_val) = ImageSlider::discrete(
                display_pan,
                1,
                200,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(550.0, 22.0)
            .down_from(state.ids.mouse_pan_label, 10.0)
            .track_breadth(30.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.mouse_pan_slider, ui)
            {
                events.push(Event::AdjustMousePan(new_val));
            }

            Text::new(&format!("{}", display_pan))
                .right_from(state.ids.mouse_pan_slider, 8.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.mouse_pan_value, ui);

            // Mouse Zoom Sensitivity
            Text::new("Zoom Sensitivity")
                .down_from(state.ids.mouse_pan_slider, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.mouse_zoom_label, ui);

            if let Some(new_val) = ImageSlider::discrete(
                display_zoom,
                1,
                200,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(550.0, 22.0)
            .down_from(state.ids.mouse_zoom_label, 10.0)
            .track_breadth(30.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.mouse_zoom_slider, ui)
            {
                events.push(Event::AdjustMouseZoom(new_val));
            }

            Text::new(&format!("{}", display_zoom))
                .right_from(state.ids.mouse_zoom_slider, 8.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.mouse_zoom_value, ui);
        }

        // 3) Controls Tab --------------------------------
        if Button::image(if let SettingsTab::Controls = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Controls = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Controls = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_press
        })
        .right_from(state.ids.gameplay, 0.0)
        .label("Controls")
        .label_font_size(14)
        .label_color(TEXT_COLOR)
        .set(state.ids.controls, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Controls));
        }

        // Contents
        if let SettingsTab::Controls = self.show.settings_tab {
            Text::new(
                "Free Cursor\n\
            Toggle Help Window\n\
            Toggle Interface\n\
            Toggle FPS and Debug Info\n\
            Take Screenshot\n\
            Toggle Nametags\n\
            Toggle Fullscreen\n\
            \n\
            \n\
            Move Forward\n\
            Move Left\n\
            Move Right\n\
            Move Backwards\n\
            \n\
            Jump\n\
            \n\
            Glider
            \n\
            Dodge\n\
            \n\
            Auto Walk\n\
            \n\
            Sheathe/Draw Weapons\n\
            \n\
            Put on/Remove Helmet\n\
            \n\
            Sit\n\
            \n\
            \n\
            Basic Attack\n\
            Secondary Attack/Block/Aim\n\
            \n\
            \n\
            Skillbar Slot 1\n\
            Skillbar Slot 2\n\
            Skillbar Slot 3\n\
            Skillbar Slot 4\n\
            Skillbar Slot 5\n\
            Skillbar Slot 6\n\
            Skillbar Slot 7\n\
            Skillbar Slot 8\n\
            Skillbar Slot 9\n\
            Skillbar Slot 10\n\
            \n\
            \n\
            Pause Menu\n\
            Settings\n\
            Social\n\
            Map\n\
            Spellbook\n\
            Character\n\
            Questlog\n\
            Bag\n\
            \n\
            \n\
            \n\
            Send Chat Message\n\
            Scroll Chat\n\
            \n\
            \n\
            Chat commands:  \n\
            \n\
            /alias [name] - Change your Chat Name   \n\
            /tp [name] - Teleports you to another player    \n\
            /jump <dx> <dy> <dz> - Offset your position \n\
            /goto <x> <y> <z> - Teleport to a position  \n\
            /kill - Kill yourself   \n\            
            /spawn <hostile/friendly> <npc-name> <amount> - Spawn NPC  \n\
            /time <day/night> - Sets time of day \n\
            /help - Display chat commands
            ",
            )
            .color(TEXT_COLOR)
            .top_left_with_margins_on(state.ids.settings_content, 5.0, 5.0)
            .font_id(self.fonts.opensans)
            .font_size(18)
            .set(state.ids.controls_text, ui);
            // TODO: Replace with buttons that show actual keybinds and allow the user to change them.
            Text::new(
                "TAB\n\
                 F1\n\
                 F2\n\
                 F3\n\
                 F4\n\
                 F6\n\
                 F11\n\
                 \n\
                 \n\
                 W\n\
                 A\n\
                 S\n\
                 D\n\
                 \n\
                 SPACE\n\
                 \n\
                 L-Shift\n\
                 \n\
                 ??\n\
                 \n\
                 ??\n\
                 \n\
                 ??\n\
                 \n\
                 ??\n\
                 \n\
                 ??\n\
                 \n\
                 \n\
                 L-Click\n\
                 R-Click\n\
                 \n\
                 \n\
                 1\n\
                 2\n\
                 3\n\
                 4\n\
                 5\n\
                 6\n\
                 7\n\
                 8\n\
                 9\n\
                 0\n\
                 \n\
                 \n\
                 ESC\n\
                 N\n\
                 O\n\
                 M\n\
                 P\n\
                 C\n\
                 L\n\
                 B\n\
                 \n\
                 \n\
                 \n\
                 ENTER\n\
                 Mousewheel\n\
                 \n\
                 \n\
                 \n\
                 \n\
                 \n\
                 \n\
                 ",
            )
            .color(TEXT_COLOR)
            .right_from(state.ids.controls_text, 0.0)
            .font_id(self.fonts.opensans)
            .font_size(18)
            .set(state.ids.controls_controls, ui);
        }

        // 4) Video Tab -----------------------------------
        if Button::image(if let SettingsTab::Video = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Video = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Video = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_press
        })
        .right_from(state.ids.controls, 0.0)
        .label("Video")
        .parent(state.ids.settings_r)
        .label_font_size(14)
        .label_color(TEXT_COLOR)
        .set(state.ids.video, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Video));
        }

        // Contents
        if let SettingsTab::Video = self.show.settings_tab {
            // View Distance
            Text::new("View Distance")
                .top_left_with_margins_on(state.ids.settings_content, 10.0, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.vd_text, ui);

            if let Some(new_val) = ImageSlider::discrete(
                self.global_state.settings.graphics.view_distance,
                1,
                25,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .down_from(state.ids.vd_text, 8.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.vd_slider, ui)
            {
                events.push(Event::AdjustViewDistance(new_val));
            }

            Text::new(&format!(
                "{}",
                self.global_state.settings.graphics.view_distance
            ))
            .right_from(state.ids.vd_slider, 8.0)
            .font_size(14)
            .font_id(self.fonts.opensans)
            .color(TEXT_COLOR)
            .set(state.ids.vd_value, ui);

            // Max FPS
            Text::new("Maximum FPS")
                .down_from(state.ids.vd_slider, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.max_fps_text, ui);

            if let Some(which) = ImageSlider::discrete(
                FPS_CHOICES
                    .iter()
                    .position(|&x| x == self.global_state.settings.graphics.max_fps)
                    .unwrap_or(5),
                0,
                10,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .down_from(state.ids.max_fps_text, 8.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.max_fps_slider, ui)
            {
                events.push(Event::MaximumFPS(FPS_CHOICES[which]));
            }

            Text::new(&format!("{}", self.global_state.settings.graphics.max_fps))
                .right_from(state.ids.max_fps_slider, 8.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.max_fps_value, ui);
        }

        // 5) Sound Tab -----------------------------------
        if Button::image(if let SettingsTab::Sound = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Sound = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Sound = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_press
        })
        .right_from(state.ids.video, 0.0)
        .parent(state.ids.settings_r)
        .label("Sound")
        .label_font_size(14)
        .label_color(TEXT_COLOR)
        .set(state.ids.sound, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Sound));
        }

        // Contents
        if let SettingsTab::Sound = self.show.settings_tab {
            Text::new("Volume")
                .top_left_with_margins_on(state.ids.settings_content, 10.0, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.audio_volume_text, ui);

            if let Some(new_val) = ImageSlider::continuous(
                self.global_state.settings.audio.music_volume,
                0.0,
                1.0,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .down_from(state.ids.audio_volume_text, 10.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.audio_volume_slider, ui)
            {
                events.push(Event::AdjustVolume(new_val));
            }

            // Audio Device Selector --------------------------------------------
            let device = self.global_state.audio.get_device_name();
            let device_list = self.global_state.audio.list_device_names();
            Text::new("Volume")
                .down_from(state.ids.audio_volume_slider, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.audio_device_text, ui);

            // Get which device is currently selected
            let selected = device_list.iter().position(|x| x.contains(&device));

            if let Some(clicked) = DropDownList::new(&device_list, selected)
                .w_h(400.0, 22.0)
                .down_from(state.ids.audio_device_text, 10.0)
                .label_font_id(self.fonts.opensans)
                .set(state.ids.audio_device_list, ui)
            {
                let new_val = device_list[clicked].clone();
                events.push(Event::ChangeAudioDevice(new_val));
            }
        }

        events
    }
}
