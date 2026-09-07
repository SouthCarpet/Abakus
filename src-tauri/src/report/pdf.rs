use super::font::{self, FontEncoding, FontKind};
use super::layout::{Color, DrawOp, RenderedReport, PAGE_HEIGHT, PAGE_WIDTH};
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream, StringFormat};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

pub(crate) fn write<W: Write>(rendered: RenderedReport, target: &mut W) -> Result<(), String> {
    let encodings = collect_encodings(&rendered)?;
    let mut document = build_document(rendered, &encodings)?;
    document.compress();
    document
        .save_to(target)
        .map_err(|error| format!("Zápis PDF zlyhal: {error}"))
}

fn collect_encodings(
    rendered: &RenderedReport,
) -> Result<BTreeMap<FontKind, FontEncoding>, String> {
    let mut scalars: BTreeMap<FontKind, BTreeSet<char>> = BTreeMap::new();
    for page in &rendered.pages {
        for operation in &page.ops {
            if let DrawOp::Text { font, text, .. } = operation {
                scalars.entry(*font).or_default().extend(text.chars());
            }
        }
    }
    scalars
        .into_iter()
        .map(|(kind, values)| Ok((kind, FontEncoding::build(kind, values)?)))
        .collect()
}

fn build_document(
    rendered: RenderedReport,
    encodings: &BTreeMap<FontKind, FontEncoding>,
) -> Result<Document, String> {
    let mut document = Document::with_version("1.7");
    let pages_id = document.new_object_id();
    let font_ids = add_fonts(&mut document, encodings)?;
    let resources_id = add_resources(&mut document, &font_ids);
    let mut page_ids = Vec::with_capacity(rendered.pages.len());
    for page in rendered.pages {
        let operations = page_operations(page.ops, encodings)?;
        let content = Content { operations }
            .encode()
            .map_err(|error| format!("Kódovanie obsahu PDF zlyhalo: {error}"))?;
        let content_id = document.add_object(Stream::new(dictionary! {}, content));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()],
            "Contents" => content_id,
        });
        page_ids.push(page_id);
    }
    let kids = page_ids
        .iter()
        .copied()
        .map(Object::Reference)
        .collect::<Vec<_>>();
    let page_count =
        i64::try_from(page_ids.len()).map_err(|_| "Počet strán PDF pretiekol.".to_string())?;
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => page_count,
        }),
    );
    let catalog_id = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    let info_id = document.add_object(dictionary! {
        "Title" => Object::string_literal("Abakus · Prehľad transakcií"),
        "Producer" => Object::string_literal("Abakus 0.1.2"),
    });
    document.trailer.set("Root", catalog_id);
    document.trailer.set("Info", info_id);
    Ok(document)
}

fn add_fonts(
    document: &mut Document,
    encodings: &BTreeMap<FontKind, FontEncoding>,
) -> Result<BTreeMap<FontKind, ObjectId>, String> {
    encodings
        .iter()
        .map(|(kind, encoding)| Ok((*kind, add_font(document, encoding)?)))
        .collect()
}

