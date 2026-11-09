use super::*;

fn rgb(hex: u32) -> Color {
    Color((hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255)
}

// Supply both Sapid's typed styles and its CSS interaction tokens from one palette.
fn orange() -> sapid::Theme {
    let mut theme = sapid::Theme::blue_light();
    theme.name = "colang".into();
    theme.canvas = rgb(0xfffbf7);
    theme.foreground = rgb(0x292524);
    theme.muted = rgb(0x78716c);
    theme.border = rgb(0xeadfd5);
    theme.accent = rgb(0xc2410c);
    theme.accent_hover = rgb(0x9a3412);
    theme.accent_soft = rgb(0xffedd5);
    theme.accent_text = rgb(0x9a3412);
    theme.focus_ring = rgb(0xc2410c);
    theme.input_focus = rgb(0xc2410c);
    theme.disabled = rgb(0xf5f5f4);
    theme.disabled_text = rgb(0xa8a29e);
    theme.danger = rgb(0xb91c1c);
    theme.danger_fill = rgb(0xb91c1c);
    theme.danger_soft = rgb(0xfef2f2);
    theme.success = rgb(0x166534);
    theme.success_soft = rgb(0xf0fdf4);
    for (role, size, line_height, weight) in [
        (sapid::TypographyRole::Control, 16., 24., Some(500.)),
        (sapid::TypographyRole::Label, 15., 23., Some(500.)),
        (sapid::TypographyRole::Body, 16., 27., None),
        (sapid::TypographyRole::Description, 14., 22., None),
    ] {
        theme.tokens = theme
            .tokens
            .with_typography(
                role,
                sapid::Size::Medium,
                sapid::TypographyMetrics {
                    font_size: size,
                    line_height,
                    weight,
                },
            )
            .expect("valid application typography");
    }
    theme.shades = [
        0xfff7ed, 0xffedd5, 0xfed7aa, 0xfdba74, 0xfb923c, 0xf97316, 0xea580c, 0xc2410c, 0x9a3412,
        0x7c2d12,
    ]
    .map(rgb);
    theme.series[0] = theme.accent;
    theme
}

pub fn action_button() -> sapid::Button {
    sapid::button().theme(orange())
}
pub fn danger_button() -> sapid::Button {
    action_button().variant(sapid::ButtonVariant::Danger)
}

pub fn text_control() -> sapid::TextField {
    sapid::text_field().theme(orange())
}

pub fn checkbox_control() -> sapid::Checkbox {
    sapid::checkbox().theme(orange())
}

pub fn install() -> std::result::Result<(), JsValue> {
    let document = document();
    let stylesheet = orange()
        .stylesheet()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let style = document.create_element("style")?;
    style.set_id("colang-theme");
    style.set_text_content(Some(&stylesheet));
    document
        .query_selector("head")?
        .ok_or("missing document head")?
        .append_child(&style)?;
    Ok(())
}
