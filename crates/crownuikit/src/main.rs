use blinc_icons::icons;
use crownuikit::layouts::sidebar::{
    sidebar, sidebar_brand, sidebar_group, sidebar_item, sidebar_separator, sidebar_subitem,
};
use crownuikit::util::{INTER, INTER_FONT_DATA};
use crownuikit::widgets::{select, slider, toggle};
use winit::error::EventLoopError;
use xilem::masonry::properties::Padding;
use xilem::masonry::properties::types::AsUnit;
use xilem::style::Style;
use xilem::view::{flex_col, flex_row, label, sized_box, CrossAxisAlignment, FlexSpacer};
use xilem::{Color, EventLoop, WidgetView, WindowOptions, Xilem};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Nav {
    Dashboard,
    Orders,
    Customers,
    LocalCurrency,
    ForeignCurrency,
    Beneficiaries,
    Transactions,
    Wallet,
    Transfers,
    Cards,
    Account,
    Billing,
    Upgrade,
    Docs,
    Support,
    Settings,
}

struct AppState {
    // Sidebar
    nav: Nav,
    bank_accounts_open: bool,
    // Existing showcase state
    slider_a: f64,
    slider_b: f64,
    slider_c: f64,
    toggle_a: bool,
    toggle_b: bool,
    fruit: Option<usize>,
}

const FRUITS: &[&str] = &[
    "Select a fruit",
    "Apple",
    "Banana",
    "Blueberry",
    "Grapes",
    "Pineapple",
];

fn sidebar_view(data: &mut AppState) -> impl WidgetView<AppState> + use<> {
    // Bind the current selection to a local so the closures below can each
    // capture a copy without lifetime gymnastics.
    let nav = data.nav;

    let content = flex_col((
        sidebar_brand("Untitled UI", icons::INFINITY, |_s: &mut AppState| ()),
        // Top nav group ---
        flex_col((
            sidebar_item("Dashboard", icons::HOUSE, nav == Nav::Dashboard, |s: &mut AppState| {
                s.nav = Nav::Dashboard
            }),
            sidebar_item("Orders", icons::SHOPPING_BAG, nav == Nav::Orders, |s: &mut AppState| {
                s.nav = Nav::Orders
            }),
            sidebar_item(
                "Customers",
                icons::USERS,
                nav == Nav::Customers,
                |s: &mut AppState| s.nav = Nav::Customers,
            ),
        ))
        .gap(2.0.px()),
        sidebar_separator(),
        // Bank accounts (collapsible) ---
        sidebar_group(
            "Bank accounts",
            icons::LANDMARK,
            data.bank_accounts_open,
            |s: &mut AppState| s.bank_accounts_open = !s.bank_accounts_open,
            flex_col((
                sidebar_subitem(
                    "Local currency",
                    nav == Nav::LocalCurrency,
                    |s: &mut AppState| s.nav = Nav::LocalCurrency,
                ),
                sidebar_subitem(
                    "Foreign currency",
                    nav == Nav::ForeignCurrency,
                    |s: &mut AppState| s.nav = Nav::ForeignCurrency,
                ),
                sidebar_subitem(
                    "Beneficiaries",
                    nav == Nav::Beneficiaries,
                    |s: &mut AppState| s.nav = Nav::Beneficiaries,
                ),
                sidebar_subitem(
                    "Transactions",
                    nav == Nav::Transactions,
                    |s: &mut AppState| s.nav = Nav::Transactions,
                ),
            ))
            .gap(2.0.px()),
        ),
        // Middle nav group ---
        flex_col((
            sidebar_item("Wallet", icons::WALLET, nav == Nav::Wallet, |s: &mut AppState| {
                s.nav = Nav::Wallet
            }),
            sidebar_item(
                "Transfers",
                icons::ARROW_LEFT_RIGHT,
                nav == Nav::Transfers,
                |s: &mut AppState| s.nav = Nav::Transfers,
            ),
            sidebar_item("Cards", icons::CREDIT_CARD, nav == Nav::Cards, |s: &mut AppState| {
                s.nav = Nav::Cards
            }),
        ))
        .gap(2.0.px()),
        sidebar_separator(),
        // Account group ---
        flex_col((
            sidebar_item(
                "My account",
                icons::CIRCLE_USER,
                nav == Nav::Account,
                |s: &mut AppState| s.nav = Nav::Account,
            ),
            sidebar_item(
                "Plans & billing",
                icons::RECEIPT,
                nav == Nav::Billing,
                |s: &mut AppState| s.nav = Nav::Billing,
            ),
            sidebar_item("Upgrade to PRO", icons::ZAP, nav == Nav::Upgrade, |s: &mut AppState| {
                s.nav = Nav::Upgrade
            }),
            sidebar_item(
                "Documentation",
                icons::BOOK_OPEN,
                nav == Nav::Docs,
                |s: &mut AppState| s.nav = Nav::Docs,
            ),
        ))
        .gap(2.0.px()),
        FlexSpacer::Flex(1.0),
        // Bottom items ---
        flex_col((
            sidebar_item(
                "Support",
                icons::LIFE_BUOY,
                nav == Nav::Support,
                |s: &mut AppState| s.nav = Nav::Support,
            ),
            sidebar_item("Settings", icons::SETTINGS, nav == Nav::Settings, |s: &mut AppState| {
                s.nav = Nav::Settings
            }),
        ))
        .gap(2.0.px()),
    ))
    .gap(6.0.px())
    .cross_axis_alignment(CrossAxisAlignment::Start);

    sidebar(content)
}

