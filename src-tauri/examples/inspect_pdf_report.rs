//! Independent PDFium inspection process for generated Abakus reports.
//!
//! Usage: `--input <pdf> --output <new-directory>`. The process binds one
//! PDFium instance, extracts every page and glyph, renders every page to PNG,
//! and publishes the output directory only after all work succeeds.

use pdfium_render::prelude::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

type AnyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Serialize)]
struct Bounds {
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
}

#[derive(Serialize)]
struct Glyph {
    index: usize,
    unicode_value: u32,
    text: Option<String>,
    tight_bounds: Bounds,
    outside_page: bool,
}

#[derive(Serialize)]
struct RenderedPage {
    page_number: usize,
    width_points: f32,
    height_points: f32,
    extracted_text: String,
    glyph_count: usize,
    outside_page_glyphs: usize,
    glyphs: Vec<Glyph>,
    png_file: String,
    png_sha256: String,
    png_bytes: u64,
    png_width: u32,
    png_height: u32,
}

#[derive(Serialize)]
struct Manifest {
    input: String,
    input_sha256: String,
    page_count: usize,
    pages_with_no_text: Vec<usize>,
    outside_page_glyphs: usize,
    pages: Vec<RenderedPage>,
}

struct Arguments {
    input: PathBuf,
    output: PathBuf,
}

fn parse_args() -> AnyResult<Arguments> {
    let values: Vec<String> = env::args().skip(1).collect();
    if values.len() != 4 || values[0] != "--input" || values[2] != "--output" {
        return Err("usage: inspect_pdf_report --input <pdf> --output <new-directory>".into());
    }
    Ok(Arguments {
        input: PathBuf::from(&values[1]),
        output: PathBuf::from(&values[3]),
    })
}

fn sha256_file(path: &Path) -> AnyResult<String> {
    let bytes = fs::read(path)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn bounds(rect: PdfRect) -> Bounds {
    Bounds {
        left: rect.left().value,
        bottom: rect.bottom().value,
        right: rect.right().value,
        top: rect.top().value,
    }
}

fn outside_page(rect: &PdfRect, width: f32, height: f32) -> bool {
    const TOLERANCE_POINTS: f32 = 0.5;
    rect.left().value < -TOLERANCE_POINTS
        || rect.bottom().value < -TOLERANCE_POINTS
        || rect.right().value > width + TOLERANCE_POINTS
        || rect.top().value > height + TOLERANCE_POINTS
}

fn extract_glyphs(text: &PdfPageText<'_>, width: f32, height: f32) -> AnyResult<Vec<Glyph>> {
    text.chars()
        .iter()
        .enumerate()
        .map(|(index, character)| {
            let rect = character.tight_bounds()?;
            Ok(Glyph {
                index,
                unicode_value: character.unicode_value(),
                text: character.unicode_string(),
                tight_bounds: bounds(rect),
                outside_page: outside_page(&rect, width, height),
            })
        })
        .collect()
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> AnyResult<()> {
    let expected = usize::try_from(width)?
        .checked_mul(usize::try_from(height)?)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("rendered PNG dimensions overflow")?;
    if rgba.len() != expected {
        return Err(format!("RGBA byte length mismatch: expected {expected}, got {}", rgba.len()).into());
    }
    let file = File::create(path)?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    writer.finish()?;
    Ok(())
}

fn render_page(page: &PdfPage<'_>, page_number: usize, stage: &Path) -> AnyResult<RenderedPage> {
    let page_width = page.width().value;
    let page_height = page.height().value;
    let page_text = page.text()?;
    let extracted_text = page_text.all();
    let glyphs = extract_glyphs(&page_text, page_width, page_height)?;
    let outside_page_glyphs = glyphs.iter().filter(|glyph| glyph.outside_page).count();

    let render_config = PdfRenderConfig::new().set_target_width(2000).set_maximum_height(3000);
    let bitmap = page.render_with_config(&render_config)?;
    let png_width = u32::try_from(bitmap.width())?;
    let png_height = u32::try_from(bitmap.height())?;
    let png_file = format!("page-{page_number:04}.png");
    let png_path = stage.join(&png_file);
    write_png(&png_path, png_width, png_height, &bitmap.as_rgba_bytes())?;

    Ok(RenderedPage {
        page_number,
        width_points: page_width,
        height_points: page_height,
        extracted_text,
        glyph_count: glyphs.len(),
        outside_page_glyphs,
        glyphs,
        png_file,
        png_sha256: sha256_file(&png_path)?,
        png_bytes: fs::metadata(&png_path)?.len(),
        png_width,
        png_height,
    })
}

fn partial_path(output: &Path) -> AnyResult<PathBuf> {
    let parent = output.parent().ok_or("output directory needs a parent")?;
    let name = output.file_name().and_then(|value| value.to_str()).ok_or("output directory needs a valid name")?;
    Ok(parent.join(format!(".{name}.partial-{}", std::process::id())))
}

fn inspect(arguments: &Arguments, stage: &Path) -> AnyResult<Manifest> {
    let library_dir = env::var_os("ABAKUS_PDFIUM_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("src-tauri/resources/pdfium"));
    let library = Pdfium::pdfium_platform_library_name_at_path(&library_dir);
    let pdfium = Pdfium::new(Pdfium::bind_to_library(&library)?);
    let document = pdfium.load_pdf_from_file(&arguments.input, None)?;
    if document.pages().is_empty() {
        return Err("PDF contains no pages".into());
    }

    let mut pages = Vec::with_capacity(document.pages().len());
    for (index, page) in document.pages().iter().enumerate() {
        pages.push(render_page(&page, index + 1, stage)?);
    }
    let pages_with_no_text = pages
        .iter()
        .filter(|page| page.extracted_text.trim().is_empty())
        .map(|page| page.page_number)
        .collect();
    let outside_page_glyphs = pages.iter().map(|page| page.outside_page_glyphs).sum();
    Ok(Manifest {
        input: arguments.input.canonicalize()?.display().to_string(),
        input_sha256: sha256_file(&arguments.input)?,
        page_count: pages.len(),
        pages_with_no_text,
        outside_page_glyphs,
        pages,
    })
}

fn publish(arguments: &Arguments) -> AnyResult<()> {
    if !arguments.input.is_file() {
        return Err(format!("input PDF is not a file: {}", arguments.input.display()).into());
    }
    if arguments.output.exists() {
        return Err(format!("refusing existing output directory: {}", arguments.output.display()).into());
    }
    let parent = arguments.output.parent().ok_or("output directory needs a parent")?;
    fs::create_dir_all(parent)?;
    let stage = partial_path(&arguments.output)?;
    if stage.exists() {
        return Err(format!("refusing existing partial directory: {}", stage.display()).into());
    }
    fs::create_dir(&stage)?;

    let result = (|| -> AnyResult<()> {
        let manifest = inspect(arguments, &stage)?;
        let manifest_path = stage.join("manifest.json");
        let file = File::create(&manifest_path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &manifest)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        if arguments.output.exists() {
            return Err(format!("output directory appeared during inspection: {}", arguments.output.display()).into());
        }
        fs::rename(&stage, &arguments.output)?;
        Ok(())
    })();

    if result.is_err() && stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    result
}

fn main() -> AnyResult<()> {
    let arguments = parse_args()?;
    publish(&arguments)
}
