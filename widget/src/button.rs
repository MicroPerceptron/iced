//! Buttons allow your users to perform actions by pressing them.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } }
//! # pub type State = ();
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! use iced::widget::button;
//!
//! #[derive(Clone)]
//! enum Message {
//!     ButtonPressed,
//! }
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     button("Press me!").on_press(Message::ButtonPressed).into()
//! }
//! ```
use crate::core::animation::{Animation, Easing};
use crate::core::border::{self, Border, Radius};
use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::theme::palette;
use crate::core::time::{Duration, Instant};
use crate::core::touch;
use crate::core::widget::Operation;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Background, Color, Element, Event, Layout, Length, Padding, Rectangle, Shadow, Shell, Size,
    Theme, Vector, Widget,
};

/// A generic widget that produces a message when pressed.
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } }
/// # pub type State = ();
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// use iced::widget::button;
///
/// #[derive(Clone)]
/// enum Message {
///     ButtonPressed,
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     button("Press me!").on_press(Message::ButtonPressed).into()
/// }
/// ```
///
/// If a [`Button::on_press`] handler is not set, the resulting [`Button`] will
/// be disabled:
///
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } }
/// # pub type State = ();
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// use iced::widget::button;
///
/// #[derive(Clone)]
/// enum Message {
///     ButtonPressed,
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     button("I am disabled!").into()
/// }
/// ```
pub struct Button<'a, Message, Theme = crate::Theme, Renderer = crate::Renderer>
where
    Renderer: crate::core::Renderer,
    Theme: Catalog,
{
    content: Element<'a, Message, Theme, Renderer>,
    on_press: Option<OnPress<'a, Message>>,
    width: Length,
    height: Length,
    padding: Padding,
    clip: bool,
    class: Theme::Class<'a>,
}

enum OnPress<'a, Message> {
    Direct(Message),
    Closure(Box<dyn Fn() -> Message + 'a>),
}

impl<Message: Clone> OnPress<'_, Message> {
    fn get(&self) -> Message {
        match self {
            OnPress::Direct(message) => message.clone(),
            OnPress::Closure(f) => f(),
        }
    }
}

