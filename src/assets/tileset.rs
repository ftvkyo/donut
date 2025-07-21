use std::{path::Path, sync::Arc};

use anyhow::{Context, Result, bail};
use image::ImageReader;
use log::debug;

use crate::assets::TextureData;

#[derive(Debug)]
pub struct Tileset {
    inner: Arc<tiled::Tileset>,
    pub texture_color: TextureData,
    pub texture_normal_specular: TextureData,
}

impl Tileset {
    pub fn load_for_map(tileset: &Arc<tiled::Tileset>) -> Result<Self> {
        debug!("Loading tileset '{}'...", tileset.name);

        let image = match &tileset.image {
            Some(image) => image,
            _ => bail!("Only 'regular' (atlas) tilesets are supported"),
        };

        let color_path = &image.source;

        let color_filename = image
            .source
            .file_name()
            .context("Image source has no filename?")?
            .to_str()
            .context("Image source filename is not valid UTF-8")?;
        let normal_filename = color_filename.replace("-color.", "-normal.");
        let specular_filename = color_filename.replace("-color.", "-specular.");

        let normal_path = image.source.with_file_name(normal_filename);
        let specular_path = image.source.with_file_name(specular_filename);

        Self::load(tileset, color_path, normal_path, specular_path)
    }

    fn load(
        tileset: &Arc<tiled::Tileset>,
        texture_color: impl AsRef<Path>,
        texture_normal: impl AsRef<Path>,
        texture_specular: impl AsRef<Path>,
    ) -> Result<Self> {
        let texture_color = texture_color.as_ref();
        let texture_normal = texture_normal.as_ref();
        let texture_specular = texture_specular.as_ref();

        debug!("Loading '{}'...", texture_color.to_string_lossy());
        let texture_color = ImageReader::open(texture_color)?.decode()?.into_rgba8();

        debug!("Loading '{}'...", texture_normal.to_string_lossy());
        let texture_normal = ImageReader::open(texture_normal)?.decode()?.into_rgba8();

        debug!("Loading '{}'...", texture_specular.to_string_lossy());
        let texture_specular = ImageReader::open(texture_specular)?.decode()?.into_luma8();

        let mut texture_normal_specular = texture_normal;
        for (x, y, pixel) in texture_specular.enumerate_pixels() {
            texture_normal_specular.get_pixel_mut(x, y).0[3] = pixel.0[0];
        }

        Ok(Self {
            inner: tileset.clone(),
            texture_color,
            texture_normal_specular,
        })
    }

    pub fn id(&self) -> TilesetId<'_> {
        TilesetId::new(&self.inner)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TilesetId<'t> {
    source: &'t Path,
    name: &'t str,
}

impl<'t> TilesetId<'t> {
    pub fn new(tileset: &'t tiled::Tileset) -> Self {
        Self {
            source: &tileset.source,
            name: &tileset.name,
        }
    }
}
