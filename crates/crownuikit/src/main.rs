use crownuikit::widgets::{context_menu::preview_menu, slider};
use winit::error::EventLoopError;
use xilem::masonry::properties::Padding;
use xilem::masonry::properties::types::AsUnit;
use xilem::style::Style;
use xilem::view::{CrossAxisAlignment, FlexSpacer, flex_col, flex_row, label, sized_box};
use xilem::{Color, EventLoop, WidgetView, WindowOptions, Xilem};

struct AppState {
    slider_a: f64,
    slider_b: f64,
    slider_c: f64,
}

fn app_logic(data: &mut AppState) -> impl WidgetView<AppState> + use<> {
    let sliders = flex_col((
        slider(0.0, 100.0, data.slider_a, |s: &mut AppState, v| s.slider_a = v),
        slider(0.0, 100.0, data.slider_b, |s: &mut AppState, v| s.slider_b = v),
        slider(0.0, 100.0, data.slider_c, |s: &mut AppState, v| s.slider_c = v),
    ))
    .gap(16.0.px())
    .cross_axis_alignment(CrossAxisAlignment::Center);

    let slider_card = sized_box(sliders)
        .padding(Padding::from_vh(24.0, 20.0))
        .background_color(Color::from_rgb8(0xF5, 0xF5, 0xF7))
        .corner_radius(16.0)
        .border(Color::from_rgb8(0xE4, 0xE4, 0xE7), 1.0);

    let showcase = flex_row((
        flex_col((label("Sliders"), slider_card))
            .gap(12.0.px())
            .cross_axis_alignment(CrossAxisAlignment::Start),
        FlexSpacer::Fixed(40.0.px()),
        flex_col((label("Context Menu"), preview_menu::<AppState, ()>()))
            .gap(12.0.px())
            .cross_axis_alignment(CrossAxisAlignment::Start),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(0.0.px());

    sized_box(showcase).padding(Padding::all(32.0))
}

fn main() -> Result<(), EventLoopError> {
    let state = AppState {
        slider_a: 20.0,
        slider_b: 50.0,
        slider_c: 90.0,
    };
    let app = Xilem::new_simple(state, app_logic, WindowOptions::new("crownuikit preview"));
    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}