fn add_font(document: &mut Document, encoding: &FontEncoding) -> Result<ObjectId, String> {
    let face = font::face(encoding.kind)?;
    let (name, bytes, stem) = match encoding.kind {
        FontKind::Regular => ("NotoSans-Regular", font::REGULAR_BYTES, 80_i64),
        FontKind::Bold => ("NotoSans-Bold", font::BOLD_BYTES, 120_i64),
    };
    let font_length = i64::try_from(bytes.len())
        .map_err(|_| "Veľkosť zabudovaného fontu pretiekla.".to_string())?;
    let font_file_id = document.add_object(Stream::new(
        dictionary! { "Length1" => font_length },
        bytes.to_vec(),
    ));
    let bbox = face.global_bounding_box();
    let descriptor_id = document.add_object(dictionary! {
        "Type" => "FontDescriptor",
        "FontName" => Object::Name(name.as_bytes().to_vec()),
        "Flags" => 32,
        "FontBBox" => vec![scale_metric(bbox.x_min, face.units_per_em()).into(), scale_metric(bbox.y_min, face.units_per_em()).into(), scale_metric(bbox.x_max, face.units_per_em()).into(), scale_metric(bbox.y_max, face.units_per_em()).into()],
        "ItalicAngle" => 0,
        "Ascent" => scale_metric(face.ascender(), face.units_per_em()),
        "Descent" => scale_metric(face.descender(), face.units_per_em()),
        "CapHeight" => scale_metric(face.capital_height().unwrap_or(face.ascender()), face.units_per_em()),
        "StemV" => stem,
        "FontFile2" => font_file_id,
    });
    let cid_map_id = document.add_object(Stream::new(dictionary! {}, cid_to_gid(&face, encoding)?));
    let widths = widths(&face, encoding)?;
    let descendant_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "CIDFontType2",
        "BaseFont" => Object::Name(name.as_bytes().to_vec()),
        "CIDSystemInfo" => dictionary! { "Registry" => Object::string_literal("Adobe"), "Ordering" => Object::string_literal("Identity"), "Supplement" => 0 },
        "FontDescriptor" => descriptor_id,
        "DW" => 1000,
        "W" => vec![1.into(), Object::Array(widths)],
        "CIDToGIDMap" => cid_map_id,
    });
    let to_unicode_id = document.add_object(Stream::new(
        dictionary! {},
        to_unicode(encoding).into_bytes(),
    ));
    Ok(document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => Object::Name(name.as_bytes().to_vec()),
        "Encoding" => "Identity-H",
        "DescendantFonts" => vec![descendant_id.into()],
        "ToUnicode" => to_unicode_id,
    }))
}

fn scale_metric(value: i16, units: u16) -> i64 {
    i64::from(value) * 1000 / i64::from(units)
}

fn cid_to_gid(face: &ttf_parser::Face<'_>, encoding: &FontEncoding) -> Result<Vec<u8>, String> {
    let length = encoding
        .by_scalar
        .len()
        .checked_add(1)
        .and_then(|value| value.checked_mul(2))
        .ok_or_else(|| "CIDToGIDMap je príliš veľká.".to_string())?;
    let mut map = vec![0_u8; length];
    for (scalar, cid) in &encoding.by_scalar {
        let glyph = face
            .glyph_index(*scalar)
            .ok_or_else(|| format!("Font neobsahuje znak U+{:04X}.", u32::from(*scalar)))?;
        let offset = usize::from(*cid) * 2;
        map[offset..offset + 2].copy_from_slice(&glyph.0.to_be_bytes());
    }
    Ok(map)
}

fn widths(face: &ttf_parser::Face<'_>, encoding: &FontEncoding) -> Result<Vec<Object>, String> {
    let units = u64::from(face.units_per_em());
    encoding
        .by_scalar
        .keys()
        .map(|scalar| {
            let glyph = face
                .glyph_index(*scalar)
                .ok_or_else(|| format!("Font neobsahuje znak U+{:04X}.", u32::from(*scalar)))?;
            let advance = u64::from(face.glyph_hor_advance(glyph).unwrap_or(0));
            let width = (advance * 1000 + units / 2) / units;
            let width =
                i64::try_from(width).map_err(|_| "Šírka znaku fontu pretiekla.".to_string())?;
            Ok(Object::Integer(width))
        })
        .collect()
}

fn to_unicode(encoding: &FontEncoding) -> String {
    let mut cmap = String::from("/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /AbakusUnicode def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n");
    let entries = encoding.by_scalar.iter().collect::<Vec<_>>();
    for chunk in entries.chunks(100) {
        cmap.push_str(&format!("{} beginbfchar\n", chunk.len()));
        for (scalar, cid) in chunk {
            cmap.push_str(&format!("<{cid:04X}> <{}>\n", utf16_hex(**scalar)));
        }
        cmap.push_str("endbfchar\n");
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    cmap
}

fn utf16_hex(scalar: char) -> String {
    let mut units = [0_u16; 2];
    scalar
        .encode_utf16(&mut units)
        .iter()
        .map(|unit| format!("{unit:04X}"))
        .collect()
}

fn add_resources(document: &mut Document, font_ids: &BTreeMap<FontKind, ObjectId>) -> ObjectId {
    let mut fonts = Dictionary::new();
    for (kind, id) in font_ids {
        fonts.set(font_resource(*kind), Object::Reference(*id));
    }
    document.add_object(dictionary! { "Font" => fonts })
}

fn font_resource(kind: FontKind) -> &'static str {
    match kind {
        FontKind::Regular => "FRegular",
        FontKind::Bold => "FBold",
    }
}

