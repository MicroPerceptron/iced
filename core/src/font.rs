//! Load and use fonts.
use std::hash::Hash;

/// Extra space between glyphs, as a fraction of the type size.
///
/// Tracking is expressed relative to the size rather than in pixels, so a
/// heading and a caption asking for the same tracking stay proportionate to
/// one another. A design that says `0.06em` means [`Tracking(0.06)`].
///
/// [`Tracking(0.06)`]: Tracking
#[derive(Debug, Clone, Copy, Default)]
pub struct Tracking(pub f32);

impl Tracking {
    /// No extra space. Glyphs sit at their natural advance.
    pub const NONE: Self = Self(0.0);

    /// Whether this tracking would change anything.
    pub fn is_none(self) -> bool {
        self.0 == 0.0
    }
}

// `Font` is a hash key throughout the text pipeline, so tracking has to be
// comparable and hashable by value. Compare and hash the bits, canonicalizing
// the two values that would otherwise break the `Eq`/`Hash` agreement: NaN is
// never equal to itself, and `-0.0 == 0.0` while their bits differ.
impl PartialEq for Tracking {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_nan() {
            other.0.is_nan()
        } else {
            self.0 == other.0
        }
    }
}

impl Eq for Tracking {}

impl std::hash::Hash for Tracking {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        const CANONICAL_NAN: u32 = 0x7fc0_0000;

        let bits = if self.0.is_nan() {
            CANONICAL_NAN
        } else {
            (self.0 + 0.0).to_bits()
        };
        bits.hash(hasher);
    }
}

/// A font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Font {
    /// The [`Family`] of the [`Font`].
    pub family: Family,
    /// The [`Weight`] of the [`Font`].
    pub weight: Weight,
    /// The [`Stretch`] of the [`Font`].
    pub stretch: Stretch,
    /// The [`Style`] of the [`Font`].
    pub style: Style,
    /// The [`Tracking`] of the [`Font`].
    pub tracking: Tracking,
}

impl Font {
    /// A non-monospaced sans-serif font with normal [`Weight`].
    pub const DEFAULT: Font = Font {
        family: Family::SansSerif,
        weight: Weight::Normal,
        stretch: Stretch::Normal,
        style: Style::Normal,
        tracking: Tracking::NONE,
    };

    /// A monospaced font with normal [`Weight`].
    pub const MONOSPACE: Font = Font {
        family: Family::Monospace,
        ..Self::DEFAULT
    };

    /// Creates a [`Font`] with the given [`Family::Name`] and default attributes.
    pub const fn new(name: &'static str) -> Self {
        Self {
            family: Family::Name(name),
            ..Self::DEFAULT
        }
    }

    /// Creates a [`Font`] with the given [`Family`] and default attributes.
    pub fn with_family(family: impl Into<Family>) -> Self {
        Font {
            family: family.into(),
            ..Self::DEFAULT
        }
    }

    /// Sets the [`Weight`] of the [`Font`].
    pub const fn weight(self, weight: Weight) -> Self {
        Self { weight, ..self }
    }

    /// Sets the [`Stretch`] of the [`Font`].
    pub const fn stretch(self, stretch: Stretch) -> Self {
        Self { stretch, ..self }
    }

    /// Sets the [`Tracking`] of the [`Font`].
    pub const fn tracking(self, tracking: Tracking) -> Self {
        Self { tracking, ..self }
    }

    /// Sets the [`Style`] of the [`Font`].
    pub const fn style(self, style: Style) -> Self {
        Self { style, ..self }
    }
}

impl From<&'static str> for Font {
    fn from(name: &'static str) -> Self {
        Font::new(name)
    }
}

impl From<Family> for Font {
    fn from(family: Family) -> Self {
        Font::with_family(family)
    }
}

