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

// fn print_with_layout(obj: &Object) -> String{


// }