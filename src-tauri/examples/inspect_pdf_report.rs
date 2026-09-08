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
use std::time::Instant;

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

/// The manifest fields known before any page is rendered, plus the running
/// aggregates collected as pages stream past. `pages` itself is never held
/// here: each page's already-serialized JSON record is assembled straight
/// into `manifest.json` from its own small file (see [`write_page_record`]
/// and [`write_manifest_file`]), so this process never holds more than one
/// page's glyph data in memory at a time.
struct ManifestHeader {
    input: String,
    input_sha256: String,
    page_count: usize,
    pages_with_no_text: Vec<usize>,
    outside_page_glyphs: usize,
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

/// Writes one page's already-computed record to its own small JSON file in
/// `stage`, immediately after that page finishes (PNG and record together),
/// so the file system carries evidence of progress instead of everything
/// staying buffered in this process until the very end.
fn write_page_record(stage: &Path, page: &RenderedPage) -> AnyResult<PathBuf> {
    let path = stage.join(format!("page-{:04}.record.json", page.page_number));
    let file = File::create(&path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, page)?;
    writer.flush()?;
    Ok(path)
}

/// Renders every page, one at a time, writing that page's PNG and JSON
/// record to `stage` as soon as it is done and printing `page i/n` progress
/// to stderr. Returns the header fields plus the ordered list of per-page
/// record files still to be folded into the final `manifest.json`.
fn inspect_pages(arguments: &Arguments, stage: &Path) -> AnyResult<(ManifestHeader, Vec<PathBuf>)> {
    let library_dir = env::var_os("ABAKUS_PDFIUM_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("src-tauri/resources/pdfium"));
    let library = Pdfium::pdfium_platform_library_name_at_path(&library_dir);
    let pdfium = Pdfium::new(Pdfium::bind_to_library(&library)?);
    let document = pdfium.load_pdf_from_file(&arguments.input, None)?;
    let page_count = document.pages().len() as usize;
    if page_count == 0 {
        return Err("PDF contains no pages".into());
    }

    let mut pages_with_no_text = Vec::new();
    let mut outside_page_glyphs = 0_usize;
    let mut record_paths = Vec::with_capacity(page_count);
    let started = Instant::now();
    for (index, page) in document.pages().iter().enumerate() {
        let page_started = Instant::now();
        let rendered = render_page(&page, index + 1, stage)?;
        if rendered.extracted_text.trim().is_empty() {
            pages_with_no_text.push(rendered.page_number);
        }
        outside_page_glyphs += rendered.outside_page_glyphs;
        eprintln!(
            "page {}/{page_count} ({:.3}s, {} glyphs)",
            rendered.page_number,
            page_started.elapsed().as_secs_f64(),
            rendered.glyph_count,
        );
        record_paths.push(write_page_record(stage, &rendered)?);
    }
    eprintln!(
        "inspect_pdf_report: {page_count} pages done in {:.3}s",
        started.elapsed().as_secs_f64()
    );

    Ok((
        ManifestHeader {
            input: arguments.input.canonicalize()?.display().to_string(),
            input_sha256: sha256_file(&arguments.input)?,
            page_count,
            pages_with_no_text,
            outside_page_glyphs,
        },
        record_paths,
    ))
}

/// Renders the manifest's non-`pages` fields as a JSON object prefix, open
/// brace through the start of the `"pages"` array.
fn manifest_header_json(header: &ManifestHeader) -> AnyResult<String> {
    Ok(format!(
        "{{\n  \"input\": {},\n  \"input_sha256\": {},\n  \"page_count\": {},\n  \"pages_with_no_text\": {},\n  \"outside_page_glyphs\": {},\n  \"pages\": [\n",
        serde_json::to_string(&header.input)?,
        serde_json::to_string(&header.input_sha256)?,
        header.page_count,
        serde_json::to_string(&header.pages_with_no_text)?,
        header.outside_page_glyphs,
    ))
}

/// Writes the `"pages"` array body by copying each already-serialized
/// per-page record file's bytes straight through, comma-separated.
fn write_manifest_pages(writer: &mut impl Write, record_paths: &[PathBuf]) -> AnyResult<()> {
    for (index, record_path) in record_paths.iter().enumerate() {
        if index > 0 {
            writer.write_all(b",\n")?;
        }
        writer.write_all(&fs::read(record_path)?)?;
    }
    Ok(())
}

/// Assembles the final `manifest.json` from the header fields and the
/// already-serialized per-page record files, in the same shape the old
/// buffered-in-memory `Manifest` struct produced, then removes the
/// now-redundant per-page record files.
fn write_manifest_file(stage: &Path, header: &ManifestHeader, record_paths: &[PathBuf]) -> AnyResult<()> {
    let file = File::create(stage.join("manifest.json"))?;
    let mut writer = BufWriter::new(file);
    writer.write_all(manifest_header_json(header)?.as_bytes())?;
    write_manifest_pages(&mut writer, record_paths)?;
    writer.write_all(b"\n  ]\n}\n")?;
    writer.flush()?;
    for record_path in record_paths {
        fs::remove_file(record_path)?;
    }
    Ok(())
}

fn create_stage(arguments: &Arguments) -> AnyResult<PathBuf> {
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
    Ok(stage)
}

fn write_manifest(arguments: &Arguments, stage: &Path) -> AnyResult<()> {
    let (header, record_paths) = inspect_pages(arguments, stage)?;
    write_manifest_file(stage, &header, &record_paths)
}

fn commit_stage(arguments: &Arguments, stage: &Path) -> AnyResult<()> {
    if arguments.output.exists() {
        return Err(format!("output directory appeared during inspection: {}", arguments.output.display()).into());
    }
    fs::rename(stage, &arguments.output)?;
    Ok(())
}

fn remove_stage(stage: &Path, parent: &Path) -> AnyResult<()> {
    let resolved_stage = stage.canonicalize()?;
    let resolved_parent = parent.canonicalize()?;
    if resolved_stage.parent() != Some(resolved_parent.as_path()) {
        return Err("refusing cleanup outside the output parent".into());
    }
    fs::remove_dir_all(resolved_stage)?;
    Ok(())
}

fn finish_stage(result: AnyResult<()>, stage: &Path, parent: &Path) -> AnyResult<()> {
    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            if stage.exists() {
                remove_stage(stage, parent)?;
            }
            Err(error)
        }
    }
}

fn publish(arguments: &Arguments) -> AnyResult<()> {
    let stage = create_stage(arguments)?;
    let parent = arguments.output.parent().ok_or("output directory needs a parent")?;
    let result = write_manifest(arguments, &stage).and_then(|()| commit_stage(arguments, &stage));
    finish_stage(result, &stage, parent)
}

fn main() -> AnyResult<()> {
    let arguments = parse_args()?;
    publish(&arguments)
}
