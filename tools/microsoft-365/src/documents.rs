use crate::api::{upload_to_drive, UploadedFile};
use crate::graph::{DOCX_MIME, PPTX_MIME};
use crate::types::{PowerpointSlide, WordSection};
use docx_rs::{
    AlignmentType, BorderType, Docx, Paragraph, Run, Table, TableBorder, TableBorderPosition,
    TableBorders, TableCell, TableRow, WidthType,
};
use std::io::Write;
use zip::write::FileOptions;

pub fn create_word_document(
    parent_folder_id: Option<&str>,
    name: &str,
    title: &str,
    subtitle: Option<&str>,
    sections: &[WordSection],
) -> Result<UploadedFile, String> {
    if title.trim().is_empty() {
        return Err("create_word_document requires a title".to_string());
    }
    let file_name = ensure_extension(name, ".docx");
    let bytes = build_docx(title, subtitle, sections)?;
    upload_to_drive(parent_folder_id, &file_name, &bytes, DOCX_MIME)
}

pub fn create_powerpoint(
    parent_folder_id: Option<&str>,
    name: &str,
    slides: &[PowerpointSlide],
) -> Result<UploadedFile, String> {
    if slides.is_empty() {
        return Err("create_powerpoint requires at least one slide".to_string());
    }
    let file_name = ensure_extension(name, ".pptx");
    let bytes = build_pptx(slides)?;
    upload_to_drive(parent_folder_id, &file_name, &bytes, PPTX_MIME)
}

fn build_docx(
    title: &str,
    subtitle: Option<&str>,
    sections: &[WordSection],
) -> Result<Vec<u8>, String> {
    let mut doc = Docx::new();

    let title_run = Run::new().add_text(title).size(48).bold();
    doc = doc.add_paragraph(
        Paragraph::new()
            .add_run(title_run)
            .align(AlignmentType::Center),
    );

    if let Some(sub) = subtitle {
        if !sub.trim().is_empty() {
            let sub_run = Run::new().add_text(sub).size(24).italic();
            doc = doc.add_paragraph(
                Paragraph::new()
                    .add_run(sub_run)
                    .align(AlignmentType::Center),
            );
        }
    }

    doc = doc.add_paragraph(Paragraph::new());

    for section in sections {
        if let Some(heading) = &section.heading {
            if !heading.trim().is_empty() {
                let run = Run::new().add_text(heading.as_str()).size(32).bold();
                doc = doc.add_paragraph(Paragraph::new().add_run(run));
            }
        }

        for paragraph in &section.paragraphs {
            let run = Run::new().add_text(paragraph.as_str()).size(22);
            doc = doc.add_paragraph(Paragraph::new().add_run(run));
        }

        if let Some(rows) = &section.table {
            if !rows.is_empty() {
                let column_count = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
                let total_width = 9000;
                let column_width = total_width / column_count;
                let grid: Vec<usize> = vec![column_width; column_count];

                let mut table_rows = Vec::with_capacity(rows.len());
                for (idx, row) in rows.iter().enumerate() {
                    let mut cells = Vec::with_capacity(column_count);
                    for col in 0..column_count {
                        let text = row.get(col).map(|s| s.as_str()).unwrap_or("");
                        let mut run = Run::new().add_text(text).size(20);
                        if idx == 0 {
                            run = run.bold();
                        }
                        cells.push(
                            TableCell::new()
                                .add_paragraph(Paragraph::new().add_run(run))
                                .width(column_width, WidthType::Dxa),
                        );
                    }
                    table_rows.push(TableRow::new(cells));
                }

                let table = Table::new(table_rows)
                    .set_grid(grid)
                    .set_borders(default_table_borders());

                doc = doc.add_table(table);
                doc = doc.add_paragraph(Paragraph::new());
            }
        }
    }

    let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
    doc.build()
        .pack(&mut cursor)
        .map_err(|e| format!("docx pack failed: {}", e))?;
    Ok(cursor.into_inner())
}

fn default_table_borders() -> TableBorders {
    let positions = [
        TableBorderPosition::Top,
        TableBorderPosition::Bottom,
        TableBorderPosition::Left,
        TableBorderPosition::Right,
        TableBorderPosition::InsideH,
        TableBorderPosition::InsideV,
    ];
    let mut borders = TableBorders::with_empty();
    for position in positions {
        let border = TableBorder::new(position)
            .border_type(BorderType::Single)
            .size(4)
            .color("000000".to_string());
        borders = borders.set(border);
    }
    borders
}