impl<'a, Message, Theme, Renderer> Button<'a, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
    Theme: Catalog,
{
    /// Creates a new [`Button`] with the given content.
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        let content = content.into();

        Button {
            content,
            on_press: None,
            width: Length::Fit,
            height: Length::Fit,
            padding: DEFAULT_PADDING,
            clip: false,
            class: Theme::default(),
        }
    }

    /// Sets the width of the [`Button`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Button`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the [`Padding`] of the [`Button`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed.
    ///
    /// Unless `on_press` is called, the [`Button`] will be disabled.
    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(OnPress::Direct(on_press));
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed.
    ///
    /// This is analogous to [`Button::on_press`], but using a closure to produce
    /// the message.
    ///
    /// This closure will only be called when the [`Button`] is actually pressed and,
    /// therefore, this method is useful to reduce overhead if creating the resulting
    /// message is slow.
    pub fn on_press_with(mut self, on_press: impl Fn() -> Message + 'a) -> Self {
        self.on_press = Some(OnPress::Closure(Box::new(on_press)));
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed,
    /// if `Some`.
    ///
    /// If `None`, the [`Button`] will be disabled.
    pub fn on_press_maybe(mut self, on_press: Option<Message>) -> Self {
        self.on_press = on_press.map(OnPress::Direct);
        self
    }

    /// Sets whether the contents of the [`Button`] should be clipped on
    /// overflow.
    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Sets the style of the [`Button`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Button`].
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }
}

#[derive(Debug, Clone)]
struct State {
    is_pressed: bool,
    hovered: Animation<bool>,
    pressed: Animation<bool>,
    now: Instant,
    initialized: bool,
}

impl Default for State {
    fn default() -> Self {
        let now = Instant::now();

        Self {
            is_pressed: false,
            hovered: control_animation(false),
            pressed: control_animation(false),
            now,
            initialized: false,
        }
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Button<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + crate::core::Renderer,
    Theme: Catalog,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));

        let size = self.content.as_widget().size();
        self.width = self.width.stack(size.width);
        self.height = self.height.stack(size.height);
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::padded(limits, self.width, self.height, self.padding, |limits| {
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits)
        })
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout.children().next().unwrap(),
                renderer,
                operation,
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            cursor,
            renderer,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. })
                if self.on_press.is_some() =>
            {
                let bounds = layout.bounds();

                if cursor.is_over(bounds) {
                    let state = tree.state.downcast_mut::<State>();

                    state.is_pressed = true;

                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. }) => {
                if let Some(on_press) = &self.on_press {
                    let state = tree.state.downcast_mut::<State>();

                    if state.is_pressed {
                        state.is_pressed = false;

                        let bounds = layout.bounds();

                        if cursor.is_over(bounds) {
                            shell.publish(on_press.get());
                        }

                        shell.capture_event();
                    }
                }
            }
            Event::Touch(touch::Event::FingerLost { .. }) => {
                let state = tree.state.downcast_mut::<State>();

                state.is_pressed = false;
            }
            _ => {}
        }

        let current_status = if self.on_press.is_none() {
            Status::Disabled
        } else if cursor.is_over(layout.bounds()) {
            let state = tree.state.downcast_ref::<State>();

            if state.is_pressed {
                Status::Pressed
            } else {
                Status::Hovered
            }
        } else {
            Status::Active
        };

        let now = match event {
            Event::Window(window::Event::RedrawRequested(now)) => *now,
            _ => Instant::now(),
        };
        let state = tree.state.downcast_mut::<State>();
        let hovered = matches!(current_status, Status::Hovered | Status::Pressed);
        let pressed = matches!(current_status, Status::Pressed);

        if state.initialized {
            if state.hovered.value() != hovered {
                state.hovered.go_mut(hovered, now);
            }
            if state.pressed.value() != pressed {
                state.pressed.go_mut(pressed, now);
            }
        } else {
            state.hovered = control_animation(hovered);
            state.pressed = control_animation(pressed);
            state.initialized = true;
        }
        state.now = now;

        if state.hovered.is_animating(now) || state.pressed.is_animating(now) {
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let content_layout = layout.children().next().unwrap();
        let state = tree.state.downcast_ref::<State>();
        let style = if self.on_press.is_none() {
            theme.style(&self.class, Status::Disabled)
        } else {
            let hover = state.hovered.interpolate(0.0, 1.0, state.now);
            let press = state.pressed.interpolate(0.0, 1.0, state.now);
            let active = theme.style(&self.class, Status::Active);
            let hovered = theme.style(&self.class, Status::Hovered);
            let pressed = theme.style(&self.class, Status::Pressed);

            interpolate_style(interpolate_style(active, hovered, hover), pressed, press)
        };
        let press_offset = if self.on_press.is_some() {
            state.pressed.interpolate(0.0, PRESS_TRANSLATE, state.now)
        } else {
            0.0
        };

        renderer.with_translation(Vector::new(press_offset, press_offset), |renderer| {
            draw(
                renderer,
                &style,
                bounds,
                viewport,
                self.clip,
                |renderer, viewport| {
                    self.content.as_widget().draw(
                        &tree.children[0],
                        renderer,
                        theme,
                        &renderer::Style {
                            text_color: style.text_color,
                        },
                        content_layout,
                        cursor,
                        viewport,
                    );
                },
            );
        });
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let is_mouse_over = cursor.is_over(layout.bounds());

        if is_mouse_over && self.on_press.is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
            viewport,
            translation,
        )
    }
}

fn draw<Renderer>(
    renderer: &mut Renderer,
    style: &Style,
    bounds: Rectangle,
    viewport: &Rectangle,
    clip: bool,
    draw_content: impl FnOnce(&mut Renderer, &Rectangle),
) where
    Renderer: crate::core::Renderer,
{
    if style.background.is_some() || style.border.width > 0.0 || style.shadow.color.a > 0.0 {
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: style.border,
                shadow: style.shadow,
                snap: style.snap,
            },
            style
                .background
                .unwrap_or(Background::Color(Color::TRANSPARENT)),
        );
    }

    let viewport = if clip {
        bounds.intersection(viewport).unwrap_or(*viewport)
    } else {
        *viewport
    };

    draw_content(renderer, &viewport);
}

const CONTROL_DURATION: Duration = Duration::from_millis(150);
const PRESS_TRANSLATE: f32 = 1.0;

fn control_animation(value: bool) -> Animation<bool> {
    Animation::new(value)
        .duration(CONTROL_DURATION)
        .easing(Easing::Custom(control_ease_out))
}

// CSS cubic-bezier(0.2, 0, 0.2, 1), evaluated by inverting x(t).
fn control_ease_out(x: f32) -> f32 {
    let mut low = 0.0;
    let mut high = 1.0;

    for _ in 0..12 {
        let t = (low + high) * 0.5;
        let inverse = 1.0 - t;
        let curve_x = 3.0 * inverse * inverse * t * 0.2 + 3.0 * inverse * t * t * 0.2 + t * t * t;

        if curve_x < x {
            low = t;
        } else {
            high = t;
        }
    }

    let t = (low + high) * 0.5;
    t * t * (3.0 - 2.0 * t)
}

fn interpolate_style(from: Style, to: Style, amount: f32) -> Style {
    let amount = amount.clamp(0.0, 1.0);

    Style {
        background: interpolate_background(from.background, to.background, amount),
        text_color: from.text_color.mix(to.text_color, amount),
        border: Border {
            color: from.border.color.mix(to.border.color, amount),
            width: lerp(from.border.width, to.border.width, amount),
            radius: interpolate_radius(from.border.radius, to.border.radius, amount),
        },
        shadow: Shadow {
            color: from.shadow.color.mix(to.shadow.color, amount),
            offset: Vector::new(
                lerp(from.shadow.offset.x, to.shadow.offset.x, amount),
                lerp(from.shadow.offset.y, to.shadow.offset.y, amount),
            ),
            blur_radius: lerp(from.shadow.blur_radius, to.shadow.blur_radius, amount),
        },
        snap: if amount < 0.5 { from.snap } else { to.snap },
    }
}

fn interpolate_background(
    from: Option<Background>,
    to: Option<Background>,
    amount: f32,
) -> Option<Background> {
    match (from, to) {
        (Some(Background::Color(from)), Some(Background::Color(to))) => {
            Some(Background::Color(from.mix(to, amount)))
        }
        (None, Some(Background::Color(to))) => {
            Some(Background::Color(Color { a: 0.0, ..to }.mix(to, amount)))
        }
        (Some(Background::Color(from)), None) => Some(Background::Color(
            from.mix(Color { a: 0.0, ..from }, amount),
        )),
        (from, to) => {
            if amount < 0.5 {
                from
            } else {
                to
            }
        }
    }
}

fn interpolate_radius(from: Radius, to: Radius, amount: f32) -> Radius {
    Radius {
        top_left: lerp(from.top_left, to.top_left, amount),
        top_right: lerp(from.top_right, to.top_right, amount),
        bottom_right: lerp(from.bottom_right, to.bottom_right, amount),
        bottom_left: lerp(from.bottom_left, to.bottom_left, amount),
    }
}

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

impl<'a, Message, Theme, Renderer> From<Button<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: crate::core::Renderer + 'a,
{
    fn from(button: Button<'a, Message, Theme, Renderer>) -> Self {
        Self::new(button)
    }
}

/// The default [`Padding`] of a [`Button`].
pub const DEFAULT_PADDING: Padding = Padding {
    top: 5.0,
    bottom: 5.0,
    right: 10.0,
    left: 10.0,
};

/// The possible status of a [`Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The [`Button`] can be pressed.
    Active,
    /// The [`Button`] can be pressed and it is being hovered.
    Hovered,
    /// The [`Button`] is being pressed.
    Pressed,
    /// The [`Button`] cannot be pressed.
    Disabled,
}

/// The style of a button.
///
/// If not specified with [`Button::style`]
/// the theme will provide the style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Background`] of the button.
    pub background: Option<Background>,
    /// The text [`Color`] of the button.
    pub text_color: Color,
    /// The [`Border`] of the button.
    pub border: Border,
    /// The [`Shadow`] of the button.
    pub shadow: Shadow,
    /// Whether the button should be snapped to the pixel grid.
    pub snap: bool,
}