/// A font family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Family {
    /// The name of a font family of choice.
    Name(&'static str),

    /// Serif fonts represent the formal text style for a script.
    Serif,

    /// Glyphs in sans-serif fonts, as the term is used in CSS, are generally low
    /// contrast and have stroke endings that are plain — without any flaring,
    /// cross stroke, or other ornamentation.
    #[default]
    SansSerif,

    /// Glyphs in cursive fonts generally use a more informal script style, and
    /// the result looks more like handwritten pen or brush writing than printed
    /// letterwork.
    Cursive,

    /// Fantasy fonts are primarily decorative or expressive fonts that contain
    /// decorative or expressive representations of characters.
    Fantasy,

    /// The sole criterion of a monospace font is that all glyphs have the same
    /// fixed width.
    Monospace,
}

impl Family {
    /// A list of all the different standalone family variants.
    pub const VARIANTS: &[Self] = &[
        Self::Serif,
        Self::SansSerif,
        Self::Cursive,
        Self::Fantasy,
        Self::Monospace,
    ];

    /// Creates a [`Family::Name`] from the given string.
    ///
    /// The name is interned in a global cache and never freed.
    pub fn name(name: &str) -> Self {
        use rustc_hash::FxHashSet;
        use std::sync::{LazyLock, Mutex};

        static NAMES: LazyLock<Mutex<FxHashSet<&'static str>>> = LazyLock::new(Mutex::default);

        let mut names = NAMES.lock().expect("lock font name cache");

        let Some(name) = names.get(name) else {
            let name: &'static str = name.to_owned().leak();
            let _ = names.insert(name);

            return Self::Name(name);
        };

        Self::Name(name)
    }
}

impl From<&str> for Family {
    fn from(name: &str) -> Self {
        Family::name(name)
    }
}

impl std::fmt::Display for Family {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Family::Name(name) => name,
            Family::Serif => "Serif",
            Family::SansSerif => "Sans-serif",
            Family::Cursive => "Cursive",
            Family::Fantasy => "Fantasy",
            Family::Monospace => "Monospace",
        })
    }
}

/// The weight of some text.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Weight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Normal,
    Medium,
    Semibold,
    Bold,
    ExtraBold,
    Black,
}

/// The width of some text.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Stretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    #[default]
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

/// The style of some text.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Style {
    #[default]
    Normal,
    Italic,
    Oblique,
}

/// A font error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn hash(font: Font) -> u64 {
        let mut hasher = DefaultHasher::new();
        font.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn a_font_defaults_to_the_face_s_own_tracking() {
        assert!(Font::DEFAULT.tracking.is_none());
        assert!(Font::MONOSPACE.tracking.is_none());
        assert!(Font::new("Archivo").tracking.is_none());
    }

    #[test]
    fn tracking_takes_part_in_equality_and_hashing() {
        let plain = Font::new("JetBrains Mono");
        let tracked = plain.tracking(Tracking(0.06));

        assert_ne!(plain, tracked);
        assert_ne!(hash(plain), hash(tracked));
        assert_eq!(
            tracked,
            Font::new("JetBrains Mono").tracking(Tracking(0.06))
        );
        assert_eq!(
            hash(tracked),
            hash(Font::new("JetBrains Mono").tracking(Tracking(0.06)))
        );
    }

    #[test]
    fn equality_and_hashing_agree_on_the_awkward_values() {
        // A hash key must never claim two values are equal while hashing them
        // differently. Negative zero and NaN are the two that would.
        let zero = Font::DEFAULT.tracking(Tracking(0.0));
        let negative_zero = Font::DEFAULT.tracking(Tracking(-0.0));
        assert_eq!(zero, negative_zero);
        assert_eq!(hash(zero), hash(negative_zero));

        let nan = Font::DEFAULT.tracking(Tracking(f32::NAN));
        let other_nan = Font::DEFAULT.tracking(Tracking(-f32::NAN));
        assert_eq!(nan, other_nan);
        assert_eq!(hash(nan), hash(other_nan));
    }

    #[test]
    fn only_a_real_tracking_counts_as_set() {
        assert!(Tracking::NONE.is_none());
        assert!(Tracking(0.0).is_none());
        assert!(Tracking(-0.0).is_none());
        assert!(!Tracking(0.06).is_none());
    }
}
