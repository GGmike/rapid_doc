use lopdf::{Document, Object};
use std::io::BufWriter;
use std::fs::File;
use std::path::Path;
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)] 
struct TextItem {
    #[pyo3(get)]
    text: String,
    #[pyo3(get)]
    x: f32,
    #[pyo3(get)]
    y: f32,
    #[pyo3(get)]
    font_size: f32,
    #[pyo3(get)]
    page_num: u32,
}

#[pyclass]
#[derive(Debug, Clone)]
struct CharItem {
    #[pyo3(get)]
    char: String,
    #[pyo3(get)]
    x: f32,
    #[pyo3(get)]
    y: f32,
    #[pyo3(get)]
    font_size: f32,
    #[pyo3(get)]
    page_num: u32,
    #[pyo3(get)]
    char_width: f32,
}

#[pyfunction]
fn replace_text_by_pos(
    path: String, 
    output_path: String,
    page_num: u32, 
    target_text: &str,
    replacement: &str,
    target_x: f32,  
    target_y: f32,
    target_font_size: f32,
) -> PyResult<String>
{
    let mut doc = Document::load(path).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
    let pages = doc.get_pages();
    if let Some(&object_id) = pages.get(&page_num) {
        let content_data = doc.get_page_content(object_id).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        let mut content = lopdf::content::Content::decode(&content_data).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        
        let mut current_font_size: f32 = 0.0;
        let mut current_x: f32 = 0.0;
        let mut current_y: f32 = 0.0;
        
        for operation in &mut content.operations {
            let operator = &operation.operator;
            let operands = &operation.operands;
            
            match operator.as_str() {
                "BT" => {
                    current_x = 0.0;
                    current_y = 0.0;
                }
                "Tf" => {
                    if operands.len() >= 2 {
                        if let Ok(size) = operands[1].as_f32() {
                            current_font_size = size;
                        }
                    }
                }
                "Td" | "TD" => {
                    if operands.len() >= 2 {
                        if let (Ok(tx), Ok(ty)) = (operands[0].as_f32(), operands[1].as_f32()) {
                            current_x += tx;
                            current_y += ty;
                        }
                    }
                }
                "Tm" => {
                    if operands.len() >= 6 {
                        if let (Ok(e), Ok(f)) = (operands[4].as_f32(), operands[5].as_f32()) {
                            current_x = e;
                            current_y = f;
                        }
                    }
                }
                "Tj" => {
                    if let Some(Object::String(bytes, _)) = operands.first() {
                        println!("Checking text at ({}, {}): '{}'", current_x, current_y, String::from_utf8_lossy(bytes));
                    
                        if current_x == target_x && current_y == target_y  {
                            println!("Positions match for Tj at ({}, {})", current_x, current_y);
                            if String::from_utf8_lossy(bytes).contains(target_text){
                                let mut original_text = String::from_utf8_lossy(bytes).to_string();
                                original_text = original_text.replace(target_text, replacement);
                                operation.operands[0] = Object::String(original_text.as_bytes().to_vec(), lopdf::StringFormat::Literal);
                                println!("Replaced text at ({}, {})", current_x, current_y);    
                                break;  // Replace first exact match
                                }
                        }
                        if String::from_utf8_lossy(bytes) == target_text
                            && (current_x - target_x).abs() < 0.01  // Allow small floating-point tolerance
                            && (current_y - target_y).abs() < 0.01
                            // && (current_font_size - target_font_size).abs() < 0.01
                                {
                            operation.operands[0] = Object::String(replacement.as_bytes().to_vec(), lopdf::StringFormat::Literal);
                            println!("Replaced text at ({}, {})", current_x, current_y);    
                            break;  // Replace first exact match
                        }
                    }
                }
                _ => {}
            }
        }
        
        let new_content_data = content.encode().map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        doc.change_page_content(object_id, new_content_data).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
    }    
    let path = Path::new(&output_path);
    let mut file = BufWriter::new(File::create(path).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?);
    doc.save_modern(&mut file).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
    println!("Saved modified PDF to {}", output_path);
    let result_message = format!("Saved modified PDF to {}", output_path);
    Ok(result_message)
}

#[pyfunction]
fn extract_text_from_pdf(path: String) -> PyResult<Vec<TextItem>> {
    let doc = Document::load(path).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
    let mut all_items = Vec::new();

    for (_page_num, object_id) in doc.get_pages() {
        if let Ok(content_data) = doc.get_page_content(object_id) {
            if let Ok(content) = lopdf::content::Content::decode(&content_data) {
                let mut items = process_content_stream(&content, _page_num);
                all_items.append(&mut items);
            }
        }
    }

    all_items.sort_by(|a, b| {
        b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal)
           .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });


    Ok(all_items)
}

#[pymodule]
fn rapid_pdf(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TextItem>()?;
    m.add_class::<CharItem>()?;
    m.add_function(wrap_pyfunction!(extract_chars_from_pdf, m)?)?;z
    m.add_function(wrap_pyfunction!(extract_text_from_pdf, m)?)?;
    m.add_function(wrap_pyfunction!(replace_text_by_pos, m)?)?;
    Ok(())
}