fn page_operations(
    operations: Vec<DrawOp>,
    encodings: &BTreeMap<FontKind, FontEncoding>,
) -> Result<Vec<Operation>, String> {
    let mut output = Vec::new();
    for operation in operations {
        match operation {
            DrawOp::Text {
                x,
                y,
                size,
                font,
                color,
                text,
            } => text_operations(
                &mut output,
                encodings,
                TextSpec {
                    x,
                    y,
                    size,
                    font,
                    color,
                    text,
                },
            )?,
            DrawOp::Line {
                x1,
                y1,
                x2,
                y2,
                width,
                color,
            } => line_operations(&mut output, x1, y1, x2, y2, width, color),
            DrawOp::Rect {
                x,
                y,
                width,
                height,
                fill,
                stroke,
            } => rect_operations(&mut output, x, y, width, height, fill, stroke),
        }
    }
    Ok(output)
}

struct TextSpec {
    x: f32,
    y: f32,
    size: f32,
    font: FontKind,
    color: Color,
    text: String,
}

fn text_operations(
    output: &mut Vec<Operation>,
    encodings: &BTreeMap<FontKind, FontEncoding>,
    spec: TextSpec,
) -> Result<(), String> {
    let encoding = encodings
        .get(&spec.font)
        .ok_or_else(|| "PDF font nemá kódovanie.".to_string())?;
    output.push(Operation::new("q", vec![]));
    output.push(fill_color(spec.color));
    output.push(Operation::new("BT", vec![]));
    output.push(Operation::new(
        "Tf",
        vec![
            Object::Name(font_resource(spec.font).as_bytes().to_vec()),
            spec.size.into(),
        ],
    ));
    output.push(Operation::new(
        "Tm",
        vec![
            1.into(),
            0.into(),
            0.into(),
            1.into(),
            spec.x.into(),
            spec.y.into(),
        ],
    ));
    output.push(Operation::new(
        "Tj",
        vec![Object::String(
            encoding.encode(&spec.text)?,
            StringFormat::Hexadecimal,
        )],
    ));
    output.push(Operation::new("ET", vec![]));
    output.push(Operation::new("Q", vec![]));
    Ok(())
}

fn line_operations(
    output: &mut Vec<Operation>,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    width: f32,
    color: Color,
) {
    output.push(Operation::new("q", vec![]));
    output.push(stroke_color(color));
    output.push(Operation::new("w", vec![width.into()]));
    output.push(Operation::new("m", vec![x1.into(), y1.into()]));
    output.push(Operation::new("l", vec![x2.into(), y2.into()]));
    output.push(Operation::new("S", vec![]));
    output.push(Operation::new("Q", vec![]));
}

fn rect_operations(
    output: &mut Vec<Operation>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    fill: Option<Color>,
    stroke: Option<Color>,
) {
    output.push(Operation::new("q", vec![]));
    if let Some(color) = fill {
        output.push(fill_color(color));
    }
    if let Some(color) = stroke {
        output.push(stroke_color(color));
        output.push(Operation::new("w", vec![0.5.into()]));
    }
    output.push(Operation::new(
        "re",
        vec![x.into(), y.into(), width.into(), height.into()],
    ));
    let paint = match (fill.is_some(), stroke.is_some()) {
        (true, true) => "B",
        (true, false) => "f",
        (false, true) => "S",
        (false, false) => "n",
    };
    output.push(Operation::new(paint, vec![]));
    output.push(Operation::new("Q", vec![]));
}

fn fill_color(Color(red, green, blue): Color) -> Operation {
    Operation::new("rg", vec![red.into(), green.into(), blue.into()])
}

fn stroke_color(Color(red, green, blue): Color) -> Operation {
    Operation::new("RG", vec![red.into(), green.into(), blue.into()])
}
