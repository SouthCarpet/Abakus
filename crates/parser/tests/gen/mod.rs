use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};
use parser::fold::strip_marks;
use std::path::Path;

pub fn write_pdf(pages: &[Vec<String>], out: &Path) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Courier", "Encoding" => "WinAnsiEncoding" });
    let resources = doc.add_object(dictionary! { "Font" => dictionary! { "F1" => font_id } });
    let mut kids = Vec::new();
    for page in pages {
        let mut ops = vec![Operation::new("BT", vec![]), Operation::new("Tf", vec!["F1".into(), 8.into()])];
        let mut y: f32 = 800.0;
        for line in page {
            ops.push(Operation::new("Td", vec![0.into(), 0.into()]));
            ops.push(Operation::new("Tm", vec![1.into(), 0.into(), 0.into(), 1.into(), 30.into(), Object::Real(y)]));
            ops.push(Operation::new("Tj", vec![Object::string_literal(strip_marks(line))]));
            y -= 10.5;
        }
        ops.push(Operation::new("ET", vec![]));
        let content = Stream::new(dictionary! {}, Content { operations: ops }.encode().unwrap());
        let content_id = doc.add_object(content);
        let page_id = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages_id, "Contents" => content_id, "Resources" => resources, "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()] });
        kids.push(page_id.into());
    }
    let count = kids.len() as i64;
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }));
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    doc.save(out).unwrap();
}
