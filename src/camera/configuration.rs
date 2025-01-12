use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Debug, Serialize)]
pub struct Configuration {
    pub width: u16,
    pub height: u16,
    pub fps: u16,
    pub format: PixelFormat,
    pub convergence: (f32, f32),
    pub multiview_mode: MultiviewMode,
    pub anaglyph_format: AnaglyphFormat,
    pub codec: VideoCodec,
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct NullableConfiguration {
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub fps: Option<u16>,
    pub format: Option<PixelFormat>,
    pub convergence: Option<(f32, f32)>,
    pub multiview_mode: Option<MultiviewMode>,
    pub anaglyph_format: Option<AnaglyphFormat>,
    pub codec: Option<VideoCodec>,
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub enum AnaglyphFormat {
    #[serde(rename = "green-magenta")]
    GreenMagenta,
    #[serde(rename = "red-cyan")]
    #[default]
    RedCyan,
    #[serde(rename = "amber-blue")]
    AmberBlue,
}

impl AnaglyphFormat {
    pub fn as_gst_str(&self) -> &str {
        match self {
            AnaglyphFormat::GreenMagenta => "0",
            AnaglyphFormat::RedCyan => "1",
            AnaglyphFormat::AmberBlue => "2",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub enum VideoCodec {
    Prores,
    #[default]
    MotionJpeg,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct MultiviewMode(gstreamer_video::VideoMultiviewMode);

impl MultiviewMode {
    pub fn as_gst(&self) -> gstreamer_video::VideoMultiviewMode {
        self.0
    }
}

impl From<MultiviewMode> for gstreamer_video::VideoMultiviewMode {
    fn from(mode: MultiviewMode) -> Self {
        mode.0
    }
}

impl Serialize for MultiviewMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use gstreamer_video::VideoMultiviewMode::*;

        match self.0 {
            SideBySide => "side-by-side",
            TopBottom => "top-bottom",
            Checkerboard => "checkerboard",
            SideBySideQuincunx => "side-by-side-quincunx",
            ColumnInterleaved => "column-interleaved",
            RowInterleaved => "row-interleaved",
            Mono => "mono",
            Left => "left",
            Right => "right",
            FrameByFrame => "frame-by-frame",
            MultiviewFrameByFrame => "multiview-frame-by-frame",
            Separated => "separated",
            _ => "none",
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MultiviewMode {
    fn deserialize<D>(deserializer: D) -> Result<MultiviewMode, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use gstreamer_video::VideoMultiviewMode::*;

        let s = String::deserialize(deserializer)?;
        let mode = match s.as_str() {
            "side-by-side" => SideBySide,
            "top-bottom" => TopBottom,
            "checkerboard" => Checkerboard,
            "side-by-side-quincunx" => SideBySideQuincunx,
            "column-interleaved" => ColumnInterleaved,
            "row-interleaved" => RowInterleaved,
            "mono" => Mono,
            "left" => Left,
            "right" => Right,
            "frame-by-frame" => FrameByFrame,
            "multiview-frame-by-frame" => MultiviewFrameByFrame,
            "separated" => Separated,
            "none" => None,
            _ => Err(serde::de::Error::custom("invalid multiview mode"))?,
        };
        Ok(MultiviewMode(mode))
    }
}

impl Default for MultiviewMode {
    fn default() -> Self {
        Self(gstreamer_video::VideoMultiviewMode::None)
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            fps: 60,
            format: PixelFormat::NV12,
            convergence: (0.0, 0.0),
            multiview_mode: MultiviewMode(gstreamer_video::VideoMultiviewMode::SideBySide),
            anaglyph_format: AnaglyphFormat::default(),
            codec: VideoCodec::default(),
        }
    }
}

impl From<Configuration> for NullableConfiguration {
    fn from(config: Configuration) -> Self {
        Self {
            width: Some(config.width),
            height: Some(config.height),
            fps: Some(config.fps),
            format: Some(config.format),
            convergence: Some(config.convergence),
            multiview_mode: Some(config.multiview_mode),
            anaglyph_format: Some(config.anaglyph_format),
            codec: Some(config.codec),
        }
    }
}

impl From<NullableConfiguration> for Configuration {
    fn from(config: NullableConfiguration) -> Self {
        let default = Configuration::default();

        Self {
            width: config.width.unwrap_or(default.width),
            height: config.height.unwrap_or(default.height),
            fps: config.fps.unwrap_or(default.fps),
            format: config.format.unwrap_or(default.format),
            convergence: config.convergence.unwrap_or(default.convergence),
            multiview_mode: config.multiview_mode.unwrap_or(default.multiview_mode),
            anaglyph_format: config.anaglyph_format.unwrap_or(default.anaglyph_format),
            codec: config.codec.unwrap_or(default.codec),
        }
    }
}

impl Configuration {
    pub fn merge(&self, other: &NullableConfiguration) -> Self {
        Self {
            width: other.width.unwrap_or(self.width),
            height: other.height.unwrap_or(self.height),
            fps: other.fps.unwrap_or(self.fps),
            format: other.format.unwrap_or(self.format),
            convergence: other.convergence.unwrap_or(self.convergence),
            multiview_mode: other.multiview_mode.unwrap_or(self.multiview_mode),
            anaglyph_format: other.anaglyph_format.unwrap_or(self.anaglyph_format),
            codec: other.codec.unwrap_or(self.codec),
        }
    }
}

impl NullableConfiguration {
    pub fn merge(&self, other: &NullableConfiguration) -> Self {
        Self {
            width: other.width.or(self.width),
            height: other.height.or(self.height),
            fps: other.fps.or(self.fps),
            format: other.format.or(self.format),
            convergence: other.convergence.or(self.convergence),
            multiview_mode: other.multiview_mode.or(self.multiview_mode),
            anaglyph_format: other.anaglyph_format.or(self.anaglyph_format),
            codec: other.codec.or(self.codec),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Deserialize, Serialize, Default)]
pub enum PixelFormat {
    #[default]
    NV12,
}

impl Display for PixelFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PixelFormat::NV12 => write!(f, "NV12"),
        }
    }
}
