use xilem::masonry::parley::style::FontStack;

pub const INTER_FONT_DATA: &[u8] = include_bytes!("../../resources/fonts/Inter.ttf");

pub const INTER: FontStack<'static> = FontStack::Source(std::borrow::Cow::Borrowed("Inter"));
