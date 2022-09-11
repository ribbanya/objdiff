use egui::{
    text::LayoutJob, CollapsingHeader, Color32, FontFamily, FontId, Rgba, ScrollArea,
    SelectableLabel, TextFormat, Ui, Widget,
};
use egui_extras::{Size, StripBuilder};

use crate::{
    app::{View, ViewState},
    jobs::build::BuildStatus,
    obj::{ObjInfo, ObjSymbol, ObjSymbolFlags},
};

pub fn match_color_for_symbol(symbol: &ObjSymbol) -> Color32 {
    if symbol.match_percent == 100.0 {
        Color32::GREEN
    } else if symbol.match_percent >= 50.0 {
        Color32::LIGHT_BLUE
    } else {
        Color32::RED
    }
}

fn symbol_ui(
    ui: &mut Ui,
    symbol: &ObjSymbol,
    highlighted_symbol: &mut Option<String>,
    selected_symbol: &mut Option<String>,
    current_view: &mut View,
) {
    let mut job = LayoutJob::default();
    let name: &str =
        if let Some(demangled) = &symbol.demangled_name { demangled } else { &symbol.name };
    let mut selected = false;
    if let Some(sym) = highlighted_symbol {
        selected = sym == &symbol.name;
    }
    let font_id = FontId::new(14.0, FontFamily::Monospace);
    job.append("[", 0.0, TextFormat {
        font_id: font_id.clone(),
        color: Color32::GRAY,
        ..Default::default()
    });
    if symbol.flags.0.contains(ObjSymbolFlags::Common) {
        job.append("c", 0.0, TextFormat {
            font_id: font_id.clone(),
            color: Color32::from_rgb(0, 255, 255),
            ..Default::default()
        });
    } else if symbol.flags.0.contains(ObjSymbolFlags::Global) {
        job.append("g", 0.0, TextFormat {
            font_id: font_id.clone(),
            color: Color32::GREEN,
            ..Default::default()
        });
    } else if symbol.flags.0.contains(ObjSymbolFlags::Local) {
        job.append("l", 0.0, TextFormat {
            font_id: font_id.clone(),
            color: Color32::GRAY,
            ..Default::default()
        });
    }
    if symbol.flags.0.contains(ObjSymbolFlags::Weak) {
        job.append("w", 0.0, TextFormat {
            font_id: font_id.clone(),
            color: Color32::GRAY,
            ..Default::default()
        });
    }
    job.append("] ", 0.0, TextFormat {
        font_id: font_id.clone(),
        color: Color32::GRAY,
        ..Default::default()
    });
    if symbol.match_percent > 0.0 {
        job.append("(", 0.0, TextFormat {
            font_id: font_id.clone(),
            color: Color32::GRAY,
            ..Default::default()
        });
        job.append(&format!("{:.0}%", symbol.match_percent), 0.0, TextFormat {
            font_id: font_id.clone(),
            color: match_color_for_symbol(symbol),
            ..Default::default()
        });
        job.append(") ", 0.0, TextFormat {
            font_id: font_id.clone(),
            color: Color32::GRAY,
            ..Default::default()
        });
    }
    job.append(name, 0.0, TextFormat { font_id, color: Color32::WHITE, ..Default::default() });
    let response = SelectableLabel::new(selected, job).ui(ui);
    if response.clicked() {
        *selected_symbol = Some(symbol.name.clone());
        *current_view = View::FunctionDiff;
    } else if response.hovered() {
        *highlighted_symbol = Some(symbol.name.clone());
    }
}

