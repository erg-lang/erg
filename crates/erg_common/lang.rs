use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageCode {
    English,
    Japanese,
    SimplifiedChinese,
    TraditionalChinese,
    Erg,
    Python,
    ErgOrPython,
}

impl FromStr for LanguageCode {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "english" | "en" => Ok(Self::English),
            "japanese" | "ja" | "jp" => Ok(Self::Japanese),
            "simplified_chinese" | "zh-CN" => Ok(Self::SimplifiedChinese),
            "traditional_chinese" | "zh-TW" => Ok(Self::TraditionalChinese),
            "erg" => Ok(Self::Erg),
            "python" => Ok(Self::Python),
            "erg,python" | "python,erg" => Ok(Self::ErgOrPython),
            _ => Err(()),
        }
    }
}

impl From<LanguageCode> for &str {
    fn from(code: LanguageCode) -> Self {
        match code {
            LanguageCode::English => "english",
            LanguageCode::Japanese => "japanese",
            LanguageCode::SimplifiedChinese => "simplified_chinese",
            LanguageCode::TraditionalChinese => "traditional_chinese",
            LanguageCode::Erg => "erg",
            LanguageCode::Python => "python",
            LanguageCode::ErgOrPython => "erg,python",
        }
    }
}

impl LanguageCode {
    pub const fn en_patterns() -> [&'static str; 2] {
        ["en", "english"]
    }
    pub const fn ja_patterns() -> [&'static str; 2] {
        ["ja", "japanese"]
    }
    pub const fn zh_cn_patterns() -> [&'static str; 2] {
        ["zh-CN", "simplified_chinese"]
    }
    pub const fn zh_tw_patterns() -> [&'static str; 2] {
        ["zh-TW", "traditional_chinese"]
    }
    pub const fn erg_patterns() -> [&'static str; 2] {
        ["erg", "erg"]
    }
    pub const fn python_patterns() -> [&'static str; 2] {
        ["python", "python"]
    }
    pub const fn erg_or_python_patterns() -> [&'static str; 2] {
        ["erg,python", "python,erg"]
    }
    pub const fn patterns(&self) -> [&'static str; 2] {
        match self {
            Self::English => Self::en_patterns(),
            Self::Japanese => Self::ja_patterns(),
            Self::SimplifiedChinese => Self::zh_cn_patterns(),
            Self::TraditionalChinese => Self::zh_tw_patterns(),
            Self::Erg => Self::erg_patterns(),
            Self::Python => Self::python_patterns(),
            Self::ErgOrPython => Self::erg_or_python_patterns(),
        }
    }

    pub const fn is_en(&self) -> bool {
        matches!(self, Self::English)
    }
    pub const fn is_ja(&self) -> bool {
        matches!(self, Self::Japanese)
    }
    pub const fn is_zh_cn(&self) -> bool {
        matches!(self, Self::SimplifiedChinese)
    }
    pub const fn is_zh_tw(&self) -> bool {
        matches!(self, Self::TraditionalChinese)
    }
    pub const fn is_erg(&self) -> bool {
        matches!(self, Self::Erg | Self::ErgOrPython)
    }
    pub const fn is_python(&self) -> bool {
        matches!(self, Self::Python | Self::ErgOrPython)
    }
    pub const fn is_pl(&self) -> bool {
        matches!(self, Self::Erg | Self::Python | Self::ErgOrPython)
    }

    pub const fn matches_feature(&self) -> bool {
        match self {
            Self::English => {
                !cfg!(feature = "japanese")
                    && !cfg!(feature = "simplified_chinese")
                    && !cfg!(feature = "traditional_chinese")
            }
            Self::Japanese => cfg!(feature = "japanese"),
            Self::SimplifiedChinese => cfg!(feature = "simplified_chinese"),
            Self::TraditionalChinese => cfg!(feature = "traditional_chinese"),
            Self::Erg => !cfg!(feature = "py_compat"),
            Self::Python => cfg!(feature = "py_compat"),
            Self::ErgOrPython => true,
        }
    }
    pub fn as_str(&self) -> &str {
        <&str>::from(*self)
    }
}
