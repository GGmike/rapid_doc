use lopdf::{Document, Object};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;


#[derive(Debug, Clone)] 
struct TextItem {
    
    text: String,
    
    x: f32,
    
    y: f32,
    
    font_size: f32,
    page_num: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::load("test.pdf")?;
    
    // doc.replace_text(1, "Company", "", None)?;
    // doc.replace_text(1, "Statement", " Testing ", None)?;
    // doc.replace_text(1, "Company Statement of Changes in Shareholders’ Equity", " Testing ", None)?;
    // doc.replace_text(1, "Company Statement of Changes in Shareholders’ Equity", " Testing ", None)?;
    replace_text_at_position(
        &mut doc,
        1,
        "CORPORATE",
        "Testing",
        277.895, 451.84802,
        0.00,   
    )?;
    let mut all_text_items = Vec::new();
    
    for (page_num, object_id) in doc.get_pages(){
        println!("Processing Page {}", page_num);
        let content_data = doc.get_page_content(object_id)?;
        let content = lopdf::content::Content::decode(&content_data)?;
        
        let mut text_items = process_content_stream(&content, page_num);
        
        text_items.sort_by(|a, b| {
            b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal)
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        });
        for item in &text_items {
            println!("  Found: '{:?}' at ({:.2}, {:.2}) size {:.2}", 
                item.text, item.x, item.y, item.font_size);
        }
        all_text_items.extend(text_items);
    }
    all_text_items.sort_by(|a, b| {
        b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal)
        .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });
    println!("Print with layout preserved:");
    let mut last_test_y = all_text_items.first().unwrap().y;
    for item in all_text_items{
        if (last_test_y - item.y).abs() > 5.0 {
            println!();
            last_test_y = item.y;
        }
        print!("{} ", item.text.trim());
    }
    // let path = Path::new("output.pdf");
    // let mut file = BufWriter::new(File::create(path)?);
    // doc.save_modern(&mut file)?;


    Ok(())
}

fn extract_text_from_pdf(path: String) -> Result<Vec<TextItem>, Box<dyn std::error::Error>> {
    let doc = Document::load(path)?;
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

fn process_content_stream(content: &lopdf::content::Content, page_num: u32) -> Vec<TextItem> {
    let mut extracted_items = Vec::new();
    let mut extracted_larget_set :Vec<TextItem> = Vec::new();
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
                    println!("Extracted text (Tj): {}", text);
                    extracted_larget_set.push(TextItem {
                        text: text.clone(),
                        x: current_x,
                        y: current_y,
                        font_size: current_font_size,
                        page_num
                    });
                    for(i, ch) in text.chars().enumerate(){
                        let char_x = current_x + (i as f32 * current_font_size * 0.6); // Approximate width (adjust factor as needed)
                        extracted_items.push(TextItem {
                            text: ch.to_string(),
                            x: char_x,
                            y: current_y,
                            font_size: current_font_size,
                            page_num,
                        });
                        println!("  Char '{}' at ({:.2}, {:.2})", ch, char_x, current_y);
                    }
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
            //         println!("Extracted Combined text (TJ): {}", combined_text);
                    
            //         extracted_items.push(TextItem {
            //             text: combined_text,
            //             x: current_x,
            //             y: current_y,
            //             font_size: current_font_size,
            //             page_num,
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


fn replace_text_at_position(
    doc: &mut Document,
    page_num: u32,
    target_text: &str,
    replacement: &str,
    target_x: f32,
    target_y: f32,
    target_font_size: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    let pages = doc.get_pages();
    println!("Replacing text {}", target_text);
    if let Some(&object_id) = pages.get(&page_num) {
        let content_data = doc.get_page_content(object_id)?;
        let mut content = lopdf::content::Content::decode(&content_data)?;
        
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
                        if current_x == target_x && current_y == target_y {
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
                // "TJ" => {
                //     // For TJ, combine text and check if it matches
                //     if let Some(Object::Array(arr)) = operands.first() {
                //         let mut combined_text = String::new();
                //         for item in arr {
                //             if let Object::String(bytes, _) = item {
                //                 combined_text.push_str(&String::from_utf8_lossy(bytes));
                //             }
                //         }
                //         if combined_text == target_text
                //             && (current_x - target_x).abs() < 0.01
                //             && (current_y - target_y).abs() < 0.01
                //             // && (current_font_size - target_font_size).abs() < 0.01 
                //             {
                //             // Replace the entire array with a single string (simplified; assumes no kerning adjustments)
                //             operation.operands[0] = Object::String(replacement.as_bytes().to_vec(), lopdf::StringFormat::Literal);
                //             println!("Replaced text at ({}, {})", current_x, current_y);
                //             break;
                //         }
                //     }
                // }
                _ => {}
            }
        }
        
        let new_content_data = content.encode()?;
        doc.change_page_content(object_id, new_content_data)?;
    }
    let path = Path::new("output.pdf");
    let mut file = BufWriter::new(File::create(path)?);
    doc.save_modern(&mut file)?;
    Ok(())
}