fn build_pptx(slides: &[PowerpointSlide]) -> Result<Vec<u8>, String> {
    let mut buffer: Vec<u8> = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let opts: FileOptions =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        write_zip(
            &mut zip,
            "[Content_Types].xml",
            content_types_xml(slides.len()),
            opts,
        )?;
        write_zip(&mut zip, "_rels/.rels", package_rels(), opts)?;
        write_zip(
            &mut zip,
            "ppt/presentation.xml",
            presentation_xml(slides.len()),
            opts,
        )?;
        write_zip(
            &mut zip,
            "ppt/_rels/presentation.xml.rels",
            presentation_rels(slides.len()),
            opts,
        )?;
        write_zip(
            &mut zip,
            "ppt/slideMasters/slideMaster1.xml",
            slide_master_xml(),
            opts,
        )?;
        write_zip(
            &mut zip,
            "ppt/slideMasters/_rels/slideMaster1.xml.rels",
            slide_master_rels(),
            opts,
        )?;
        write_zip(
            &mut zip,
            "ppt/slideLayouts/slideLayout1.xml",
            slide_layout_xml(),
            opts,
        )?;
        write_zip(
            &mut zip,
            "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
            slide_layout_rels(),
            opts,
        )?;
        write_zip(&mut zip, "ppt/theme/theme1.xml", theme_xml(), opts)?;

        for (idx, slide) in slides.iter().enumerate() {
            let slide_num = idx + 1;
            write_zip(
                &mut zip,
                &format!("ppt/slides/slide{}.xml", slide_num),
                build_slide_xml(slide),
                opts,
            )?;
            write_zip(
                &mut zip,
                &format!("ppt/slides/_rels/slide{}.xml.rels", slide_num),
                slide_rels(),
                opts,
            )?;
        }

        zip.finish()
            .map_err(|e| format!("pptx finalize failed: {}", e))?;
    }
    Ok(buffer)
}

fn write_zip<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    path: &str,
    contents: String,
    opts: FileOptions,
) -> Result<(), String> {
    zip.start_file(path, opts)
        .map_err(|e| format!("zip start_file {} failed: {}", path, e))?;
    zip.write_all(contents.as_bytes())
        .map_err(|e| format!("zip write {} failed: {}", path, e))?;
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn ensure_extension(name: &str, ext: &str) -> String {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(ext) {
        name.to_string()
    } else {
        format!("{}{}", name, ext)
    }
}

fn content_types_xml(slide_count: usize) -> String {
    let mut parts = String::new();
    parts.push_str(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
<Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
<Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
<Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
"#,
    );
    for i in 1..=slide_count {
        parts.push_str(&format!(
            "<Override PartName=\"/ppt/slides/slide{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>\n",
            i
        ));
    }
    parts.push_str("</Types>");
    parts
}

fn package_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#
        .to_string()
}

fn presentation_xml(slide_count: usize) -> String {
    let mut slide_id_list = String::new();
    for i in 0..slide_count {
        slide_id_list.push_str(&format!(
            "<p:sldId id=\"{}\" r:id=\"rId{}\"/>\n",
            256 + i,
            2 + i
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" saveSubsetFonts="1">
<p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>
<p:sldIdLst>
{slide_id_list}</p:sldIdLst>
<p:sldSz cx="9144000" cy="6858000" type="screen4x3"/>
<p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#
    )
}

fn presentation_rels(slide_count: usize) -> String {
    let mut rels = String::new();
    rels.push_str(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
"#,
    );
    for i in 1..=slide_count {
        rels.push_str(&format!(
            "<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{}.xml\"/>\n",
            i + 1,
            i
        ));
    }
    rels.push_str("</Relationships>");
    rels
}

fn slide_master_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:bg><p:bgRef idx="1001"><a:schemeClr val="bg1"/></p:bgRef></p:bg>
<p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
</p:spTree></p:cSld>
<p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
<p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst>
<p:txStyles>
<p:titleStyle><a:lvl1pPr><a:defRPr sz="4400"/></a:lvl1pPr></p:titleStyle>
<p:bodyStyle><a:lvl1pPr><a:defRPr sz="2400"/></a:lvl1pPr></p:bodyStyle>
<p:otherStyle/>
</p:txStyles>
</p:sldMaster>"#
        .to_string()
}

fn slide_master_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#
        .to_string()
}

