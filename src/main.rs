fn main() {
    println!("OTLP String Processing Example");
    
    // Example data simulating OTLP string fields with duplicates
    let raw_data = vec![
        b"service.name".to_vec(),
        b"http.method".to_vec(),
        b"service.name".to_vec(),  // Duplicate
        b"http.status_code".to_vec(),
        b"region".to_vec(),
        b"http.method".to_vec(),   // Another duplicate
        b"trace.id".to_vec(),
    ];

    println!("\nInput: {} raw byte strings (with duplicates)", raw_data.len());
    
    match otlp_string_processing::process_otlp_strings(raw_data) {
        Ok(processed) => {
            println!("\nProcessed strings:");
            for (i, s) in processed.iter().enumerate() {
                println!("  {}: {}", i, s);
            }
            println!("\nTotal processed: {} strings", processed.len());
        }
        Err(e) => eprintln!("Error processing strings: {:?}", e),
    }
}