impl Style {
    /// Updates the [`Style`] with the given [`Background`].
    pub fn with_background(self, background: impl Into<Background>) -> Self {
        Self {
            background: Some(background.into()),
            ..self
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: None,
            text_color: Color::BLACK,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: renderer::CRISP,
        }
    }
}

/// The theme catalog of a [`Button`].
///
/// All themes that can be used with [`Button`]
/// must implement this trait.
///
/// # Example
/// ```no_run
/// # use iced_widget::core::{Color, Background};
/// # use iced_widget::button::{Catalog, Status, Style};
/// # struct MyTheme;
/// #[derive(Debug, Default)]
/// pub enum ButtonClass {
///     #[default]
///     Primary,
///     Secondary,
///     Danger
/// }
///
/// impl Catalog for MyTheme {
///     type Class<'a> = ButtonClass;
///     
///     fn default<'a>() -> Self::Class<'a> {
///         ButtonClass::default()
///     }
///     
///
///     fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
///         let mut style = Style::default();
///
///         match class {
///             ButtonClass::Primary => {
///                 style.background = Some(Background::Color(Color::from_rgb(0.529, 0.808, 0.921)));
///             },
///             ButtonClass::Secondary => {
///                 style.background = Some(Background::Color(Color::WHITE));
///             },
///             ButtonClass::Danger => {
///                 style.background = Some(Background::Color(Color::from_rgb(0.941, 0.502, 0.502)));
///             },
///         }
///
///         style
///     }
/// }
/// ```
///
/// Although, in order to use [`Button::style`]
/// with `MyTheme`, [`Catalog::Class`] must implement
/// `From<StyleFn<'_, MyTheme>>`.
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