fn process_content_stream(content: &lopdf::content::Content, page_num: u32) -> Vec<TextItem> {
    let mut extracted_items = Vec::new();

    let mut current_font_size: f32 = 0.0;
    let mut current_x: f32 = 0.0;
    let mut current_y: f32 = 0.0;

    for operation in &content.operations {
        let operator = &operation.operator; // e.g., "Tf", "Tj", "Tm"
        let operands = &operation.operands; 


        match operator.as_str() {
            
            // "BT": Begin Text Object. Resets the text matrix.
            "BT" => {
                current_x = 0.0;
                current_y = 0.0;
            }

            // "Tf": Set Text Font and Size.
            "Tf" => {
                if operands.len() >= 2 {
                    if let Ok(size) = operands[1].as_f32() {
                        current_font_size = size;
                    }
                }
            }

            // "Td": Move Text Position.
            "Td" | "TD" => {
                if operands.len() >= 2 {
                    if let (Ok(tx), Ok(ty)) = (operands[0].as_f32(), operands[1].as_f32()) {
                        current_x += tx;
                        current_y += ty;
                    }
                }
            }

            // "Tm": Set Text Matrix (absolute positioning).
            "Tm" => {
                if operands.len() >= 6 {
                    if let (Ok(e), Ok(f)) = (operands[4].as_f32(), operands[5].as_f32()) {
                        current_x = e;
                        current_y = f;
                    }
                }
            }

            // "Tj": Show Text.
            "Tj" => {
                if let Some(text_obj) = operands.first() {
                    let text = extract_text_from_object(text_obj);
                    
                    extracted_items.push(TextItem {
                        text,
                        x: current_x,
                        y: current_y,
                        font_size: current_font_size,
                        page_num: page_num  ,

                    });
                }
            }

            // // "TJ": Show Text with Adjustments (kerning).
            // "TJ" => {
            //     // TJ is complex because it mixes strings and numbers (spacing).
            //     if let Some(Object::Array(arr)) = operands.first() {
            //         let mut combined_text = String::new();
            //         for item in arr {
            //             if let Object::String(bytes, _) = item {

            //                 combined_text.push_str(&String::from_utf8_lossy(bytes));

            //                 // combined_text.push_str(&String::from_utf8(bytes.clone()).unwrap_or_default());
            //             }
            //         }
                    
            //         extracted_items.push(TextItem {
            //             text: combined_text,
            //             x: current_x,
            //             y: current_y,
            //             font_size: current_font_size,
            //             page_num: page_num  ,
            //         });
            //     }
            // }

            _ => {} 
        }
    }
    extracted_items.sort_by(|a, b| {
        b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal)
           .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });

    extracted_items
}

fn extract_text_from_object(obj: &Object) -> String {
    match obj {
        Object::String(bytes, _) => {

            // String::from_utf8_lossy(bytes).to_string()
            String::from_utf8(bytes.clone()).unwrap_or_default()
        },
        _ => String::new(),
    }
}

#[pyfunction]
fn extract_chars_from_pdf(path: String) -> PyResult<Vec<CharItem>> {
    let doc = Document::load(&path).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
    let mut all_chars = Vec::new();

    for (page_num, object_id) in doc.get_pages() {
        if let Ok(content_data) = doc.get_page_content(object_id) {
            if let Ok(content) = lopdf::content::Content::decode(&content_data) {
                let mut chars = process_content_stream_chars(&content, page_num);
                all_chars.append(&mut chars);
            }
        }
    }

    all_chars.sort_by(|a, b| {
        a.page_num.cmp(&b.page_num)
            .then(b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal))
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });

    Ok(all_chars)
}

