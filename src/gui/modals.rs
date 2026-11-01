use super::*;

impl ZaloGuiApp {
    pub(super) fn render_profile_popover(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        if !self.profile_menu_open {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.profile_menu_open = false;
            return;
        }

        let mut close_menu = false;
        let mut do_logout = false;
        let area_resp = egui::Area::new(Id::new("left_rail_profile_popover"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(68.0, 44.0))
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(pal.bg_elevated)
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.set_width(220.0);
                        let display_user = if self.user_name.is_empty() {
                            "Người dùng Zalo"
                        } else {
                            &self.user_name
                        };
                        ui.label(
                            RichText::new(display_user)
                                .size(14.0)
                                .color(pal.text_primary)
                                .strong(),
                        );
                        ui.horizontal(|ui| {
                            let (dr, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                            ui.painter()
                                .circle_filled(dr.center(), 3.8, pal.accent_online);
                            ui.label(
                                RichText::new("Đang hoạt động")
                                    .size(11.5)
                                    .color(pal.text_secondary),
                            );
                        });
                        if !self.user_uin.is_empty() {
                            ui.label(
                                RichText::new(format!("UIN: {}", self.user_uin))
                                    .size(10.5)
                                    .color(pal.text_muted),
                            );
                        }

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);

                        if ui
                            .add_sized(
                                [200.0, 28.0],
                                egui::Button::new(
                                    RichText::new(format!("{}  Đã đồng bộ đám mây", lucide::CHECK))
                                        .size(12.2)
                                        .color(pal.text_primary),
                                )
                                .fill(Color32::TRANSPARENT),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::RefreshThreads);
                            }
                            self.show_toast("Đang đồng bộ 20 hội thoại ngầm…");
                            close_menu = true;
                        }

                        let theme_lbl = if self.dark_mode {
                            format!("{}  Chuyển giao diện Sáng", lucide::SUN)
                        } else {
                            format!("{}  Chuyển giao diện Tối", lucide::MOON)
                        };
                        if ui
                            .add_sized(
                                [200.0, 28.0],
                                egui::Button::new(
                                    RichText::new(theme_lbl).size(12.2).color(pal.text_primary),
                                )
                                .fill(Color32::TRANSPARENT),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.dark_mode = !self.dark_mode;
                            self.theme_dirty = true;
                            close_menu = true;
                        }

