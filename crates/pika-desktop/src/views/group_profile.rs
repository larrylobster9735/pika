use std::path::PathBuf;

use base64::Engine as _;
use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Element, Fill, Task, Theme};
use pika_core::MyProfileState;

use crate::icons;
use crate::theme;
use crate::views::avatar::avatar_circle;

#[derive(Debug)]
pub struct State {
    chat_id: String,
    name: String,
    about: String,
    pending_picture_path: Option<String>,
    pending_image: Option<PendingImage>,
    upload_after_save: bool,
    uploading: bool,
}

#[derive(Debug, Clone)]
struct PendingImage {
    image_base64: String,
    mime_type: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    AboutChanged(String),
    PickProfileImage,
    ProfileImagePicked(Vec<PathBuf>),
    Save,
    Close,
}

pub enum Event {
    SaveGroupProfile {
        chat_id: String,
        name: String,
        about: String,
    },
    UploadGroupProfileImage {
        chat_id: String,
        image_base64: String,
        mime_type: String,
    },
    Close,
}

impl State {
    pub fn new(chat_id: String, profile: Option<&MyProfileState>) -> Self {
        Self {
            chat_id,
            name: profile.map(|p| p.name.clone()).unwrap_or_default(),
            about: profile.map(|p| p.about.clone()).unwrap_or_default(),
            pending_picture_path: None,
            pending_image: None,
            upload_after_save: false,
            uploading: false,
        }
    }

    pub fn update(&mut self, message: Message) -> (Option<Event>, Option<Task<Message>>) {
        match message {
            Message::NameChanged(name) => {
                self.name = name;
            }
            Message::AboutChanged(about) => {
                self.about = about;
            }
            Message::PickProfileImage => {
                let task = Task::perform(
                    async {
                        let handle = rfd::AsyncFileDialog::new()
                            .set_title("Choose group profile picture")
                            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
                            .pick_file()
                            .await;
                        match handle {
                            Some(h) => vec![h.path().to_path_buf()],
                            None => vec![],
                        }
                    },
                    Message::ProfileImagePicked,
                );
                return (None, Some(task));
            }
            Message::ProfileImagePicked(paths) => {
                if let Some(img) = prepare_profile_image(&paths) {
                    if let Some(path) = paths.first() {
                        self.pending_picture_path =
                            Some(format!("file://{}", path.to_string_lossy()));
                    }
                    self.pending_image = Some(img);
                }
            }
            Message::Save => {
                if self.pending_image.is_some() {
                    self.upload_after_save = true;
                    self.uploading = true;
                }
                return (
                    Some(Event::SaveGroupProfile {
                        chat_id: self.chat_id.clone(),
                        name: self.name.clone(),
                        about: self.about.clone(),
                    }),
                    None,
                );
            }
            Message::Close => return (Some(Event::Close), None),
        }
        (None, None)
    }

    pub fn take_deferred_upload(&mut self) -> Option<Event> {
        if self.upload_after_save {
            self.upload_after_save = false;
            if let Some(img) = self.pending_image.take() {
                return Some(Event::UploadGroupProfileImage {
                    chat_id: self.chat_id.clone(),
                    image_base64: img.image_base64,
                    mime_type: img.mime_type,
                });
            }
        }
        None
    }

    pub fn sync_profile(&mut self, profile: &MyProfileState) {
        self.name = profile.name.clone();
        self.about = profile.about.clone();
        if profile.picture_url.is_some() && self.pending_image.is_none() {
            self.pending_picture_path = None;
            self.uploading = false;
        }
    }

    pub fn view<'a>(
        &'a self,
        picture_url: Option<&'a str>,
        avatar_cache: &mut super::avatar::AvatarCache,
    ) -> Element<'a, Message, Theme> {
        let mut content = column![].spacing(4).width(Fill);

        // ── Back button ───────────────────────────────────────────────
        content = content.push(
            container(
                button(
                    row![
                        text(icons::CHEVRON_LEFT)
                            .font(icons::LUCIDE_FONT)
                            .size(18)
                            .color(theme::text_secondary()),
                        text("Back").size(14).color(theme::text_secondary()),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::Close)
                .padding([8, 12])
                .style(theme::icon_button_style(false)),
            )
            .padding([12, 16]),
        );

        // ── Header ───────────────────────────────────────────────────
        content = content.push(
            container(
                text("Group Profile")
                    .size(16)
                    .font(icons::BOLD)
                    .color(theme::text_primary()),
            )
            .width(Fill)
            .center_x(Fill)
            .padding([8, 0]),
        );

        // ── Avatar (clickable to change) ─────────────────────────────
        let display_name = if self.name.is_empty() {
            "Me"
        } else {
            self.name.as_str()
        };
        let effective_picture = self.pending_picture_path.as_deref().or(picture_url);

        let avatar_label = if self.uploading {
            text("Uploading\u{2026}")
                .size(12)
                .color(theme::text_secondary())
        } else {
            text("Change photo").size(12).color(theme::accent_blue())
        };

        let avatar_button = button(
            column![
                avatar_circle(Some(display_name), effective_picture, 80.0, avatar_cache,),
                avatar_label,
            ]
            .spacing(4)
            .align_x(Alignment::Center),
        )
        .padding(4)
        .style(|_: &Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered => theme::hover_bg(),
                _ => iced::Color::TRANSPARENT,
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::border::rounded(12),
                ..Default::default()
            }
        });

        let avatar_button = if self.uploading {
            avatar_button
        } else {
            avatar_button.on_press(Message::PickProfileImage)
        };

        content = content.push(
            container(avatar_button)
                .width(Fill)
                .center_x(Fill)
                .padding([8, 0]),
        );

        // ── Name field ────────────────────────────────────────────────
        content = content.push(icon_input_row(
            icons::USER,
            "Display name\u{2026}",
            self.name.as_str(),
            Message::NameChanged,
        ));

        // ── About field ───────────────────────────────────────────────
        content = content.push(icon_input_row(
            icons::PEN,
            "About\u{2026}",
            self.about.as_str(),
            Message::AboutChanged,
        ));

        // ── Save button ──────────────────────────────────────────────
        let save_button =
            button(text("Save Changes").size(14).font(icons::MEDIUM).center()).padding([10, 24]);

        let save_button = if self.uploading {
            save_button.style(theme::secondary_button_style)
        } else {
            save_button
                .on_press(Message::Save)
                .style(theme::primary_button_style)
        };

        content = content.push(
            container(save_button)
                .width(Fill)
                .center_x(Fill)
                .padding([8, 24]),
        );

        content = content.push(Space::new().height(Fill));

        container(content)
            .width(Fill)
            .height(Fill)
            .style(theme::surface_style)
            .into()
    }
}

fn prepare_profile_image(paths: &[PathBuf]) -> Option<PendingImage> {
    let path = paths.first()?;
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[group-pfp] failed to read {}: {e}", path.display());
            return None;
        }
    };
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let mime_type = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/jpeg",
    }
    .to_string();
    let image_base64 = base64::engine::general_purpose::STANDARD.encode(&data);
    Some(PendingImage {
        image_base64,
        mime_type,
    })
}

fn icon_input_row<'a>(
    icon_cp: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: impl 'a + Fn(String) -> Message,
) -> Element<'a, Message, Theme> {
    container(
        row![
            text(icon_cp)
                .font(icons::LUCIDE_FONT)
                .size(18)
                .color(theme::text_secondary()),
            text_input(placeholder, value)
                .on_input(on_input)
                .padding(10)
                .width(Fill)
                .style(theme::dark_input_style),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([4, 24])
    .into()
}