fn widget_showcase(data: &mut AppState) -> impl WidgetView<AppState> + use<> {
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

    let toggles = flex_col((
        toggle("Wi-Fi", data.toggle_a, |s: &mut AppState, v| s.toggle_a = v),
        toggle("Notifications", data.toggle_b, |s: &mut AppState, v| {
            s.toggle_b = v
        }),
    ))
    .gap(12.0.px())
    .cross_axis_alignment(CrossAxisAlignment::Start);

    let toggle_card = sized_box(toggles)
        .padding(Padding::from_vh(20.0, 20.0))
        .background_color(Color::from_rgb8(0xF5, 0xF5, 0xF7))
        .corner_radius(16.0)
        .border(Color::from_rgb8(0xE4, 0xE4, 0xE7), 1.0);

    let fruit_select = select(
        Some("Fruits"),
        FRUITS.iter().copied(),
        data.fruit,
        |s: &mut AppState, i| s.fruit = Some(i),
    );
    let select_card = sized_box(fruit_select)
        .padding(Padding::from_vh(20.0, 20.0))
        .background_color(Color::from_rgb8(0x0A, 0x0A, 0x0A))
        .corner_radius(16.0)
        .border(Color::from_rgb8(0x27, 0x27, 0x2A), 1.0);

    let showcase = flex_col((
        label("Widget preview").font(INTER),
        flex_col((
            flex_col((label("Sliders").font(INTER), slider_card))
                .gap(12.0.px())
                .cross_axis_alignment(CrossAxisAlignment::Start),
            flex_col((label("Toggles").font(INTER), toggle_card))
                .gap(12.0.px())
                .cross_axis_alignment(CrossAxisAlignment::Start),
            flex_col((label("Select").font(INTER), select_card))
                .gap(12.0.px())
                .cross_axis_alignment(CrossAxisAlignment::Start),
        ))
        .gap(24.0.px())
        .cross_axis_alignment(CrossAxisAlignment::Start),
    ))
    .gap(16.0.px())
    .cross_axis_alignment(CrossAxisAlignment::Start);

    sized_box(showcase).padding(Padding::all(32.0))
}

fn app_logic(data: &mut AppState) -> impl WidgetView<AppState> + use<> {
    flex_row((sidebar_view(data), widget_showcase(data)))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(0.0.px())
}

fn main() -> Result<(), EventLoopError> {
    let state = AppState {
        nav: Nav::Dashboard,
        bank_accounts_open: true,
        slider_a: 20.0,
        slider_b: 50.0,
        slider_c: 90.0,
        toggle_a: false,
        toggle_b: true,
        fruit: Some(3),
    };
    let app = Xilem::new_simple(state, app_logic, WindowOptions::new("crownuikit preview"))
        .with_font(INTER_FONT_DATA.to_vec());
    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}