fn slide_layout_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="title" preserve="1">
<p:cSld name="Title Slide">
<p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
</p:spTree></p:cSld>
</p:sldLayout>"#
        .to_string()
}

fn slide_layout_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#
        .to_string()
}

fn theme_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
<a:themeElements>
<a:clrScheme name="Office">
<a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
<a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
<a:dk2><a:srgbClr val="1F497D"/></a:dk2>
<a:lt2><a:srgbClr val="EEECE1"/></a:lt2>
<a:accent1><a:srgbClr val="4F81BD"/></a:accent1>
<a:accent2><a:srgbClr val="C0504D"/></a:accent2>
<a:accent3><a:srgbClr val="9BBB59"/></a:accent3>
<a:accent4><a:srgbClr val="8064A2"/></a:accent4>
<a:accent5><a:srgbClr val="4BACC6"/></a:accent5>
<a:accent6><a:srgbClr val="F79646"/></a:accent6>
<a:hlink><a:srgbClr val="0000FF"/></a:hlink>
<a:folHlink><a:srgbClr val="800080"/></a:folHlink>
</a:clrScheme>
<a:fontScheme name="Office">
<a:majorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont>
<a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont>
</a:fontScheme>
<a:fmtScheme name="Office">
<a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst>
<a:lnStyleLst><a:ln><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst>
<a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst>
<a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst>
</a:fmtScheme>
</a:themeElements>
</a:theme>"#
        .to_string()
}

fn build_slide_xml(slide: &PowerpointSlide) -> String {
    let title = xml_escape(&slide.title);
    let mut body_paragraphs = String::new();
    for bullet in &slide.bullets {
        body_paragraphs.push_str(&format!(
            "<a:p><a:pPr lvl=\"0\"/><a:r><a:rPr lang=\"en-US\" dirty=\"0\"/><a:t>{}</a:t></a:r></a:p>",
            xml_escape(bullet)
        ));
    }
    if body_paragraphs.is_empty() {
        body_paragraphs.push_str("<a:p><a:endParaRPr lang=\"en-US\"/></a:p>");
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
<p:sp>
<p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
<p:spPr><a:xfrm><a:off x="457200" y="457200"/><a:ext cx="8229600" cy="1143000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>
<p:txBody><a:bodyPr anchor="ctr"/><a:lstStyle/><a:p><a:r><a:rPr lang="en-US" sz="4400" b="1"/><a:t>{title}</a:t></a:r></a:p></p:txBody>
</p:sp>
<p:sp>
<p:nvSpPr><p:cNvPr id="3" name="Body"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph idx="1"/></p:nvPr></p:nvSpPr>
<p:spPr><a:xfrm><a:off x="457200" y="1828800"/><a:ext cx="8229600" cy="4572000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>
<p:txBody><a:bodyPr/><a:lstStyle/>{body_paragraphs}</p:txBody>
</p:sp>
</p:spTree>
</p:cSld>
</p:sld>"#
    )
}

fn slide_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_escape_handles_all_reserved_chars() {
        assert_eq!(
            xml_escape(r#"<a href="x">'foo' & "bar"</a>"#),
            "&lt;a href=&quot;x&quot;&gt;&apos;foo&apos; &amp; &quot;bar&quot;&lt;/a&gt;"
        );
    }

    #[test]
    fn xml_escape_passes_through_safe_input() {
        assert_eq!(xml_escape("hello world"), "hello world");
    }

    #[test]
    fn ensure_extension_appends_when_missing() {
        assert_eq!(ensure_extension("report", ".docx"), "report.docx");
    }

    #[test]
    fn ensure_extension_idempotent_when_present() {
        assert_eq!(ensure_extension("report.docx", ".docx"), "report.docx");
    }

    #[test]
    fn ensure_extension_case_insensitive() {
        assert_eq!(ensure_extension("REPORT.DOCX", ".docx"), "REPORT.DOCX");
    }
}