fn symbol_list_ui(
    ui: &mut Ui,
    obj: &ObjInfo,
    highlighted_symbol: &mut Option<String>,
    selected_symbol: &mut Option<String>,
    current_view: &mut View,
    reverse_function_order: bool,
) {
    ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        ui.scope(|ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.style_mut().wrap = Some(false);

            if !obj.common.is_empty() {
                CollapsingHeader::new(".comm").default_open(true).show(ui, |ui| {
                    for symbol in &obj.common {
                        symbol_ui(ui, symbol, highlighted_symbol, selected_symbol, current_view);
                    }
                });
            }

            for section in &obj.sections {
                CollapsingHeader::new(format!("{} ({:x})", section.name, section.size))
                    .default_open(true)
                    .show(ui, |ui| {
                        if section.name == ".text" && reverse_function_order {
                            for symbol in section.symbols.iter().rev() {
                                symbol_ui(
                                    ui,
                                    symbol,
                                    highlighted_symbol,
                                    selected_symbol,
                                    current_view,
                                );
                            }
                        } else {
                            for symbol in &section.symbols {
                                symbol_ui(
                                    ui,
                                    symbol,
                                    highlighted_symbol,
                                    selected_symbol,
                                    current_view,
                                );
                            }
                        }
                    });
            }
        });
    });
}

fn build_log_ui(ui: &mut Ui, status: &BuildStatus) {
    ui.scope(|ui| {
        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
        ui.style_mut().wrap = Some(false);
        ui.colored_label(Color32::from_rgb(255, 0, 0), &status.log);
    });
}

pub fn symbol_diff_ui(ui: &mut Ui, view_state: &mut ViewState) {
    if let (Some(result), highlighted_symbol, selected_symbol, current_view) = (
        &view_state.build,
        &mut view_state.highlighted_symbol,
        &mut view_state.selected_symbol,
        &mut view_state.current_view,
    ) {
        StripBuilder::new(ui).size(Size::exact(40.0)).size(Size::remainder()).vertical(
            |mut strip| {
                strip.strip(|builder| {
                    builder.sizes(Size::remainder(), 2).horizontal(|mut strip| {
                        strip.cell(|ui| {
                            ui.scope(|ui| {
                                ui.style_mut().override_text_style =
                                    Some(egui::TextStyle::Monospace);
                                ui.style_mut().wrap = Some(false);

                                ui.label("Build asm:");
                                if result.first_status.success {
                                    ui.label("OK");
                                } else {
                                    ui.colored_label(Rgba::from_rgb(1.0, 0.0, 0.0), "Fail");
                                }
                            });
                            ui.separator();
                        });
                        strip.cell(|ui| {
                            ui.scope(|ui| {
                                ui.style_mut().override_text_style =
                                    Some(egui::TextStyle::Monospace);
                                ui.style_mut().wrap = Some(false);

                                ui.label("Build src:");
                                if result.second_status.success {
                                    ui.label("OK");
                                } else {
                                    ui.colored_label(Rgba::from_rgb(1.0, 0.0, 0.0), "Fail");
                                }
                            });
                            ui.separator();
                        });
                    });
                });
                strip.strip(|builder| {
                    builder.sizes(Size::remainder(), 2).horizontal(|mut strip| {
                        strip.cell(|ui| {
                            if result.first_status.success {
                                if let Some(obj) = &result.first_obj {
                                    ui.push_id("left", |ui| {
                                        symbol_list_ui(
                                            ui,
                                            obj,
                                            highlighted_symbol,
                                            selected_symbol,
                                            current_view,
                                            view_state.reverse_fn_order,
                                        );
                                    });
                                }
                            } else {
                                build_log_ui(ui, &result.first_status);
                            }
                        });
                        strip.cell(|ui| {
                            if result.second_status.success {
                                if let Some(obj) = &result.second_obj {
                                    ui.push_id("right", |ui| {
                                        symbol_list_ui(
                                            ui,
                                            obj,
                                            highlighted_symbol,
                                            selected_symbol,
                                            current_view,
                                            view_state.reverse_fn_order,
                                        );
                                    });
                                }
                            } else {
                                build_log_ui(ui, &result.second_status);
                            }
                        });
                    });
                });
            },
        );
    }
}