fn process_content_stream_chars(content: &lopdf::content::Content, page_num: u32) -> Vec<CharItem> {
    let mut extracted_chars = Vec::new();

    // Text state variables
    let mut current_font_size: f32 = 12.0;
    let mut text_matrix: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]; // a, b, c, d, e, f
    let mut line_matrix: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut char_spacing: f32 = 0.0;
    let mut word_spacing: f32 = 0.0;
    let mut horizontal_scaling: f32 = 100.0;
    let mut text_rise: f32 = 0.0;
    
    // Approximate character width (as fraction of font size)
    // In reality, you'd need to read font metrics from the PDF
    let default_char_width_factor: f32 = 0.5; // Approximate for most fonts

    for operation in &content.operations {
        let operator = &operation.operator;
        let operands = &operation.operands;

        match operator.as_str() {
            // Begin Text Object - reset text matrix to identity
            "BT" => {
                text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                line_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
            }

            // Set character spacing
            "Tc" => {
                if let Some(obj) = operands.first() {
                    if let Ok(tc) = obj.as_f32() {
                        char_spacing = tc;
                    }
                }
            }

            // Set word spacing
            "Tw" => {
                if let Some(obj) = operands.first() {
                    if let Ok(tw) = obj.as_f32() {
                        word_spacing = tw;
                    }
                }
            }

            // Set horizontal scaling
            "Tz" => {
                if let Some(obj) = operands.first() {
                    if let Ok(tz) = obj.as_f32() {
                        horizontal_scaling = tz;
                    }
                }
            }

            // Set text rise
            "Ts" => {
                if let Some(obj) = operands.first() {
                    if let Ok(ts) = obj.as_f32() {
                        text_rise = ts;
                    }
                }
            }

            // Set Text Font and Size
            "Tf" => {
                if operands.len() >= 2 {
                    if let Ok(size) = operands[1].as_f32() {
                        current_font_size = size;
                    }
                }
            }

            // Move text position (relative)
            "Td" => {
                if operands.len() >= 2 {
                    if let (Ok(tx), Ok(ty)) = (operands[0].as_f32(), operands[1].as_f32()) {
                        line_matrix[4] += tx;
                        line_matrix[5] += ty;
                        text_matrix = line_matrix;
                    }
                }
            }

            // Move text position and set leading
            "TD" => {
                if operands.len() >= 2 {
                    if let (Ok(tx), Ok(ty)) = (operands[0].as_f32(), operands[1].as_f32()) {
                        line_matrix[4] += tx;
                        line_matrix[5] += ty;
                        text_matrix = line_matrix;
                    }
                }
            }

            // Set text matrix (absolute positioning)
            "Tm" => {
                if operands.len() >= 6 {
                    if let (Ok(a), Ok(b), Ok(c), Ok(d), Ok(e), Ok(f)) = (
                        operands[0].as_f32(),
                        operands[1].as_f32(),
                        operands[2].as_f32(),
                        operands[3].as_f32(),
                        operands[4].as_f32(),
                        operands[5].as_f32(),
                    ) {
                        text_matrix = [a, b, c, d, e, f];
                        line_matrix = text_matrix;
                    }
                }
            }

            // Move to start of next line
            "T*" => {
                // Uses current leading (we'd need to track TL operator for accuracy)
                line_matrix[5] -= current_font_size; // Approximate
                text_matrix = line_matrix;
            }

            // Show text string
            "Tj" => {
                if let Some(Object::String(bytes, _)) = operands.first() {
                    let text = String::from_utf8_lossy(bytes);
                    let scale = horizontal_scaling / 100.0;
                    
                    for ch in text.chars() {
                        let x = text_matrix[4];
                        let y = text_matrix[5] + text_rise;
                        
                        // Estimate character width
                        let char_width = current_font_size * default_char_width_factor * scale;
                        
                        extracted_chars.push(CharItem {
                            char: ch.to_string(),
                            x,
                            y,
                            font_size: current_font_size,
                            page_num,
                            char_width,
                        });
                        
                        // Advance text position
                        let mut advance = char_width + char_spacing;
                        if ch == ' ' {
                            advance += word_spacing;
                        }
                        text_matrix[4] += advance;
                    }
                }
            }

            // Show text with positioning adjustments (kerning)
            "TJ" => {
                if let Some(Object::Array(arr)) = operands.first() {
                    let scale = horizontal_scaling / 100.0;
                    
                    for item in arr {
                        match item {
                            Object::String(bytes, _) => {
                                let text = String::from_utf8_lossy(bytes);
                                
                                for ch in text.chars() {
                                    let x = text_matrix[4];
                                    let y = text_matrix[5] + text_rise;
                                    
                                    let char_width = current_font_size * default_char_width_factor * scale;
                                    
                                    extracted_chars.push(CharItem {
                                        char: ch.to_string(),
                                        x,
                                        y,
                                        font_size: current_font_size,
                                        page_num,
                                        char_width,
                                    });
                                    
                                    let mut advance = char_width + char_spacing;
                                    if ch == ' ' {
                                        advance += word_spacing;
                                    }
                                    text_matrix[4] += advance;
                                }
                            }
                            Object::Integer(n) => {
                                // Negative numbers move right, positive move left
                                // Value is in thousandths of a unit of text space
                                text_matrix[4] -= (*n as f32) * current_font_size / 1000.0 * scale;
                            }
                            Object::Real(n) => {
                                text_matrix[4] -= n * current_font_size / 1000.0 * scale;
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Show text, move to next line
            "'" => {
                line_matrix[5] -= current_font_size;
                text_matrix = line_matrix;
                // Then show text like Tj
                if let Some(Object::String(bytes, _)) = operands.first() {
                    let text = String::from_utf8_lossy(bytes);
                    let scale = horizontal_scaling / 100.0;
                    
                    for ch in text.chars() {
                        let x = text_matrix[4];
                        let y = text_matrix[5] + text_rise;
                        let char_width = current_font_size * default_char_width_factor * scale;
                        
                        extracted_chars.push(CharItem {
                            char: ch.to_string(),
                            x,
                            y,
                            font_size: current_font_size,
                            page_num,
                            char_width,
                        });
                        
                        let mut advance = char_width + char_spacing;
                        if ch == ' ' {
                            advance += word_spacing;
                        }
                        text_matrix[4] += advance;
                    }
                }
            }

            _ => {}
        }
    }

    extracted_chars
}