                        if self.dev_mode {
                            let dev_lbl = if self.dev_panel_open {
                                "Đóng Dev Inspector"
                            } else {
                                "Mở Dev Inspector"
                            };
                            if ui
                                .add_sized(
                                    [200.0, 28.0],
                                    egui::Button::new(
                                        RichText::new(dev_lbl).size(12.2).color(pal.warning),
                                    )
                                    .fill(Color32::TRANSPARENT),
                                )
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.dev_panel_open = !self.dev_panel_open;
                                if self.dev_panel_open {
                                    self.scanner_findings = logger::scan_logs();
                                }
                                close_menu = true;
                            }
                        }

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        if ui
                            .add_sized(
                                [200.0, 30.0],
                                egui::Button::new(
                                    RichText::new("Đăng xuất tài khoản")
                                        .size(12.5)
                                        .color(pal.danger)
                                        .strong(),
                                )
                                .fill(pal.danger.linear_multiply(0.12))
                                .rounding(Rounding::same(7.0)),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            do_logout = true;
                            close_menu = true;
                        }
                        ui.add_space(4.0);
                        if ui
                            .add_sized(
                                [200.0, 28.0],
                                egui::Button::new(
                                    RichText::new(format!(
                                        "{}  Thoát hoàn toàn (Ctrl+Q)",
                                        lucide::X
                                    ))
                                    .size(12.0)
                                    .color(pal.text_secondary),
                                )
                                .fill(pal.bg_input)
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .rounding(Rounding::same(7.0)),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
            });

        if ctx.input(|i| i.pointer.any_click())
            && !area_resp.response.hovered()
            && ctx
                .input(|i| i.pointer.interact_pos())
                .map(|p| p.x > 64.0 || p.y > 60.0)
                .unwrap_or(false)
        {
            close_menu = true;
        }

        if do_logout {
            self.is_logged_in = false;
            self.user_name.clear();
            self.user_uin.clear();
            self.threads.clear();
            self.selected_thread_index = 0;
            self.qr_texture = None;
            self.login_status_msg.clear();
            self.qr_status = "Đang xin mã QR mới…".into();
            self.status_msg = "Đã đăng xuất".into();
            let _ = std::fs::remove_file("session.json");
            let _ = std::fs::remove_file("threads_cache.json");
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::Logout);
                let _ = tx.send(WorkerCmd::RequestQr);
            }
        }
        if close_menu {
            self.profile_menu_open = false;
        }
    }

    pub(super) fn render_toast_overlay(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        let Some((ref text, ts)) = self.toast_notice else {
            return;
        };
        if ts.elapsed() > Duration::from_millis(2500) {
            self.toast_notice = None;
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(200));
        let screen = ctx.screen_rect();
        egui::Area::new(Id::new("floating_toast_notice"))
            .order(egui::Order::Tooltip)
            .fixed_pos(Pos2::new((screen.right() - 320.0).max(80.0), screen.bottom() - 54.0))
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(pal.bg_elevated)
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::symmetric(14.0, 9.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (dr, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                            ui.painter()
                                .circle_filled(dr.center(), 4.0, pal.accent_online);
                            ui.label(
                                RichText::new(text)
                                    .size(12.0)
                                    .color(pal.text_primary)
                                    .strong(),
                            );
                        });
                    });
            });
    }

    pub(super) fn render_phone_lookup_modal(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        if !self.phone_modal_open {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.phone_modal_open = false;
            return;
        }

        let mut open = true;
        let mut trigger_lookup: Option<String> = None;
        let mut open_chat_user: Option<crate::chat::PhoneLookupResult> = None;
        let mut add_friend_user: Option<(String, String)> = None;

        egui::Window::new("Tìm kiếm Số điện thoại & Kết bạn Zalo")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width((ctx.screen_rect().width() * 0.78).clamp(280.0, 390.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(pal.bg_elevated)
                    .rounding(Rounding::same(14.0))
                    .stroke(Stroke::new(1.2_f32, pal.primary))
                    .inner_margin(Margin::symmetric(20.0, 16.0)),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("Nhập số điện thoại để tìm tài khoản Zalo (VD: 0912345678 hoặc +84...):")
                        .size(12.2)
                        .color(pal.text_secondary),
                );
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    let te = egui::TextEdit::singleline(&mut self.phone_modal_input)
                        .hint_text("Số điện thoại Zalo...")
                        .margin(Margin::symmetric(10.0, 7.0))
                        .desired_width(250.0);
                    let resp = ui.add(te);
                    let enter = (resp.lost_focus() || resp.has_focus())
                        && ui.input(|i| i.key_pressed(Key::Enter));

                    let search_btn = egui::Button::new(
                        RichText::new("Tìm kiếm")
                            .size(12.8)
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(pal.primary)
                    .rounding(Rounding::same(8.0))
                    .min_size(Vec2::new(88.0, 32.0));

                    if (ui
                        .add(search_btn)
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .clicked()
                        || enter)
                        && !self.phone_modal_input.trim().is_empty()
                    {
                        trigger_lookup = Some(self.phone_modal_input.trim().to_string());
                    }
                });

                ui.add_space(12.0);

                if self.phone_lookup_loading {
                    ui.vertical_centered(|ui| {
                        ui.add_space(16.0);
                        ui.spinner();
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Đang tra cứu hồ sơ Zalo theo số điện thoại…")
                                .size(12.5)
                                .color(pal.text_secondary),
                        );
                        ui.add_space(12.0);
                    });
                } else if let Some(ref err) = self.phone_lookup_error {
                    egui::Frame::none()
                        .fill(pal.danger.linear_multiply(0.14))
                        .stroke(Stroke::new(1.0_f32, pal.danger))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::symmetric(12.0, 10.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("Không tìm thấy: {err}"))
                                    .size(12.5)
                                    .color(pal.text_primary)
                                    .strong(),
                            );
                        });
                } else if let Some(ref prof) = self.phone_lookup_result {
                    egui::Frame::none()
                        .fill(pal.bg_input)
                        .stroke(Stroke::new(1.0_f32, pal.border))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(Margin::symmetric(14.0, 12.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (av_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(54.0), Sense::hover());
                                paint_user_avatar(
                                    ui.painter(),
                                    av_rect.center(),
                                    27.0,
                                    &prof.display_name,
                                    false,
                                    Some(prof.avatar.as_str()),
                                    &self.sticker_textures,
                                    &mut self.loading_stickers,
                                );

                                ui.add_space(6.0);
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(&prof.display_name)
                                                .size(15.0)
                                                .color(pal.text_primary)
                                                .strong(),
                                        );
                                        let (f_lbl, f_col) = match prof.friendship {
                                            crate::chat::FriendshipStatus::Friend => {
                                                ("✓ Bạn bè", pal.accent_online)
                                            }
                                            crate::chat::FriendshipStatus::IncomingPending => {
                                                ("Chờ bạn đồng ý", pal.primary_hover)
                                            }
                                            crate::chat::FriendshipStatus::OutgoingPending => {
                                                ("Đã gửi lời mời", pal.warning)
                                            }
                                            crate::chat::FriendshipStatus::Stranger => (
                                                "Chưa kết bạn",
                                                Color32::from_rgb(249, 115, 22),
                                            ),
                                            crate::chat::FriendshipStatus::Unknown => {
                                                ("Đang kiểm tra…", pal.text_muted)
                                            }
                                        };
                                        egui::Frame::none()
                                            .fill(f_col.linear_multiply(0.15))
                                            .rounding(Rounding::same(5.0))
                                            .inner_margin(Margin::symmetric(6.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(f_lbl)
                                                        .size(10.5)
                                                        .color(f_col)
                                                        .strong(),
                                                );
                                            });
                                    });
                                    ui.label(
                                        RichText::new(format!(
                                            "SĐT: {} · UID: {}",
                                            prof.phone, prof.user_id
                                        ))
                                        .size(11.5)
                                        .color(pal.text_secondary),
                                    );
                                    if !prof.status.is_empty() {
                                        ui.label(
                                            RichText::new(&prof.status)
                                                .size(11.5)
                                                .color(pal.text_muted),
                                        );
                                    }
                                });
                            });

                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                let chat_btn = egui::Button::new(
                                    RichText::new("Nhắn tin ngay")
                                        .size(12.5)
                                        .color(Color32::WHITE)
                                        .strong(),
                                )
                                .fill(pal.primary)
                                .rounding(Rounding::same(8.0))
                                .min_size(Vec2::new(155.0, 32.0));
                                if ui
                                    .add(chat_btn)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    open_chat_user = Some(prof.clone());
                                }

                                match prof.friendship {
                                    crate::chat::FriendshipStatus::Friend => {
                                        let badge_btn = egui::Button::new(
                                            RichText::new(format!(
                                                "{} Đã là bạn bè",
                                                lucide::CHECK
                                            ))
                                            .size(12.2)
                                            .color(pal.accent_online)
                                            .strong(),
                                        )
                                        .fill(pal.accent_online.linear_multiply(0.14))
                                        .stroke(Stroke::new(1.0_f32, pal.accent_online))
                                        .rounding(Rounding::same(8.0))
                                        .min_size(Vec2::new(155.0, 32.0));
                                        ui.add_enabled(false, badge_btn);
                                    }
                                    crate::chat::FriendshipStatus::OutgoingPending => {
                                        let pending_btn = egui::Button::new(
                                            RichText::new("Đã gửi lời mời")
                                                .size(12.2)
                                                .color(pal.warning)
                                                .strong(),
                                        )
                                        .fill(pal.warning.linear_multiply(0.14))
                                        .stroke(Stroke::new(1.0_f32, pal.warning))
                                        .rounding(Rounding::same(8.0))
                                        .min_size(Vec2::new(155.0, 32.0));
                                        ui.add_enabled(false, pending_btn);
                                    }
                                    crate::chat::FriendshipStatus::IncomingPending => {
                                        let accept_btn = egui::Button::new(
                                            RichText::new(format!(
                                                "{} Đồng ý kết bạn",
                                                lucide::CHECK
                                            ))
                                            .size(12.5)
                                            .color(Color32::WHITE)
                                            .strong(),
                                        )
                                        .fill(pal.accent_online)
                                        .rounding(Rounding::same(8.0))
                                        .min_size(Vec2::new(155.0, 32.0));
                                        if ui
                                            .add(accept_btn)
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            add_friend_user = Some((
                                                prof.user_id.clone(),
                                                prof.display_name.clone(),
                                            ));
                                        }
                                    }
                                    _ => {
                                        let friend_btn = egui::Button::new(
                                            RichText::new(format!(
                                                "{} Kết bạn Zalo",
                                                lucide::USER_PLUS
                                            ))
                                            .size(12.5)
                                            .color(pal.accent_online)
                                            .strong(),
                                        )
                                        .fill(pal.accent_online.linear_multiply(0.15))
                                        .stroke(Stroke::new(1.0_f32, pal.accent_online))
                                        .rounding(Rounding::same(8.0))
                                        .min_size(Vec2::new(155.0, 32.0));
                                        if ui
                                            .add(friend_btn)
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            add_friend_user = Some((
                                                prof.user_id.clone(),
                                                prof.display_name.clone(),
                                            ));
                                        }
                                    }
                                }
                            });
                        });
                }
            });

        if let Some(phone_q) = trigger_lookup {
            self.phone_lookup_loading = true;
            self.phone_lookup_error = None;
            self.phone_lookup_result = None;
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::LookupPhone { phone: phone_q });
            }
        }

        if let Some(prof) = open_chat_user {
            if let Some(idx) = self.threads.iter().position(|t| t.id == prof.user_id) {
                self.selected_thread_index = idx;
            } else {
                self.threads.insert(
                    0,
                    Thread {
                        id: prof.user_id.clone(),
                        name: prof.display_name.clone(),
                        preview: format!("SĐT: {}", prof.phone),
                        time: crate::ui::clock(),
                        avatar: if prof.avatar.is_empty() {
                            None
                        } else {
                            Some(prof.avatar)
                        },
                        is_group: false,
                        messages: Vec::new(),
                    },
                );
                self.selected_thread_index = 0;
                self.persist_threads_async();
            }
            self.search_query.clear();
            self.phone_modal_open = false;
            self.show_toast(format!("Đã mở cuộc trò chuyện với {}", prof.display_name));
        }

        if let Some((uid, uname)) = add_friend_user {
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::AddFriend {
                    user_id: uid,
                    user_name: uname.clone(),
                });
            }
            self.show_toast(format!("Đang gửi lời mời kết bạn tới {uname}…"));
        }

        if !open {
            self.phone_modal_open = false;
        }
    }
    pub(super) fn render_settings_modal(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        if !self.settings_modal_open {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.settings_modal_open = false;
            self.settings.save();
            return;
        }

        let mut open = true;
        let screen = ctx.screen_rect();
        let win_w = (screen.width() * 0.88).clamp(620.0, 780.0);
        let win_h = (screen.height() * 0.82).clamp(480.0, 600.0);

        let mut theme_switched = false;
        let mut trigger_sync = false;
        let mut pick_dl_dir = false;
        let mut open_dl_dir = false;

        egui::Window::new("settings_window_modal")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(win_w, win_h))
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_chat_canvas)
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(Margin::ZERO),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    // Left Tabs Navigation Sidebar (200px)
                    egui::Frame::none()
                        .fill(pal.bg_sidebar)
                        .inner_margin(Margin::symmetric(10.0, 16.0))
                        .show(ui, |ui| {
                            ui.set_width(200.0);
                            ui.set_height(win_h - 32.0);
                            ui.vertical(|ui| {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Cài đặt")
                                        .size(17.5)
                                        .color(pal.text_primary)
                                        .strong(),
                                );
                                ui.add_space(14.0);

                                let tabs = [
                                    (SettingsTab::General, lucide::SETTINGS, "Cài đặt chung"),
                                    (SettingsTab::Privacy, lucide::LOCK, "Quyền riêng tư"),
                                    (SettingsTab::Sync, lucide::REFRESH, "Đồng bộ tin nhắn"),
                                    (SettingsTab::Appearance, lucide::PAINTBRUSH, "Giao diện"),
                                    (SettingsTab::Notifications, lucide::BELL, "Thông báo"),
                                    (SettingsTab::Messages, lucide::MESSAGE_SQUARE, "Tin nhắn"),
                                    (SettingsTab::Utilities, lucide::SLIDERS, "Tiện ích"),
                                ];

                                for (tab, icon, title) in tabs {
                                    let is_active = self.settings_tab == tab;
                                    let text_col = if is_active {
                                        pal.primary_hover
                                    } else {
                                        pal.text_secondary
                                    };

                                    let (rect, resp) = ui.allocate_exact_size(
                                        Vec2::new(196.0, 36.0),
                                        Sense::click(),
                                    );
                                    if is_active || resp.hovered() {
                                        ui.painter().rect_filled(
                                            rect,
                                            Rounding::same(8.0),
                                            if is_active {
                                                pal.bg_selected
                                            } else {
                                                pal.bg_hover
                                            },
                                        );
                                    }
                                    let icon_pt = Pos2::new(rect.left() + 18.0, rect.center().y);
                                    draw_lucide_icon(
                                        ui.painter(),
                                        icon_pt,
                                        icon,
                                        14.5,
                                        text_col,
                                    );
                                    let text_pt = Pos2::new(rect.left() + 38.0, rect.center().y);
                                    ui.painter().text(
                                        text_pt,
                                        egui::Align2::LEFT_CENTER,
                                        title,
                                        FontId::proportional(13.2),
                                        if is_active {
                                            pal.text_primary
                                        } else {
                                            text_col
                                        },
                                    );

                                    if resp
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        self.settings_tab = tab;
                                    }
                                    ui.add_space(2.0);
                                }
                            });
                        });

                    // Right Content Panel
                    let content_w = win_w - 220.0;
                    egui::Frame::none()
                        .fill(pal.bg_chat_canvas)
                        .inner_margin(Margin::symmetric(24.0, 16.0))
                        .show(ui, |ui| {
                            ui.set_width(content_w);
                            ui.set_height(win_h - 32.0);

                            ui.horizontal(|ui| {
                                let tab_title = match self.settings_tab {
                                    SettingsTab::General => "Cài đặt chung",
                                    SettingsTab::Privacy => "Quyền riêng tư",
                                    SettingsTab::Sync => "Đồng bộ tin nhắn",
                                    SettingsTab::Appearance => "Cài đặt giao diện",
                                    SettingsTab::Notifications => "Cài đặt thông báo",
                                    SettingsTab::Messages => "Cài đặt tin nhắn",
                                    SettingsTab::Utilities => "Tiện ích & Gợi ý",
                                };
                                ui.label(
                                    RichText::new(tab_title)
                                        .size(15.5)
                                        .color(pal.text_primary)
                                        .strong(),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let (cr, c_resp) = ui.allocate_exact_size(
                                        Vec2::splat(26.0),
                                        Sense::click(),
                                    );
                                    if c_resp.hovered() {
                                        ui.painter().rect_filled(
                                            cr,
                                            Rounding::same(6.0),
                                            pal.bg_hover,
                                        );
                                    }
                                    draw_lucide_icon(
                                        ui.painter(),
                                        cr.center(),
                                        lucide::X,
                                        14.0,
                                        if c_resp.hovered() {
                                            pal.text_primary
                                        } else {
                                            pal.text_secondary
                                        },
                                    );
                                    if c_resp
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        open = false;
                                    }
                                });
                            });

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(8.0);

                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    match self.settings_tab {
                                        SettingsTab::General => {
                                            ui.label(RichText::new("Danh bạ").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(2.0);
                                            ui.label(RichText::new("Hiển thị danh bạ trong Zalo, bạn có thể chọn hiển thị tất cả danh bạ hoặc chỉ những bạn bè đang sử dụng Zalo").color(pal.text_secondary).size(12.0));
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.set_min_width(ui.available_width());
                                                    ui.radio_value(&mut self.settings.show_all_contacts, true, "Hiển thị tất cả danh bạ");
                                                    ui.add_space(4.0);
                                                    ui.radio_value(&mut self.settings.show_all_contacts, false, "Chỉ hiển thị bạn bè đang sử dụng Zalo");
                                                });
                                            ui.add_space(14.0);

                                            ui.label(RichText::new("Ngôn ngữ").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Thay đổi ngôn ngữ").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            egui::ComboBox::from_id_salt("setting_lang_combo")
                                                                .selected_text(&self.settings.language)
                                                                .width(130.0)
                                                                .show_ui(ui, |ui| {
                                                                    ui.selectable_value(&mut self.settings.language, "Tiếng Việt".into(), "Tiếng Việt");
                                                                    ui.selectable_value(&mut self.settings.language, "English".into(), "English");
                                                                });
                                                        });
                                                    });
                                                });
                                            ui.add_space(14.0);

                                            ui.label(RichText::new("Khởi động & Lưu File").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Khởi động Zalo khi mở máy").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.autostart_on_boot, pal);
                                                        });
                                                    });
                                                    ui.add_space(8.0);
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Ghi nhớ tài khoản đăng nhập lần trước").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.remember_login, pal);
                                                        });
                                                    });
                                                    ui.add_space(10.0);
                                                    ui.separator();
                                                    ui.add_space(6.0);
                                                    ui.label(RichText::new("File được lưu tại:").color(pal.text_secondary).size(12.2));
                                                    ui.add_space(4.0);
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            RichText::new(truncate_chars(&self.settings.download_dir, 36))
                                                                .size(12.0)
                                                                .color(pal.text_muted),
                                                        );
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            if ui.button("Thay đổi").clicked() {
                                                                pick_dl_dir = true;
                                                            }
                                                            if ui.button("Mở thư mục").clicked() {
                                                                open_dl_dir = true;
                                                            }
                                                        });
                                                    });
                                                });
                                            ui.add_space(14.0);

                                            ui.label(RichText::new("Thiết bị Âm thanh & Video (Cuộc gọi)").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    let (mics, cams) = crate::media::list_media_devices(false);
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new(format!("{} Đầu vào Mic:", lucide::MIC)).color(pal.text_primary).size(13.0));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            let cur_mic_lbl = if self.settings.selected_mic_device.is_empty() {
                                                                "Mặc định hệ thống"
                                                            } else {
                                                                &self.settings.selected_mic_device
                                                            };
                                                            egui::ComboBox::from_id_salt("setting_mic_device_combo")
                                                                .selected_text(truncate_chars(cur_mic_lbl, 30))
                                                                .width(230.0)
                                                                .show_ui(ui, |ui| {
                                                                    if ui.selectable_label(self.settings.selected_mic_device.is_empty(), "Mặc định hệ thống").clicked() {
                                                                        self.settings.selected_mic_device.clear();
                                                                        crate::media::set_selected_mic_device("");
                                                                        self.settings.save();
                                                                    }
                                                                    for m in &mics {
                                                                        if ui.selectable_label(self.settings.selected_mic_device == *m, m).clicked() {
                                                                            self.settings.selected_mic_device = m.clone();
                                                                            crate::media::set_selected_mic_device(m);
                                                                            self.settings.save();
                                                                        }
                                                                    }
                                                                });
                                                        });
                                                    });
                                                    ui.add_space(8.0);
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new(format!("{} Máy ảnh (Camera):", lucide::VIDEO)).color(pal.text_primary).size(13.0));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            let cur_cam_lbl = if self.settings.selected_camera_device.is_empty() {
                                                                cams.first().map(String::as_str).unwrap_or("Mặc định hệ thống")
                                                            } else {
                                                                &self.settings.selected_camera_device
                                                            };
                                                            egui::ComboBox::from_id_salt("setting_cam_device_combo")
                                                                .selected_text(truncate_chars(cur_cam_lbl, 30))
                                                                .width(230.0)
                                                                .show_ui(ui, |ui| {
                                                                    if ui.selectable_label(self.settings.selected_camera_device.is_empty(), "Tự động (Camera đầu tiên)").clicked() {
                                                                        self.settings.selected_camera_device.clear();
                                                                        crate::media::set_selected_camera_device("");
                                                                        self.settings.save();
                                                                    }
                                                                    for c in &cams {
                                                                        if ui.selectable_label(self.settings.selected_camera_device == *c, c).clicked() {
                                                                            self.settings.selected_camera_device = c.clone();
                                                                            crate::media::set_selected_camera_device(c);
                                                                            self.settings.save();
                                                                        }
                                                                    }
                                                                });
                                                        });
                                                    });
                                                    ui.add_space(6.0);
                                                    ui.horizontal(|ui| {
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            if ui.small_button(format!("{} Quét lại thiết bị", lucide::REFRESH)).clicked() {
                                                                let _ = crate::media::list_media_devices(true);
                                                            }
                                                        });
                                                    });
                                                });
                                        }
                                        SettingsTab::Privacy => {
                                            ui.label(RichText::new("Cá nhân").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Hiện ngày sinh").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            egui::ComboBox::from_id_salt("setting_bday_combo")
                                                                .selected_text(&self.settings.show_birthday)
                                                                .width(170.0)
                                                                .show_ui(ui, |ui| {
                                                                    ui.selectable_value(&mut self.settings.show_birthday, "Không hiện".into(), "Không hiện");
                                                                    ui.selectable_value(&mut self.settings.show_birthday, "Hiện ngày, tháng, năm".into(), "Hiện ngày, tháng, năm");
                                                                    ui.selectable_value(&mut self.settings.show_birthday, "Hiện ngày, tháng".into(), "Hiện ngày, tháng");
                                                                });
                                                        });
                                                    });
                                                    ui.add_space(8.0);
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Hiển thị trạng thái truy cập").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.show_online_status, pal);
                                                        });
                                                    });
                                                });
                                            ui.add_space(14.0);

                                            ui.label(RichText::new("Tin nhắn và cuộc gọi").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Hiện trạng thái \"Đã xem\"").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.show_seen_status, pal);
                                                        });
                                                    });
                                                    ui.add_space(8.0);
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Cho phép nhắn tin").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            egui::ComboBox::from_id_salt("setting_msg_stranger_combo")
                                                                .selected_text(&self.settings.allow_message_from)
                                                                .width(170.0)
                                                                .show_ui(ui, |ui| {
                                                                    ui.selectable_value(&mut self.settings.allow_message_from, "Tất cả mọi người".into(), "Tất cả mọi người");
                                                                    ui.selectable_value(&mut self.settings.allow_message_from, "Chỉ bạn bè".into(), "Chỉ bạn bè");
                                                                });
                                                        });
                                                    });
                                                    ui.add_space(8.0);
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Cho phép gọi điện").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            egui::ComboBox::from_id_salt("setting_call_stranger_combo")
                                                                .selected_text(&self.settings.allow_call_from)
                                                                .width(220.0)
                                                                .show_ui(ui, |ui| {
                                                                    ui.selectable_value(&mut self.settings.allow_call_from, "Bạn bè và người lạ từng liên hệ".into(), "Bạn bè và người lạ từng liên hệ");
                                                                    ui.selectable_value(&mut self.settings.allow_call_from, "Tất cả mọi người".into(), "Tất cả mọi người");
                                                                    ui.selectable_value(&mut self.settings.allow_call_from, "Chỉ bạn bè".into(), "Chỉ bạn bè");
                                                                });
                                                        });
                                                    });
                                                });
                                            ui.add_space(14.0);

                                            ui.label(RichText::new("Nguồn tìm kiếm").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Cho phép người lạ tìm thấy và kết bạn qua số điện thoại").color(pal.text_primary).size(13.0));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.allow_search_by_phone, pal);
                                                        });
                                                    });
                                                    ui.add_space(8.0);
                                                    ui.separator();
                                                    ui.add_space(6.0);
                                                    ui.label(RichText::new("Cho phép người lạ kết bạn").color(pal.text_secondary).size(12.2));
                                                    ui.add_space(4.0);
                                                    ui.checkbox(&mut self.settings.friend_src_qr, "Mã QR của tôi");
                                                    ui.checkbox(&mut self.settings.friend_src_group, "Nhóm chung");
                                                    ui.checkbox(&mut self.settings.friend_src_card, "Danh thiếp Zalo");
                                                });
                                        }
                                        SettingsTab::Sync => {
                                            ui.label(RichText::new("Đồng bộ tin nhắn").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(10.0))
                                                .inner_margin(Margin::same(14.0))
                                                .show(ui, |ui| {
                                                    ui.vertical(|ui| {
                                                        ui.label(RichText::new("Zalo lưu trữ và đồng bộ dữ liệu lịch sử tin nhắn với đám mây Zalo và SQLite cục bộ (zalo_history.db).").color(pal.text_primary).size(13.0));
                                                        ui.add_space(4.0);
                                                        let total_msgs: usize = self.threads.iter().map(|t| t.messages.len()).sum();
                                                        ui.label(RichText::new(format!("Hiện có {} hội thoại ({total_msgs} tin nhắn) trong bộ nhớ.", self.threads.len())).color(pal.text_secondary).size(12.2));
                                                        ui.add_space(10.0);
                                                        if ui.button(RichText::new(format!("{} Đồng bộ ngay", lucide::REFRESH)).size(13.0).strong()).clicked() {
                                                            trigger_sync = true;
                                                        }
                                                    });
                                                });
                                        }
                                        SettingsTab::Appearance => {
                                            ui.label(RichText::new("Cài đặt giao diện").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(8.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(10.0))
                                                .inner_margin(Margin::symmetric(18.0, 16.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 18.0;
                                                        for (mode_key, label, is_dark_val) in [
                                                            ("light", "Sáng", false),
                                                            ("dark", "Tối", true),
                                                            ("system", "Hệ Thống", true),
                                                        ] {
                                                            let selected = self.settings.theme_mode == mode_key;
                                                            ui.vertical_centered(|ui| {
                                                                let (card_r, c_resp) = ui.allocate_exact_size(Vec2::new(132.0, 84.0), Sense::click());
                                                                let stroke_col = if selected {
                                                                    pal.primary_hover
                                                                } else if c_resp.hovered() {
                                                                    pal.border
                                                                } else {
                                                                    pal.border.linear_multiply(0.6)
                                                                };
                                                                if mode_key == "system" {
                                                                    let left_half = Rect::from_min_max(card_r.min, Pos2::new(card_r.center().x, card_r.max.y));
                                                                    let right_half = Rect::from_min_max(Pos2::new(card_r.center().x, card_r.min.y), card_r.max);
                                                                    ui.painter().rect_filled(left_half, Rounding { nw: 8.0, sw: 8.0, ne: 0.0, se: 0.0 }, Color32::from_rgb(235, 242, 252));
                                                                    ui.painter().rect_filled(right_half, Rounding { nw: 0.0, sw: 0.0, ne: 8.0, se: 8.0 }, Color32::from_rgb(24, 26, 31));
                                                                } else {
                                                                    ui.painter().rect_filled(
                                                                        card_r,
                                                                        Rounding::same(8.0),
                                                                        if is_dark_val {
                                                                            Color32::from_rgb(24, 26, 31)
                                                                        } else {
                                                                            Color32::from_rgb(235, 242, 252)
                                                                        },
                                                                    );
                                                                }
                                                                ui.painter().rect_stroke(card_r, Rounding::same(8.0), Stroke::new(if selected { 2.0_f32 } else { 1.0_f32 }, stroke_col));
                                                                ui.painter().circle_filled(Pos2::new(card_r.left() + 18.0, card_r.top() + 22.0), 5.5, Color32::from_rgb(82, 139, 255));
                                                                let mock_in = Rect::from_min_size(Pos2::new(card_r.left() + 30.0, card_r.top() + 15.0), Vec2::new(62.0, 22.0));
                                                                ui.painter().rect_filled(mock_in, Rounding::same(4.0), if is_dark_val && mode_key != "light" { Color32::from_rgb(76, 82, 94) } else { Color32::WHITE });
                                                                let mock_out = Rect::from_min_size(Pos2::new(card_r.right() - 56.0, card_r.bottom() - 28.0), Vec2::new(44.0, 18.0));
                                                                ui.painter().rect_filled(mock_out, Rounding::same(4.0), Color32::from_rgb(82, 139, 255));

                                                                ui.add_space(6.0);
                                                                let r_clicked = ui.radio(selected, label).clicked();
                                                                if c_resp.on_hover_cursor(CursorIcon::PointingHand).clicked() || r_clicked {
                                                                    self.settings.theme_mode = mode_key.into();
                                                                    self.dark_mode = is_dark_val;
                                                                    theme_switched = true;
                                                                }
                                                            });
                                                        }
                                                    });
                                                });
                                            ui.add_space(16.0);

                                            ui.label(RichText::new("Hình nền chat").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Sử dụng Avatar làm hình nền").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.use_avatar_as_wallpaper, pal);
                                                        });
                                                    });
                                                });
                                        }
                                        SettingsTab::Notifications => {
                                            ui.label(RichText::new("Cài đặt thông báo").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(2.0);
                                            ui.label(RichText::new("Nhận được thông báo mỗi khi có tin nhắn mới").color(pal.text_secondary).size(12.2));
                                            ui.add_space(8.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(10.0))
                                                .inner_margin(Margin::symmetric(24.0, 16.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 28.0;
                                                        for (en_val, label) in [(true, "Bật"), (false, "Tắt")] {
                                                            let selected = self.settings.enable_notifications == en_val;
                                                            ui.vertical_centered(|ui| {
                                                                let (lap_r, l_resp) = ui.allocate_exact_size(Vec2::new(148.0, 92.0), Sense::click());
                                                                let stroke_c = if selected {
                                                                    pal.primary_hover
                                                                } else {
                                                                    pal.border
                                                                };
                                                                let scr_r = Rect::from_min_max(Pos2::new(lap_r.left() + 14.0, lap_r.top() + 6.0), Pos2::new(lap_r.right() - 14.0, lap_r.bottom() - 16.0));
                                                                ui.painter().rect_stroke(scr_r, Rounding::same(4.0), Stroke::new(1.8_f32, stroke_c));
                                                                let base_r = Rect::from_min_max(Pos2::new(lap_r.left() + 6.0, lap_r.bottom() - 16.0), Pos2::new(lap_r.right() - 6.0, lap_r.bottom() - 6.0));
                                                                ui.painter().rect_stroke(base_r, Rounding::same(4.0), Stroke::new(1.8_f32, stroke_c));
                                                                if en_val {
                                                                    let toast_r = Rect::from_min_size(Pos2::new(scr_r.right() - 62.0, scr_r.top() + 8.0), Vec2::new(54.0, 22.0));
                                                                    ui.painter().rect_filled(toast_r, Rounding::same(3.0), pal.bg_selected);
                                                                }
                                                                ui.add_space(6.0);
                                                                let r_click = ui.radio(selected, label).clicked();
                                                                if l_resp.on_hover_cursor(CursorIcon::PointingHand).clicked() || r_click {
                                                                    self.settings.enable_notifications = en_val;
                                                                }
                                                            });
                                                        }
                                                    });
                                                });
                                            ui.add_space(16.0);

                                            ui.label(RichText::new("Âm thanh thông báo").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Phát âm thanh khi có tin nhắn & thông báo mới").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.play_notification_sound, pal);
                                                        });
                                                    });
                                                });
                                        }
                                        SettingsTab::Messages => {
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Tin nhắn nhanh").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.quick_messages_enabled, pal);
                                                        });
                                                    });
                                                });
                                            ui.add_space(14.0);

                                            ui.label(RichText::new("Thiết lập ẩn trò chuyện").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(2.0);
                                            ui.label(RichText::new("Đặt mã PIN cho các cuộc trò chuyện riêng tư để tránh người khác nhìn thấy trên máy của bạn").color(pal.text_secondary).size(12.0));
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Mã PIN").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.hidden_chat_pin_enabled, pal);
                                                        });
                                                    });
                                                });
                                            ui.add_space(14.0);

                                            ui.label(RichText::new("Chia mục Ưu tiên và Khác").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(2.0);
                                            ui.label(RichText::new("Tách riêng trò chuyện không ưu tiên và chuyển sang mục Khác để tập trung hơn").color(pal.text_secondary).size(12.0));
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Chia mục Ưu tiên").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.split_priority_inbox, pal);
                                                        });
                                                    });
                                                });
                                            ui.add_space(14.0);

                                            ui.label(RichText::new("Hiển thị nội dung file Excel").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(2.0);
                                            ui.label(RichText::new("Định dạng khi sao chép file Excel sau đó dán vào Zalo").color(pal.text_secondary).size(12.0));
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    for (val, lbl) in [("image", "Dạng hình"), ("text", "Dạng chữ"), ("ask", "Luôn hỏi")] {
                                                        ui.horizontal(|ui| {
                                                            ui.label(RichText::new(lbl).color(pal.text_primary).size(13.0));
                                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                                ui.radio_value(&mut self.settings.excel_paste_mode, val.into(), "");
                                                            });
                                                        });
                                                        ui.add_space(4.0);
                                                    }
                                                });
                                            ui.add_space(14.0);

                                            ui.label(RichText::new("Cài đặt khác").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Bấm đúp vào vùng cạnh tin nhắn để trả lời").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.double_click_to_reply, pal);
                                                        });
                                                    });
                                                    ui.add_space(8.0);
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Bạn sẽ thấy bạn bè bạn đang soạn tin nhắn").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.show_typing_indicator, pal);
                                                        });
                                                    });
                                                });
                                        }
                                        SettingsTab::Utilities => {
                                            ui.label(RichText::new("Gợi ý Sticker").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new("Hiện gợi ý Sticker phù hợp với nội dung tin nhắn đang soạn").color(pal.text_primary).size(13.2));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.suggest_stickers_while_typing, pal);
                                                        });
                                                    });
                                                });
                                            ui.add_space(16.0);

                                            ui.label(RichText::new("Gợi ý @").color(pal.text_primary).size(13.5).strong());
                                            ui.add_space(6.0);
                                            egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 12.0))
                                                .show(ui, |ui| {
                                                    // Row 1: Gợi ý nhắc tên
                                                    ui.horizontal(|ui| {
                                                        let (b_rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                                                        ui.painter().circle_filled(b_rect.center(), 15.0, Color32::from_rgb(224, 108, 117));
                                                        draw_lucide_icon(ui.painter(), b_rect.center(), lucide::USER, 15.0, Color32::WHITE);
                                                        ui.vertical(|ui| {
                                                            ui.label(RichText::new("Gợi ý nhắc tên").color(pal.text_primary).size(13.2).strong());
                                                            ui.label(RichText::new("Gợi ý nhắc tên theo nội dung đang soạn").color(pal.text_secondary).size(11.8));
                                                        });
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.suggest_mentions, pal);
                                                        });
                                                    });
                                                    ui.add_space(10.0);
                                                    // Row 2: Tìm kiếm GIF
                                                    ui.horizontal(|ui| {
                                                        let (b_rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                                                        ui.painter().circle_filled(b_rect.center(), 15.0, Color32::from_rgb(82, 139, 255));
                                                        ui.painter().text(b_rect.center(), egui::Align2::CENTER_CENTER, "GIF", FontId::proportional(10.5), Color32::WHITE);
                                                        ui.vertical(|ui| {
                                                            ui.label(RichText::new("Tìm kiếm GIF").color(pal.text_primary).size(13.2).strong());
                                                            ui.label(RichText::new("Gõ từ khóa để tìm GIF").color(pal.text_secondary).size(11.8));
                                                        });
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.enable_gif_search, pal);
                                                        });
                                                    });
                                                    ui.add_space(10.0);
                                                    // Row 3: Tìm sticker
                                                    ui.horizontal(|ui| {
                                                        let (b_rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                                                        ui.painter().circle_filled(b_rect.center(), 15.0, Color32::from_rgb(152, 88, 178));
                                                        draw_lucide_icon(ui.painter(), b_rect.center(), lucide::SMILE, 15.5, Color32::WHITE);
                                                        ui.vertical(|ui| {
                                                            ui.label(RichText::new("Tìm sticker").color(pal.text_primary).size(13.2).strong());
                                                            ui.label(RichText::new("Gõ từ khóa để tìm Sticker").color(pal.text_secondary).size(11.8));
                                                        });
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            render_toggle_switch(ui, &mut self.settings.enable_sticker_keyword_search, pal);
                                                        });
                                                    });
                                                });
                                        }
                                    }
                                });
                        });
                });
            });

        if theme_switched {
            self.theme_dirty = true;
            self.settings.save();
            self.show_toast(if self.dark_mode {
                "Đã chuyển sang giao diện Tối (Atom One Dark)"
            } else {
                "Đã chuyển sang giao diện Sáng (Zalo Light)"
            });
        }
        if trigger_sync {
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::RefreshThreads);
            }
            self.show_toast("Đang đồng bộ dữ liệu hội thoại từ máy chủ Zalo…");
        }
        if pick_dl_dir {
            if let Some(folder) = pick_native_file_dialog(false) {
                let dir_path = folder.parent().unwrap_or(&folder).to_path_buf();
                self.settings.download_dir = dir_path.display().to_string();
                self.settings.save();
                self.show_toast(format!("Đã đổi thư mục lưu file: {}", self.settings.download_dir));
            }
        }
        if open_dl_dir {
            let p = std::path::PathBuf::from(&self.settings.download_dir);
            let _ = std::fs::create_dir_all(&p);
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("explorer").arg(&p).spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open").arg(&p).spawn();
            }
            #[cfg(not(any(target_os = "windows", target_os = "macos")))]
            {
                let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
            }
        }

        if !open {
            self.settings_modal_open = false;
            self.settings.save();
        }
    }


    pub(super) fn render_call_overlay(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        let Some(mut call) = self.active_call.clone() else {
            return;
        };
        // Keep timer and video textures updating smoothly while call window is open
        ctx.request_repaint_after(Duration::from_millis(66));

        let elapsed_f = call.started_at.elapsed().as_secs_f32();
        let elapsed = elapsed_f as u64;
        let mins = elapsed / 60;
        let secs = elapsed % 60;
        let timer_str = format!("{mins:02}:{secs:02}");

        let mut hangup_clicked = false;
        let mut retry_clicked = false;

        let has_err = call.error_reason.is_some();
        let is_video_stage = call.is_video && !has_err;
        let default_w = if is_video_stage { 860.0 } else { 440.0 };
        let default_h = if is_video_stage { 660.0 } else { 470.0 };
        let win_title = if call.is_video {
            format!("Zalo Video Call — {}", call.partner_name)
        } else {
            format!("Zalo Voice Call — {}", call.partner_name)
        };

        let call_ava = self
            .threads
            .iter()
            .find(|t| t.id == call.partner_id || t.name == call.partner_name)
            .and_then(|t| t.avatar.clone());

        let viewport_id = egui::ViewportId::from_hash_of("zalo_dedicated_call_window");
        let builder = egui::ViewportBuilder::default()
            .with_title(win_title)
            .with_inner_size([default_w, default_h])
            .with_min_inner_size([380.0, 420.0]);

        ctx.show_viewport_immediate(viewport_id, builder, |vp_ctx, _class| {
            if vp_ctx.input(|i| i.viewport().close_requested()) {
                hangup_clicked = true;
            }
            if vp_ctx.input(|i| i.key_pressed(Key::Escape)) && call.error_reason.is_some() {
                hangup_clicked = true;
            }

            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(pal.bg_sidebar)
                        .inner_margin(Margin::symmetric(20.0, 16.0)),
                )
                .show(vp_ctx, |ui| {
                    let avail_w = ui.available_width();
                    let avail_h = ui.available_height();

                    // 1. Symmetrically Centered Top Toolbar: Badge · Mic · Camera · Refresh
                    ui.vertical_centered(|ui| {
                        egui::Frame::none()
                            .fill(pal.bg_elevated)
                            .stroke(Stroke::new(1.0_f32, pal.border))
                            .rounding(Rounding::same(14.0))
                            .inner_margin(Margin::symmetric(12.0, 6.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 8.0;
                                    let pill_col = if has_err {
                                        pal.danger.linear_multiply(0.16)
                                    } else {
                                        pal.accent_online.linear_multiply(0.15)
                                    };
                                    let pill_fg = if has_err {
                                        Color32::from_rgb(252, 165, 165)
                                    } else {
                                        Color32::from_rgb(110, 231, 183)
                                    };
                                    let badge_txt = if call.is_video {
                                        format!("{} Zalo Video · ZRTP", lucide::VIDEO)
                                    } else {
                                        format!("{} Zalo Voice · ZRTP", lucide::SHIELD_CHECK)
                                    };
                                    egui::Frame::none()
                                        .fill(pill_col)
                                        .rounding(Rounding::same(10.0))
                                        .inner_margin(Margin::symmetric(10.0, 4.0))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(badge_txt)
                                                    .size(11.5)
                                                    .color(pill_fg)
                                                    .strong(),
                                            );
                                        });

                                    let (mics, cams) = crate::media::list_media_devices(false);
                                    let cur_mic_lbl = if self.settings.selected_mic_device.is_empty()
                                    {
                                        mics.first().map(String::as_str).unwrap_or("Mic mặc định")
                                    } else {
                                        &self.settings.selected_mic_device
                                    };
                                    egui::ComboBox::from_id_salt("call_vp_mic_combo")
                                        .selected_text(format!(
                                            "{} {}",
                                            lucide::MIC,
                                            truncate_chars(cur_mic_lbl, 22)
                                        ))
                                        .width(190.0)
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_label(
                                                    self.settings.selected_mic_device.is_empty(),
                                                    "Mặc định hệ thống",
                                                )
                                                .clicked()
                                            {
                                                self.settings.selected_mic_device.clear();
                                                crate::media::set_selected_mic_device("");
                                                self.settings.save();
                                            }
                                            for m in &mics {
                                                if ui
                                                    .selectable_label(
                                                        self.settings.selected_mic_device == *m,
                                                        m,
                                                    )
                                                    .clicked()
                                                {
                                                    self.settings.selected_mic_device = m.clone();
                                                    crate::media::set_selected_mic_device(m);
                                                    self.settings.save();
                                                }
                                            }
                                        });

                                    let cur_cam_lbl =
                                        if self.settings.selected_camera_device.is_empty() {
                                            cams.first()
                                                .map(String::as_str)
                                                .unwrap_or("Camera mặc định")
                                        } else {
                                            &self.settings.selected_camera_device
                                        };
                                    egui::ComboBox::from_id_salt("call_vp_cam_combo")
                                        .selected_text(format!(
                                            "{} {}",
                                            lucide::VIDEO,
                                            truncate_chars(cur_cam_lbl, 22)
                                        ))
                                        .width(190.0)
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_label(
                                                    self.settings.selected_camera_device.is_empty(),
                                                    "Tự động (Camera đầu tiên)",
                                                )
                                                .clicked()
                                            {
                                                self.settings.selected_camera_device.clear();
                                                crate::media::set_selected_camera_device("");
                                                self.settings.save();
                                            }
                                            for c in &cams {
                                                if ui
                                                    .selectable_label(
                                                        self.settings.selected_camera_device == *c,
                                                        c,
                                                    )
                                                    .clicked()
                                                {
                                                    self.settings.selected_camera_device =
                                                        c.clone();
                                                    crate::media::set_selected_camera_device(c);
                                                    self.settings.save();
                                                }
                                            }
                                        });

                                    if ui
                                        .small_button(lucide::REFRESH)
                                        .on_hover_text("Quét lại danh sách Mic & Camera")
                                        .clicked()
                                    {
                                        let _ = crate::media::list_media_devices(true);
                                    }
                                });
                            });
                    });

                    ui.add_space(12.0);

                    // 2. Main Stage — Strictly Centered
                    ui.vertical_centered(|ui| {
                        if is_video_stage {
                            let stage_w = avail_w.max(320.0);
                            let stage_h = (avail_h - 156.0).max(240.0);
                            let (stage_rect, _) = ui.allocate_exact_size(
                                Vec2::new(stage_w, stage_h),
                                Sense::hover(),
                            );
                            let painter = ui.painter();
                            painter.rect_filled(
                                stage_rect,
                                Rounding::same(16.0),
                                Color32::from_rgb(15, 20, 30),
                            );
                            painter.rect_stroke(
                                stage_rect,
                                Rounding::same(16.0),
                                Stroke::new(1.0_f32, pal.border),
                            );

                            // Remote video frame card: fitted tightly to the true video aspect ratio (no black bars!)
                            if let Some(vtex) = self.sticker_textures.get("call_remote_video") {
                                let inner = stage_rect.shrink(10.0);
                                let tex_size = vtex.size_vec2();
                                let tex_aspect = if tex_size.y > 0.0 {
                                    tex_size.x / tex_size.y
                                } else {
                                    4.0 / 3.0
                                };
                                let box_aspect = inner.width() / inner.height().max(1.0);
                                let draw_size = if box_aspect > tex_aspect {
                                    Vec2::new(inner.height() * tex_aspect, inner.height())
                                } else {
                                    Vec2::new(inner.width(), inner.width() / tex_aspect)
                                };
                                let draw_rect =
                                    Rect::from_center_size(inner.center(), draw_size);
                                painter.rect_filled(
                                    draw_rect.expand(2.0),
                                    Rounding::same(12.0),
                                    Color32::from_rgb(8, 11, 18),
                                );
                                painter.image(
                                    vtex.id(),
                                    draw_rect,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                                painter.rect_stroke(
                                    draw_rect.expand(1.0),
                                    Rounding::same(12.0),
                                    Stroke::new(
                                        1.2_f32,
                                        Color32::from_rgba_unmultiplied(148, 163, 184, 70),
                                    ),
                                );
                            } else {
                                let center = Pos2::new(
                                    stage_rect.center().x,
                                    stage_rect.center().y - 16.0,
                                );
                                let pulse =
                                    ((elapsed_f * 2.6).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                                painter.circle_filled(
                                    center,
                                    46.0 + pulse * 6.0,
                                    pal.primary.linear_multiply(0.12),
                                );
                                painter.circle_stroke(
                                    center,
                                    43.0,
                                    Stroke::new(1.8_f32, pal.primary_hover.linear_multiply(0.8)),
                                );
                                paint_user_avatar(
                                    painter,
                                    center,
                                    40.0,
                                    &call.partner_name,
                                    call.is_group,
                                    call_ava.as_deref(),
                                    &self.sticker_textures,
                                    &mut self.loading_stickers,
                                );
                                painter.text(
                                    Pos2::new(stage_rect.center().x, center.y + 56.0),
                                    egui::Align2::CENTER_CENTER,
                                    &call.partner_name,
                                    FontId::proportional(17.5),
                                    pal.text_primary,
                                );
                            }

                            // Local Camera Picture-in-Picture (PIP) preview in bottom-right corner
                            let pip_w = 184.0_f32;
                            let pip_h = 138.0_f32;
                            let pip_rect = Rect::from_min_size(
                                Pos2::new(
                                    stage_rect.right() - pip_w - 16.0,
                                    stage_rect.bottom() - pip_h - 16.0,
                                ),
                                Vec2::new(pip_w, pip_h),
                            );
                            painter.rect_filled(
                                pip_rect,
                                Rounding::same(12.0),
                                pal.bg_elevated,
                            );
                            if call.cam_enabled {
                                if let Some(ltex) = self.sticker_textures.get("call_local_video") {
                                    let inner_pip = pip_rect.shrink(2.0);
                                    let lsize = ltex.size_vec2();
                                    let laspect = if lsize.y > 0.0 {
                                        lsize.x / lsize.y
                                    } else {
                                        4.0 / 3.0
                                    };
                                    let paspect = inner_pip.width() / inner_pip.height().max(1.0);
                                    let uv = if laspect > paspect {
                                        let frac = paspect / laspect;
                                        let m = (1.0 - frac) * 0.5;
                                        Rect::from_min_max(Pos2::new(m, 0.0), Pos2::new(1.0 - m, 1.0))
                                    } else {
                                        let frac = laspect / paspect;
                                        let m = (1.0 - frac) * 0.5;
                                        Rect::from_min_max(Pos2::new(0.0, m), Pos2::new(1.0, 1.0 - m))
                                    };
                                    painter.image(
                                        ltex.id(),
                                        inner_pip,
                                        uv,
                                        Color32::WHITE,
                                    );
                                } else {
                                    let self_disp = if self.user_name.is_empty() {
                                        "Tôi"
                                    } else {
                                        &self.user_name
                                    };
                                    paint_user_avatar(
                                        painter,
                                        Pos2::new(pip_rect.center().x, pip_rect.center().y - 10.0),
                                        20.0,
                                        self_disp,
                                        false,
                                        self.user_avatar.as_deref(),
                                        &self.sticker_textures,
                                        &mut self.loading_stickers,
                                    );
                                }
                            } else {
                                let self_disp = if self.user_name.is_empty() {
                                    "Tôi"
                                } else {
                                    &self.user_name
                                };
                                paint_user_avatar(
                                    painter,
                                    Pos2::new(pip_rect.center().x, pip_rect.center().y - 10.0),
                                    20.0,
                                    self_disp,
                                    false,
                                    self.user_avatar.as_deref(),
                                    &self.sticker_textures,
                                    &mut self.loading_stickers,
                                );
                            }
                            let pip_badge_rect = Rect::from_center_size(
                                Pos2::new(pip_rect.center().x, pip_rect.bottom() - 14.0),
                                Vec2::new(124.0, 20.0),
                            );
                            painter.rect_filled(
                                pip_badge_rect,
                                Rounding::same(10.0),
                                Color32::from_rgba_unmultiplied(8, 11, 18, 185),
                            );
                            let pip_status = if call.cam_enabled {
                                format!("{} Camera đang bật", lucide::VIDEO)
                            } else {
                                format!("{} Camera đã tắt", lucide::VIDEO_OFF)
                            };
                            painter.text(
                                pip_badge_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                pip_status,
                                FontId::proportional(10.5),
                                if call.cam_enabled {
                                    pal.accent_online
                                } else {
                                    pal.text_muted
                                },
                            );
                            painter.rect_stroke(
                                pip_rect,
                                Rounding::same(12.0),
                                Stroke::new(
                                    1.4_f32,
                                    if call.cam_enabled {
                                        pal.primary_hover
                                    } else {
                                        pal.border
                                    },
                                ),
                            );
                            ui.add_space(10.0);
                        } else {
                            ui.add_space(18.0);
                            let (av_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(96.0), Sense::hover());
                            let center = av_rect.center();
                            let ring_col = if has_err {
                                pal.danger
                            } else if call.is_incoming_ringing {
                                pal.accent_online
                            } else {
                                pal.primary_hover
                            };
                            let pulse = ((elapsed_f * 2.6).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                            ui.painter().circle_filled(
                                center,
                                44.0 + pulse * 5.0,
                                ring_col.linear_multiply(0.10),
                            );
                            ui.painter().circle_stroke(
                                center,
                                42.0,
                                Stroke::new(1.8_f32, ring_col.linear_multiply(0.75)),
                            );
                            paint_user_avatar(
                                ui.painter(),
                                center,
                                38.0,
                                &call.partner_name,
                                call.is_group,
                                call_ava.as_deref(),
                                &self.sticker_textures,
                                &mut self.loading_stickers,
                            );

                            ui.add_space(10.0);
                            ui.label(
                                RichText::new(&call.partner_name)
                                    .size(18.5)
                                    .color(pal.text_primary)
                                    .strong(),
                            );
                            ui.add_space(4.0);
                        }

                        // 3. Status + Timer Pill — Perfectly Centered
                        if let Some(err_msg) = &call.error_reason {
                            ui.label(
                                RichText::new("Cuộc gọi chưa kết nối được")
                                    .size(12.5)
                                    .color(Color32::from_rgb(248, 113, 113))
                                    .strong(),
                            );
                            ui.add_space(6.0);
                            let clean_err = err_msg
                                .strip_prefix("Lỗi khởi tạo cuộc gọi (401): ")
                                .unwrap_or(err_msg);
                            egui::Frame::none()
                                .fill(Color32::from_rgba_unmultiplied(127, 29, 29, 68))
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgba_unmultiplied(239, 68, 68, 110),
                                ))
                                .rounding(Rounding::same(12.0))
                                .inner_margin(Margin::symmetric(12.0, 8.0))
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(clean_err)
                                                .size(11.8)
                                                .color(Color32::from_rgb(254, 226, 226)),
                                        )
                                        .wrap(),
                                    );
                                });
                        } else {
                            egui::Frame::none()
                                .fill(Color32::from_rgb(30, 41, 59))
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .rounding(Rounding::same(12.0))
                                .inner_margin(Margin::symmetric(14.0, 4.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 8.0;
                                        ui.label(
                                            RichText::new(&call.state_label)
                                                .size(12.5)
                                                .color(Color32::from_rgb(203, 213, 225)),
                                        );
                                        ui.label(
                                            RichText::new("·")
                                                .size(13.0)
                                                .color(pal.text_muted),
                                        );
                                        ui.label(
                                            RichText::new(&timer_str)
                                                .monospace()
                                                .size(13.5)
                                                .color(pal.accent_online)
                                                .strong(),
                                        );
                                    });
                                });
                        }

                        ui.add_space(10.0);

                        // 4. Bottom Action Dock — Exact Mathematical Center (`196.0 px` total width: `48 + 22 + 56 + 22 + 48`)
                        if has_err {
                            let dock_w = 216.0_f32;
                            let (dock_rect, _) =
                                ui.allocate_exact_size(Vec2::new(dock_w, 40.0), Sense::hover());
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(dock_rect), |ui| {
                                ui.horizontal_centered(|ui| {
                                    ui.spacing_mut().item_spacing.x = 12.0;
                                    let retry_btn = egui::Button::new(
                                        RichText::new(format!("{} Gọi lại", lucide::PHONE_CALL))
                                            .size(12.8)
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(pal.primary)
                                    .rounding(Rounding::same(18.0))
                                    .min_size(Vec2::new(104.0, 36.0));
                                    if ui
                                        .add(retry_btn)
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        retry_clicked = true;
                                    }

                                    let close_btn = egui::Button::new(
                                        RichText::new(format!("{} Đóng", lucide::X))
                                            .size(12.8)
                                            .color(Color32::from_rgb(226, 232, 240))
                                            .strong(),
                                    )
                                    .fill(Color32::from_rgb(30, 41, 59))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                                    .rounding(Rounding::same(18.0))
                                    .min_size(Vec2::new(100.0, 36.0));
                                    if ui
                                        .add(close_btn)
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        hangup_clicked = true;
                                    }
                                });
                            });
                        } else if call.is_incoming_ringing {
                            let dock_w = 148.0_f32;
                            let (dock_rect, _) =
                                ui.allocate_exact_size(Vec2::new(dock_w, 72.0), Sense::hover());
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(dock_rect), |ui| {
                                ui.horizontal_centered(|ui| {
                                    ui.spacing_mut().item_spacing.x = 44.0;
                                    ui.vertical_centered(|ui| {
                                        let (dec_r, dec_resp) = ui.allocate_exact_size(
                                            Vec2::splat(52.0),
                                            Sense::click(),
                                        );
                                        ui.painter().circle_filled(
                                            dec_r.center(),
                                            25.0,
                                            pal.danger,
                                        );
                                        draw_lucide_icon(
                                            ui.painter(),
                                            dec_r.center(),
                                            lucide::PHONE_OFF,
                                            20.0,
                                            Color32::WHITE,
                                        );
                                        if dec_resp
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .on_hover_text("Từ chối cuộc gọi")
                                            .clicked()
                                        {
                                            if let Some(tx) = WORKER_TX.get() {
                                                let _ = tx
                                                    .send(WorkerCmd::AnswerCall { accept: false });
                                            }
                                            self.active_call = None;
                                        }
                                        ui.label(
                                            RichText::new("Từ chối")
                                                .size(11.0)
                                                .color(pal.text_secondary),
                                        );
                                    });

                                    ui.vertical_centered(|ui| {
                                        let (acc_r, acc_resp) = ui.allocate_exact_size(
                                            Vec2::splat(52.0),
                                            Sense::click(),
                                        );
                                        ui.painter().circle_filled(
                                            acc_r.center(),
                                            25.0,
                                            pal.accent_online,
                                        );
                                        draw_lucide_icon(
                                            ui.painter(),
                                            acc_r.center(),
                                            lucide::PHONE_CALL,
                                            20.0,
                                            Color32::WHITE,
                                        );
                                        if acc_resp
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .on_hover_text("Nghe máy")
                                            .clicked()
                                        {
                                            call.is_incoming_ringing = false;
                                            call.state_label = "Đang kết nối cuộc gọi…".into();
                                            if let Some(tx) = WORKER_TX.get() {
                                                let _ = tx
                                                    .send(WorkerCmd::AnswerCall { accept: true });
                                            }
                                        }
                                        ui.label(
                                            RichText::new("Nghe máy")
                                                .size(11.0)
                                                .color(pal.accent_online),
                                        );
                                    });
                                });
                            });
                        } else {
                            let dock_w = 196.0_f32;
                            let (dock_rect, _) =
                                ui.allocate_exact_size(Vec2::new(dock_w, 56.0), Sense::hover());
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(dock_rect), |ui| {
                                ui.horizontal_centered(|ui| {
                                    ui.spacing_mut().item_spacing.x = 22.0;

                                    // 1) Circular Mic Button
                                    let (mic_r, mic_resp) =
                                        ui.allocate_exact_size(Vec2::splat(48.0), Sense::click());
                                    let mic_bg = if call.mic_muted {
                                        pal.warning.linear_multiply(0.28)
                                    } else if mic_resp.hovered() {
                                        Color32::from_rgb(51, 65, 85)
                                    } else {
                                        Color32::from_rgb(30, 41, 59)
                                    };
                                    ui.painter().circle_filled(mic_r.center(), 23.0, mic_bg);
                                    ui.painter().circle_stroke(
                                        mic_r.center(),
                                        23.0,
                                        Stroke::new(
                                            1.0_f32,
                                            if call.mic_muted {
                                                pal.warning
                                            } else {
                                                Color32::from_rgb(71, 85, 105)
                                            },
                                        ),
                                    );
                                    draw_lucide_icon(
                                        ui.painter(),
                                        mic_r.center(),
                                        if call.mic_muted {
                                            lucide::MIC_OFF
                                        } else {
                                            lucide::MIC
                                        },
                                        18.5,
                                        if call.mic_muted {
                                            pal.warning
                                        } else {
                                            Color32::WHITE
                                        },
                                    );
                                    if mic_resp
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .on_hover_text(if call.mic_muted {
                                            "Bật Mic"
                                        } else {
                                            "Tắt Mic"
                                        })
                                        .clicked()
                                    {
                                        call.mic_muted = !call.mic_muted;
                                        if let Some(tx) = WORKER_TX.get() {
                                            let _ = tx.send(WorkerCmd::SetCallMediaState {
                                                is_on_mic: !call.mic_muted,
                                                is_on_cam: call.cam_enabled,
                                            });
                                        }
                                    }

                                    // 2) Prominent Circular Red Hangup Button (dead-center of window)
                                    let (end_r, end_resp) =
                                        ui.allocate_exact_size(Vec2::splat(56.0), Sense::click());
                                    ui.painter().circle_filled(
                                        end_r.center(),
                                        27.0,
                                        if end_resp.hovered() {
                                            Color32::from_rgb(220, 38, 38)
                                        } else {
                                            pal.danger
                                        },
                                    );
                                    draw_lucide_icon(
                                        ui.painter(),
                                        end_r.center(),
                                        lucide::PHONE_OFF,
                                        21.0,
                                        Color32::WHITE,
                                    );
                                    if end_resp
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .on_hover_text("Kết thúc cuộc gọi")
                                        .clicked()
                                    {
                                        hangup_clicked = true;
                                    }

                                    // 3) Circular Camera Button
                                    let (cam_r, cam_resp) =
                                        ui.allocate_exact_size(Vec2::splat(48.0), Sense::click());
                                    let cam_bg = if call.cam_enabled {
                                        pal.primary.linear_multiply(0.32)
                                    } else if cam_resp.hovered() {
                                        Color32::from_rgb(51, 65, 85)
                                    } else {
                                        Color32::from_rgb(30, 41, 59)
                                    };
                                    ui.painter().circle_filled(cam_r.center(), 23.0, cam_bg);
                                    ui.painter().circle_stroke(
                                        cam_r.center(),
                                        23.0,
                                        Stroke::new(
                                            1.0_f32,
                                            if call.cam_enabled {
                                                pal.primary_hover
                                            } else {
                                                Color32::from_rgb(71, 85, 105)
                                            },
                                        ),
                                    );
                                    draw_lucide_icon(
                                        ui.painter(),
                                        cam_r.center(),
                                        if call.cam_enabled {
                                            lucide::VIDEO
                                        } else {
                                            lucide::VIDEO_OFF
                                        },
                                        18.5,
                                        Color32::WHITE,
                                    );
                                    if cam_resp
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .on_hover_text(if call.cam_enabled {
                                            "Tắt Camera"
                                        } else {
                                            "Bật Camera"
                                        })
                                        .clicked()
                                    {
                                        call.cam_enabled = !call.cam_enabled;
                                        call.is_video = call.cam_enabled;
                                        if !call.cam_enabled {
                                            self.sticker_textures.remove("call_local_video");
                                        }
                                        if let Some(tx) = WORKER_TX.get() {
                                            let _ = tx.send(WorkerCmd::SetCallMediaState {
                                                is_on_mic: !call.mic_muted,
                                                is_on_cam: call.cam_enabled,
                                            });
                                        }
                                    }
                                });
                            });
                        }
                    });
                });
        });

        if retry_clicked {
            call.error_code = None;
            call.error_reason = None;
            call.started_at = std::time::Instant::now();
            call.state_label = if call.is_video {
                "Đang kết nối cuộc gọi video…".into()
            } else {
                "Đang kết nối cuộc gọi thoại…".into()
            };
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::StartCall {
                    partner_id: call.partner_id.clone(),
                    partner_name: call.partner_name.clone(),
                    partner_avatar: None,
                    is_video: call.is_video,
                    is_group: call.is_group,
                });
            }
            self.active_call = Some(call);
        } else if hangup_clicked {
            self.active_call = None;
            self.sticker_textures.remove("call_remote_video");
            self.sticker_textures.remove("call_local_video");
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::EndCall);
            }
        } else if self.active_call.is_some() {
            self.active_call = Some(call);
        }
    }

    pub(super) fn render_forward_dialog(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        if self.forward_msg.is_none() {
            return;
        }
        let preview = self
            .forward_msg
            .as_ref()
            .map(|m| {
                if m.sticker_id.is_some() {
                    "[Nhãn dán]".to_string()
                } else {
                    truncate_chars(&m.text, 80).into_owned()
                }
            })
            .unwrap_or_default();
        let query = self.forward_filter.to_lowercase();
        let candidates: Vec<(String, String, bool)> = self
            .threads
            .iter()
            .filter(|t| query.is_empty() || t.name.to_lowercase().contains(&query))
            .map(|t| (t.id.clone(), t.name.clone(), t.is_group))
            .collect();
        let mut open = true;
        let mut chosen: Option<(String, bool)> = None;
        egui::Window::new("Chuyển tiếp tin nhắn")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(360.0, 420.0))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!("Nội dung: {preview}"))
                        .size(12.5)
                        .color(pal.text_secondary),
                );
                ui.add_space(6.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.forward_filter)
                        .hint_text("Tìm hội thoại…")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (id, name, is_group) in &candidates {
                            let label = if *is_group {
                                format!("[Nhóm] {name}")
                            } else {
                                name.clone()
                            };
                            let btn = egui::Button::new(RichText::new(label).size(13.5))
                                .fill(pal.bg_input);
                            if ui
                                .add_sized([ui.available_width(), 30.0], btn)
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                chosen = Some((id.clone(), *is_group));
                            }
                        }
                    });
            });
        if let Some((target_id, target_is_group)) = chosen {
            if let (Some(msg), Some(tx)) = (self.forward_msg.take(), WORKER_TX.get()) {
                let _ = tx.send(WorkerCmd::Forward {
                    target_id,
                    target_is_group,
                    msg,
                });
                self.status_msg = "Đang chuyển tiếp…".into();
            }
        } else if !open {
            self.forward_msg = None;
        }
    }

    pub(super) fn render_image_viewer_modal(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        let Some(mut viewer) = self.image_viewer.clone() else {
            return;
        };

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.image_viewer = None;
            return;
        }

        let screen_rect = ctx.screen_rect();
        // Start below the 32px custom frameless title bar so window controls stay accessible
        let viewer_top = (screen_rect.top() + 32.0).min(screen_rect.bottom());
        let toolbar_h = 46.0_f32;
        let toolbar_rect = Rect::from_min_max(
            Pos2::new(screen_rect.left(), viewer_top),
            Pos2::new(screen_rect.right(), (viewer_top + toolbar_h).min(screen_rect.bottom())),
        );
        let stage_area_rect = Rect::from_min_max(
            Pos2::new(screen_rect.left(), toolbar_rect.bottom()),
            screen_rect.max,
        );
        let mut should_close = false;

        let active_tex = viewer
            .hd_tex_key
            .as_ref()
            .and_then(|hk| self.sticker_textures.get(hk))
            .or_else(|| self.sticker_textures.get(&viewer.tex_key))
            .cloned();

        let max_stage_w = (stage_area_rect.width() - 48.0).max(160.0);
        let max_stage_h = (stage_area_rect.height() - 36.0).max(120.0);

        let (base_w, base_h, natural_dims) = if let Some(ref tex) = active_tex {
            let [tw, th] = tex.size();
            let tw_f = (tw.max(1)) as f32;
            let th_f = (th.max(1)) as f32;
            let fit_scale = (max_stage_w / tw_f).min(max_stage_h / th_f).min(1.5);
            (tw_f * fit_scale, th_f * fit_scale, Some((tw, th)))
        } else {
            (340.0_f32.min(max_stage_w), 220.0_f32.min(max_stage_h), None)
        };

        // Mouse-wheel zoom (zooms smoothly around cursor without clamping disp_w/disp_h to max_stage)
        let scroll_y = ctx.input(|i| {
            if i.raw_scroll_delta.y.abs() > 0.1 {
                i.raw_scroll_delta.y
            } else {
                i.smooth_scroll_delta.y
            }
        });
        if scroll_y.abs() > 0.1 {
            let old_zoom = viewer.zoom;
            let factor = if scroll_y > 0.0 { 1.15 } else { 1.0 / 1.15 };
            let new_zoom = (old_zoom * factor).clamp(0.25, 8.0);
            if let Some(ptr) = ctx.input(|i| i.pointer.hover_pos()) {
                if stage_area_rect.contains(ptr) {
                    let rel = ptr - (stage_area_rect.center() + viewer.pan);
                    viewer.pan -= rel * (new_zoom / old_zoom - 1.0);
                }
            }
            viewer.zoom = new_zoom;
        }

        let disp_w = (base_w * viewer.zoom).max(48.0);
        let disp_h = (base_h * viewer.zoom).max(48.0);

        // Clamp pan so the image stays centered when <= stage size, and pans within bounds when enlarged
        let max_pan_x = ((disp_w - stage_area_rect.width() + 40.0) * 0.5).max(0.0);
        let max_pan_y = ((disp_h - stage_area_rect.height() + 40.0) * 0.5).max(0.0);
        viewer.pan.x = viewer.pan.x.clamp(-max_pan_x, max_pan_x);
        viewer.pan.y = viewer.pan.y.clamp(-max_pan_y, max_pan_y);

        // 1. Unified Clipped Stage Canvas (never uses a floating Area at img_pos so it NEVER shifts left!)
        egui::Area::new(Id::new("image_viewer_backdrop"))
            .order(egui::Order::Foreground)
            .constrain(false)
            .fixed_pos(stage_area_rect.min)
            .show(ctx, |ui| {
                let (bg_rect, bg_resp) =
                    ui.allocate_exact_size(stage_area_rect.size(), Sense::click_and_drag());
                let painter = ui.painter().with_clip_rect(bg_rect);
                let bg_col = if pal.is_dark {
                    Color32::from_rgba_unmultiplied(24, 26, 31, 244)
                } else {
                    Color32::from_rgba_unmultiplied(24, 28, 36, 228)
                };
                painter.rect_filled(bg_rect, Rounding::ZERO, bg_col);

                let img_center = bg_rect.center() + viewer.pan;
                let img_rect = Rect::from_center_size(img_center, Vec2::new(disp_w, disp_h));

                if let Some(ref tex) = active_tex {
                    painter.image(
                        tex.id(),
                        img_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    painter.rect_stroke(
                        img_rect,
                        Rounding::ZERO,
                        Stroke::new(1.0_f32, pal.border),
                    );

                    if bg_resp.dragged_by(egui::PointerButton::Primary) {
                        viewer.pan += bg_resp.drag_delta();
                        viewer.pan.x = viewer.pan.x.clamp(-max_pan_x, max_pan_x);
                        viewer.pan.y = viewer.pan.y.clamp(-max_pan_y, max_pan_y);
                        ctx.set_cursor_icon(CursorIcon::Grabbing);
                    } else if let Some(ptr) = bg_resp.hover_pos() {
                        if img_rect.contains(ptr) && (max_pan_x > 1.0 || max_pan_y > 1.0) {
                            ctx.set_cursor_icon(CursorIcon::Grab);
                        }
                    }

                    if bg_resp.double_clicked() {
                        if (viewer.zoom - 1.0).abs() < 0.05 {
                            viewer.zoom = 2.0;
                        } else {
                            viewer.zoom = 1.0;
                            viewer.pan = Vec2::ZERO;
                        }
                    } else if bg_resp.clicked() {
                        let ptr = bg_resp
                            .interact_pointer_pos()
                            .or_else(|| ctx.input(|i| i.pointer.latest_pos()));
                        if ptr.map(|p| !img_rect.contains(p)).unwrap_or(false) {
                            should_close = true;
                        }
                    }
                } else {
                    painter.rect_filled(img_rect, Rounding::same(8.0), pal.bg_elevated);
                    painter.rect_stroke(
                        img_rect,
                        Rounding::same(8.0),
                        Stroke::new(1.0_f32, pal.border),
                    );
                    painter.text(
                        img_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Đang tải hình ảnh…",
                        FontId::proportional(13.0),
                        pal.text_primary,
                    );
                    if bg_resp.clicked() {
                        should_close = true;
                    }
                }
            });
        // 3. Clean Unified Top Toolbar Bar (docked under custom title bar, matching Atom One Dark pal.bg_sidebar)
        let clean_title = {
            let mut t = viewer
                .title
                .trim()
                .trim_start_matches(lucide::IMAGE)
                .trim();
            if let Some(paren_idx) = t.rfind(" (") {
                if t[paren_idx..].contains('×') || t[paren_idx..].contains("KB") || t[paren_idx..].contains("MB") {
                    t = t[..paren_idx].trim();
                }
            }
            if t.is_empty() {
                "Hình ảnh đính kèm"
            } else {
                t
            }
        };

        egui::Area::new(Id::new("image_viewer_top_bar"))
            .order(egui::Order::Tooltip)
            .fixed_pos(toolbar_rect.min)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(pal.bg_sidebar)
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .inner_margin(Margin::symmetric(16.0, 6.0))
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(
                            (toolbar_rect.width() - 32.0).max(200.0),
                            toolbar_h - 12.0,
                        ));
                        ui.set_max_size(Vec2::new(
                            (toolbar_rect.width() - 32.0).max(200.0),
                            toolbar_h - 12.0,
                        ));
                        ui.horizontal_centered(|ui| {
                            // Left: Subtle icon badge + filename + sender/time/dimensions
                            let (badge_r, _) =
                                ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                            ui.painter().rect_filled(
                                badge_r,
                                Rounding::same(6.0),
                                pal.bg_elevated,
                            );
                            draw_icon_image(
                                ui.painter(),
                                badge_r.center(),
                                15.0,
                                pal.primary_hover,
                            );
                            ui.add_space(4.0);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                ui.label(
                                    RichText::new(truncate_chars(clean_title, 44))
                                        .size(13.0)
                                        .color(pal.text_primary)
                                        .strong(),
                                );
                                let dim_str = match natural_dims {
                                    Some((tw, th)) => format!(" · {tw}×{th} px"),
                                    None => String::new(),
                                };
                                let sub = if viewer.time.is_empty() {
                                    format!("{}{dim_str}", viewer.sender)
                                } else {
                                    format!("Gửi bởi {} lúc {}{dim_str}", viewer.sender, viewer.time)
                                };
                                ui.label(
                                    RichText::new(sub)
                                        .size(11.0)
                                        .color(pal.text_muted),
                                );
                            });

                            // Right: Unified Zoom + Actions + Close
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;

                                // Close button (30x28, Atom One Dark style)
                                let (close_r, close_resp) = ui.allocate_exact_size(
                                    Vec2::new(30.0, 28.0),
                                    Sense::click(),
                                );
                                ui.painter().rect_filled(
                                    close_r,
                                    Rounding::same(6.0),
                                    if close_resp.hovered() {
                                        pal.danger
                                    } else {
                                        pal.bg_elevated
                                    },
                                );
                                ui.painter().rect_stroke(
                                    close_r,
                                    Rounding::same(6.0),
                                    Stroke::new(1.0_f32, pal.border),
                                );
                                draw_lucide_icon(
                                    ui.painter(),
                                    close_r.center(),
                                    lucide::X,
                                    14.5,
                                    if close_resp.hovered() {
                                        Color32::WHITE
                                    } else {
                                        pal.text_secondary
                                    },
                                );
                                if close_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Đóng xem ảnh (Esc)")
                                    .clicked()
                                {
                                    should_close = true;
                                }

                                if !viewer.full_url.is_empty() {
                                    let ext_btn = egui::Button::new(
                                        RichText::new(format!("{} Mở gốc", lucide::EXTERNAL_LINK))
                                            .size(11.8)
                                            .color(pal.text_primary),
                                    )
                                    .fill(pal.bg_elevated)
                                    .stroke(Stroke::new(1.0_f32, pal.border))
                                    .rounding(Rounding::same(6.0))
                                    .min_size(Vec2::new(72.0, 28.0));
                                    if ui
                                        .add(ext_btn)
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .on_hover_text(&viewer.full_url)
                                        .clicked()
                                    {
                                        open_external_url(&viewer.full_url);
                                    }

                                    let copy_btn = egui::Button::new(
                                        RichText::new(format!("{} Sao chép", lucide::COPY))
                                            .size(11.8)
                                            .color(pal.text_secondary),
                                    )
                                    .fill(pal.bg_elevated)
                                    .stroke(Stroke::new(1.0_f32, pal.border))
                                    .rounding(Rounding::same(6.0))
                                    .min_size(Vec2::new(78.0, 28.0));
                                    if ui
                                        .add(copy_btn)
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        ui.ctx().copy_text(viewer.full_url.clone());
                                        self.show_toast("Đã sao chép đường dẫn ảnh vào bộ nhớ tạm");
                                    }
                                }

                                // Zoom In (+)
                                let zin_btn = egui::Button::new(
                                    RichText::new(lucide::PLUS)
                                        .size(12.5)
                                        .color(pal.text_secondary),
                                )
                                .fill(pal.bg_elevated)
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .rounding(Rounding::same(6.0))
                                .min_size(Vec2::new(28.0, 28.0));
                                if ui
                                    .add(zin_btn)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Phóng to (+25%)")
                                    .clicked()
                                {
                                    viewer.zoom = (viewer.zoom + 0.25).min(8.0);
                                }

                                // Zoom Percentage Reset
                                let zoom_pct = format!("{:.0}%", viewer.zoom * 100.0);
                                let zreset_btn = egui::Button::new(
                                    RichText::new(zoom_pct)
                                        .size(11.8)
                                        .color(pal.primary_hover)
                                        .strong(),
                                )
                                .fill(pal.bg_input)
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .rounding(Rounding::same(6.0))
                                .min_size(Vec2::new(50.0, 28.0));
                                if ui
                                    .add(zreset_btn)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Đặt lại 100% (hoặc nhấp đúp vào ảnh)")
                                    .clicked()
                                {
                                    viewer.zoom = 1.0;
                                    viewer.pan = Vec2::ZERO;
                                }

                                // Zoom Out (-)
                                let zout_btn = egui::Button::new(
                                    RichText::new(lucide::MINUS)
                                        .size(12.5)
                                        .color(pal.text_secondary),
                                )
                                .fill(pal.bg_elevated)
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .rounding(Rounding::same(6.0))
                                .min_size(Vec2::new(28.0, 28.0));
                                if ui
                                    .add(zout_btn)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Thu nhỏ (-25%)")
                                    .clicked()
                                {
                                    viewer.zoom = (viewer.zoom - 0.25).max(0.25);
                                    if viewer.zoom <= 1.0 {
                                        viewer.pan = Vec2::ZERO;
                                    }
                                }
                            });
                        });
                    });
            });

        if should_close {
            self.image_viewer = None;
        } else {
            self.image_viewer = Some(viewer);
        }
    }


    /// Detect an `http://` or `https://` URL in `self.input_message` and asynchronously fetch its
    /// OpenGraph / Zalo `parselink` metadata (`title`, `desc`, `thumb`, `src`).
    pub(super) fn resolve_composer_link_preview(
        &mut self,
        ctx: &egui::Context,
    ) -> Option<crate::chat::LinkMetadata> {
        let Some(url) = crate::chat::extract_first_url(&self.input_message) else {
            self.dismissed_link_preview_for.clear();
            return None;
        };
        if self.dismissed_link_preview_for == url {
            return None;
        }
        let elapsed = self.last_composer_edit_at.elapsed();
        let debounce = Duration::from_millis(200);
        if elapsed < debounce {
            ctx.request_repaint_after(debounce - elapsed + Duration::from_millis(15));
            return self.link_metadata_cache.get(&url).cloned();
        }
        if let Some(cached) = self.link_metadata_cache.get(&url).cloned() {
            return Some(cached);
        }
        if self.loading_link_previews.insert(url.clone()) {
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::ParseLinkPreview { url: url.clone() });
            }
        }
        let domain = url
            .split("://")
            .nth(1)
            .and_then(|r| r.split('/').next())
            .unwrap_or("Liên kết")
            .to_string();
        Some(crate::chat::LinkMetadata {
            url,
            title: "Đang tải thông tin liên kết…".into(),
            desc: String::new(),
            thumb: String::new(),
            src: domain,
        })
    }

    /// Render a floating Link Preview card (`egui::Area`) anchored directly above `composer_rect`
    /// when the user types or pastes a URL into the composer input.
    pub(super) fn render_composer_link_preview_popover(
        &mut self,
        ctx: &egui::Context,
        composer_rect: Rect,
        meta: &crate::chat::LinkMetadata,
        pal: &ThemePalette,
    ) {
        let strip_h = 72.0_f32;
        let strip_w = composer_rect.width().max(220.0);
        let pos = Pos2::new(composer_rect.left(), (composer_rect.top() - strip_h).max(48.0));
        let mut dismiss_clicked = false;

        egui::Area::new(Id::new("zalo_composer_link_preview_popover"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(if pal.is_dark {
                        Color32::from_rgba_unmultiplied(33, 37, 43, 248)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 250)
                    })
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .rounding(Rounding {
                        nw: 12.0,
                        ne: 12.0,
                        sw: 0.0,
                        se: 0.0,
                    })
                    .shadow(egui::epaint::Shadow {
                        offset: Vec2::new(0.0, -3.0),
                        blur: 12.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(if pal.is_dark { 85 } else { 24 }),
                    })
                    .inner_margin(Margin::symmetric(14.0, 8.0))
                    .show(ui, |ui| {
                        let inner_w = (strip_w - 28.0).max(160.0);
                        ui.set_min_width(inner_w);
                        ui.set_max_width(inner_w);
                        ui.set_max_height(54.0);
                        ui.horizontal(|ui| {
                            let thumb_key = if !meta.thumb.is_empty() {
                                Some(format!("thumb_{}", crate::crypto::md5_hex(&meta.thumb)))
                            } else {
                                None
                            };
                            if let Some(tkey) = thumb_key.as_deref() {
                                if let Some(tex) = self.sticker_textures.get(tkey) {
                                    ui.add(
                                        egui::Image::new(tex)
                                            .fit_to_exact_size(Vec2::new(48.0, 48.0))
                                            .rounding(Rounding::same(7.0)),
                                    );
                                    ui.add_space(6.0);
                                }
                            }
                            let text_w = (inner_w - 94.0).max(100.0);
                            ui.vertical(|ui| {
                                ui.set_max_width(text_w);
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}",
                                        lucide::GLOBE,
                                        truncate_chars(&meta.title, 54)
                                    ))
                                    .size(12.5)
                                    .color(pal.primary_hover)
                                    .strong(),
                                );
                                let sub = if !meta.desc.is_empty() {
                                    truncate_chars(&meta.desc, 68).into_owned()
                                } else {
                                    clean_display_url(&meta.url, 48)
                                };
                                ui.label(
                                    RichText::new(sub)
                                        .size(11.3)
                                        .color(pal.text_secondary),
                                );
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .small_button(lucide::X)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Ẩn xem trước liên kết")
                                    .clicked()
                                {
                                    dismiss_clicked = true;
                                }
                            });
                        });
                    });
            });

        if dismiss_clicked {
            self.dismissed_link_preview_for = meta.url.clone();
        }
    }


    pub(super) fn render_dev_logs(&mut self, ui: &mut egui::Ui) {
        let entries = logger::get_entries();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(360.0);
                egui::ScrollArea::vertical()
                    .id_salt("dev_logs_list_scroll")
                    .show(ui, |ui| {
                        for e in entries.iter().rev() {
                            let st_str =
                                e.status.map(|s| s.to_string()).unwrap_or_else(|| "ERR".into());
                            let row = ui.selectable_label(
                                self.selected_log
                                    .as_ref()
                                    .map(|s| s.id == e.id)
                                    .unwrap_or(false),
                                format!(
                                    "[{}] {} {} ({}ms)",
                                    e.timestamp, e.method, st_str, e.latency_ms
                                ),
                            );
                            if row.clicked() {
                                self.selected_log = Some(e.clone());
                            }
                        }
                    });
            });

            ui.separator();

            ui.vertical(|ui| {
                if let Some(e) = &self.selected_log {
                    ui.strong(format!("{} {}", e.method, e.url));
                    ui.label(format!(
                        "Thời gian: {} · Độ trễ: {}ms · HTTP: {:?}",
                        e.timestamp, e.latency_ms, e.status
                    ));
                    if let Some(err) = &e.error_msg {
                        ui.colored_label(Color32::RED, format!("Lỗi: {err}"));
                    }
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .id_salt("dev_logs_detail_scroll")
                        .show(ui, |ui| {
                            if let Some(dec) = &e.decrypted_preview {
                                ui.label("Dữ liệu giải mã (Decrypted Payload):");
                                ui.code(dec);
                            } else {
                                ui.label("Payload:");
                                ui.code(&e.payload_preview);
                            }
                        });
                } else {
                    ui.label("Chọn một request bên trái để kiểm tra chi tiết.");
                }
            });
        });
    }

    pub(super) fn render_dev_scanner(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("dev_scanner_scroll")
            .show(ui, |ui| {
                ui.heading("Báo cáo quét chẩn đoán tự động:");
                ui.add_space(6.0);

                if self.scanner_findings.is_empty() {
                    ui.label("Bấm 'Làm mới quét' để chạy kiểm tra.");
                }

                for f in &self.scanner_findings {
                    let (badge_text, color) = match f.level {
                        FindingLevel::Info => ("INFO", Color32::from_rgb(40, 167, 69)),
                        FindingLevel::Warning => ("CẢNH BÁO", Color32::from_rgb(255, 193, 7)),
                        FindingLevel::Critical => ("NGHIÊM TRỌNG", Color32::RED),
                    };

                    egui::Frame::group(ui.style())
                        .stroke(Stroke::new(1.0_f32, color))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(color, format!("[{badge_text}]"));
                                ui.strong(&f.title);
                            });
                            ui.label(&f.detail);
                            ui.label(
                                RichText::new(format!("Khắc phục: {}", f.suggestion))
                                    .color(Color32::from_rgb(0, 104, 255)),
                            );
                        });
                    ui.add_space(6.0);
                }
            });
    }

    pub(super) fn render_dev_crypto(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("dev_crypto_scroll")
            .show(ui, |ui| {
                ui.heading("Thông số mật mã & Khung giao thức:");
                ui.add_space(8.0);

                egui::Grid::new("crypto_grid").striped(true).show(ui, |ui| {
                    ui.label("API Version (zpw_ver):");
                    ui.code(crate::session::ZPW_VER);
                    ui.end_row();

                    ui.label("API Type (zpw_type):");
                    ui.code(crate::session::ZPW_TYPE);
                    ui.end_row();

                    ui.label("ParamCipher Master Key:");
                    ui.code(format!("{:02X?}", crate::param_cipher::PARAM_CIPHER_KEY));
                    ui.end_row();

                    ui.label("Zero IV:");
                    ui.code("00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 (16 zero bytes)");
                    ui.end_row();

                    ui.label("Thuật toán mã hoá:");
                    ui.label("AES-128-CBC / AES-256-CBC with PKCS7 padding");
                    ui.end_row();

                    ui.label("Chữ ký SignKey:");
                    ui.code("md5('zsecure' + tag + sorted_values)");
                    ui.end_row();
                });
            });
    }
}