/// A styling function for a [`Button`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(primary)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// A primary button; denoting a main action.
pub fn primary(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.primary.base);

    match status {
        Status::Active | Status::Pressed => base,
        Status::Hovered => Style {
            background: Some(Background::Color(palette.primary.strong.color)),
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A secondary button; denoting a complementary action.
pub fn secondary(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.secondary.base);

    match status {
        Status::Active | Status::Pressed => base,
        Status::Hovered => Style {
            background: Some(Background::Color(palette.secondary.strong.color)),
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A success button; denoting a good outcome.
pub fn success(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.success.base);

    match status {
        Status::Active | Status::Pressed => base,
        Status::Hovered => Style {
            background: Some(Background::Color(palette.success.strong.color)),
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A warning button; denoting a risky action.
pub fn warning(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.warning.base);

    match status {
        Status::Active | Status::Pressed => base,
        Status::Hovered => Style {
            background: Some(Background::Color(palette.warning.strong.color)),
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A danger button; denoting a destructive action.
pub fn danger(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.danger.base);

    match status {
        Status::Active | Status::Pressed => base,
        Status::Hovered => Style {
            background: Some(Background::Color(palette.danger.strong.color)),
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A text button; useful for links.
pub fn text(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    let base = Style {
        text_color: palette.background.base.text,
        ..Style::default()
    };

    match status {
        Status::Active | Status::Pressed => base,
        Status::Hovered => Style {
            text_color: palette.background.base.text.scale_alpha(0.8),
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A button using background shades.
pub fn background(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.background.base);

    match status {
        Status::Active => base,
        Status::Pressed => Style {
            background: Some(Background::Color(palette.background.strong.color)),
            ..base
        },
        Status::Hovered => Style {
            background: Some(Background::Color(palette.background.weak.color)),
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A subtle button using weak background shades.
pub fn subtle(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.background.weakest);

    match status {
        Status::Active => base,
        Status::Pressed => Style {
            background: Some(Background::Color(palette.background.strong.color)),
            ..base
        },
        Status::Hovered => Style {
            background: Some(Background::Color(palette.background.weaker.color)),
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

fn styled(pair: palette::Pair) -> Style {
    Style {
        background: Some(Background::Color(pair.color)),
        text_color: pair.text,
        border: border::rounded(2),
        ..Style::default()
    }
}

fn disabled(style: Style) -> Style {
    Style {
        background: style
            .background
            .map(|background| background.scale_alpha(0.5)),
        text_color: style.text_color.scale_alpha(0.5),
        ..style
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_easing_is_an_ease_out_curve() {
        assert!(control_ease_out(0.0) < 0.001);
        assert!(control_ease_out(0.5) > 0.5);
        assert!((control_ease_out(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn controls_animate_for_the_design_system_duration() {
        let started = Instant::now();
        let mut animation = control_animation(false);
        animation.go_mut(true, started);

        let middle = animation.interpolate(0.0, 1.0, started + Duration::from_millis(75));
        assert!(middle > 0.5 && middle < 1.0);
        assert!(animation.is_animating(started + Duration::from_millis(149)));
        assert!(!animation.is_animating(started + CONTROL_DURATION));
        assert_eq!(PRESS_TRANSLATE, 1.0);
    }

    #[test]
    fn control_styles_interpolate_colors_and_geometry() {
        let from = Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: Color::BLACK,
            border: Border {
                color: Color::BLACK,
                width: 0.0,
                radius: 2.0.into(),
            },
            ..Style::default()
        };
        let to = Style {
            background: Some(Background::Color(Color::WHITE)),
            text_color: Color::WHITE,
            border: Border {
                color: Color::WHITE,
                width: 2.0,
                radius: 6.0.into(),
            },
            ..Style::default()
        };

        let middle = interpolate_style(from, to, 0.5);
        let Some(Background::Color(background)) = middle.background else {
            panic!("solid control backgrounds remain solid while interpolating");
        };

        assert!(background.r > 0.0 && background.r < 1.0);
        assert_eq!(background.a, 0.5);
        assert!(middle.text_color.r > 0.0 && middle.text_color.r < 1.0);
        assert_eq!(middle.text_color.a, 1.0);
        assert_eq!(middle.border.width, 1.0);
        assert_eq!(middle.border.radius, 4.0.into());
    }
}
