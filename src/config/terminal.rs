//! Bridge our gui config into the terminal crate configuration

use crate::config::configuration;
use termwiz::hyperlink::Rule as HyperlinkRule;
use wezterm_term::color::ColorPalette;

#[derive(Debug)]
pub struct TermConfig;

impl wezterm_term::TerminalConfiguration for TermConfig {
    fn generation(&self) -> usize {
        configuration().generation()
    }

    fn scrollback_size(&self) -> usize {
        configuration().scrollback_lines
    }

    fn hyperlink_rules(&self) -> (usize, Vec<HyperlinkRule>) {
        let config = configuration();
        (config.generation(), config.hyperlink_rules.clone())
    }

    fn enable_csi_u_key_encoding(&self) -> bool {
        configuration().enable_csi_u_key_encoding
    }

    fn color_palette(&self) -> ColorPalette {
        let config = configuration();

        if let Some(scheme_name) = config.color_scheme.as_ref() {
            if let Some(palette) = config.color_schemes.get(scheme_name) {
                return palette.clone().into();
            }
        }

        config
            .colors
            .as_ref()
            .cloned()
            .map(Into::into)
            .unwrap_or_else(ColorPalette::default)
    }
}
