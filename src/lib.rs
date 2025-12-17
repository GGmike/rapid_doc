use lopdf::{Document, Object};
use std::env;
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
}

// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let args: Vec<String> = env::args().collect();
//     if args.len() < 2 {
//         eprintln!("Usage: {} <path_to_pdf>", args[0]);
//         std::process::exit(1);
//     }
//     let path = &args[1];

//     let doc = Document::load(path)?;

//     for (page_num, object_id) in doc.get_pages() {
//         println!("Processing Page {}", page_num);
//         let content_data = doc.get_page_content(object_id)?;
//         let content = lopdf::content::Content::decode(&content_data)?;

//         // println!("Content Operations:");
//         // for operation in &content.operations {
//         //     println!("  Operator: {}, Operands: {:?}", 
//         //         operation.operator, 
//         //         operation.operands.iter().map(|op| print_with_layout(op)).collect::<Vec<String>>());
//         // }

//         let mut text_items = process_content_stream(&content);
//         // text_items.sort_by(|a, b| {
//         //     b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal)
//         //     .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
//         //     });
//         for item in text_items {
//             println!("  Found: '{:?}' at ({:.2}, {:.2}) size {:.2}", 
//                 item.text, item.x, item.y, item.font_size);
//         }
//     }

//     Ok(())
// }

#[pyfunction]
fn extract_text_from_pdf(path: String) -> PyResult<Vec<TextItem>> {
    let doc = Document::load(path).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
    let mut all_items = Vec::new();

    for (_page_num, object_id) in doc.get_pages() {
        if let Ok(content_data) = doc.get_page_content(object_id) {
            if let Ok(content) = lopdf::content::Content::decode(&content_data) {
                let mut items = process_content_stream(&content);
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
fn rapid_pdf(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<TextItem>()?;
    m.add_function(wrap_pyfunction!(extract_text_from_pdf, m)?)?;
    Ok(())
}

fn process_content_stream(content: &lopdf::content::Content) -> Vec<TextItem> {
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
                    });
                }
            }

            // "TJ": Show Text with Adjustments (kerning).
            "TJ" => {
                // TJ is complex because it mixes strings and numbers (spacing).
                if let Some(Object::Array(arr)) = operands.first() {
                    let mut combined_text = String::new();
                    for item in arr {
                        if let Object::String(bytes, _) = item {

                            combined_text.push_str(&String::from_utf8_lossy(bytes));

                            // combined_text.push_str(&String::from_utf8(bytes.clone()).unwrap_or_default());
                        }
                    }
                    
                    extracted_items.push(TextItem {
                        text: combined_text,
                        x: current_x,
                        y: current_y,
                        font_size: current_font_size,
                    });
                }
            }

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