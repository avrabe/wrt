#![allow(dead_code)]

use std::error::Error;
use std::fs;
use std::path::Path;

pub fn run() -> Result<(), Box<dyn Error>> {
    println!("Generating qualification status report...");

    // Check for qualification materials
    let qualification_materials = [
        ("Evaluation Plan", "docs/source/evaluation_plan.rst"),
        ("Evaluation Report", "docs/source/evaluation_report.rst"),
        ("Qualification Plan", "docs/source/qualification_plan.rst"),
        ("Qualification Report", "docs/source/qualification_report.rst"),
        ("Traceability Matrix", "docs/source/traceability_matrix.rst"),
        ("Document List", "docs/source/document_list.rst"),
        ("Internal Procedures", "docs/source/internal_procedures.rst"),
        ("Technical Report", "docs/source/technical_report.rst"),
        ("Safety Analysis", "docs/source/safety_analysis.rst"),
    ];

    // Print status of each material with color coding
    println!("\n{}", colorize("Qualification Materials Status:", "BOLD"));
    println!("{}", colorize("=================================", "BOLD"));

    let mut not_started = 0;
    let mut partial = 0;
    let mut implemented = 0;

    for (name, primary_path, alternate_path) in
        qualification_materials.map(|(name, path)| (name, path, ""))
    {
        let primary_exists = Path::new(primary_path).exists();
        let alternate_exists = alternate_path.is_empty() || Path::new(alternate_path).exists();
        let exists = primary_exists || alternate_exists;

        let path_to_check = if primary_exists {
            primary_path
        } else if !alternate_path.is_empty() && alternate_exists {
            alternate_path
        } else {
            primary_path
        };

        let status = if exists {
            // For existing files, check for placeholder content
            let content = fs::read_to_string(path_to_check)?;
            if content.contains("TBD") || content.contains("TODO") {
                partial += 1;
                colorize("Partial", "YELLOW")
            } else {
                implemented += 1;
                colorize("Implemented", "GREEN")
            }
        } else {
            not_started += 1;
            colorize("Not Started", "RED")
        };

        println!("{:<20} - {:<25} ({})", name, status, path_to_check);
    }

    // Summarize overall status
    println!("\n{}", colorize("Qualification Progress Summary", "BOLD"));
    println!("{}", colorize("=================================", "BOLD"));

    let total = qualification_materials.len();
    let percentage = (implemented as f32 + (partial as f32 * 0.5)) / (total as f32) * 100.0;

    println!(
        "Implemented:  {}/{} ({:.1}%)",
        implemented,
        total,
        implemented as f32 / total as f32 * 100.0
    );
    println!("Partial:      {}/{}", partial, total);
    println!("Not Started:  {}/{}", not_started, total);

    // Print overall progress with color based on percentage
    let progress_color = if percentage < 30.0 {
        "RED"
    } else if percentage < 70.0 {
        "YELLOW"
    } else {
        "GREEN"
    };

    println!("Overall Progress: {}", colorize(&format!("{:.1}%", percentage), progress_color));

    // Print sphinx-needs related information
    println!("\n{}", colorize("Sphinx-Needs Integration", "BOLD"));
    println!("{}", colorize("======================", "BOLD"));
    println!(
        "Requirement objects:      {}",
        count_objects_in_file("docs/source/requirements.rst", ".. req::")?
    );
    println!(
        "Specification objects:    {}",
        count_objects_in_file("docs/source/architecture.rst", ".. spec::")?
    );
    println!(
        "Implementation objects:   {}",
        count_objects_in_file("docs/source/architecture.rst", ".. impl::")?
    );
    println!(
        "Qualification objects:    {}",
        count_objects_in_file("docs/source/qualification.rst", ".. spec::")?
    );
    println!(
        "Safety objects:           {}",
        count_objects_in_file("docs/source/safety_analysis.rst", ".. safety::")?
    );

    println!("\nFor a detailed assessment, run: cargo xtask qualification assess");
    println!("To generate the traceability matrix: cargo xtask qualification traceability");

    Ok(())
}

fn colorize(text: &str, color: &str) -> String {
    match color {
        "RED" => format!("\x1b[31m{}\x1b[0m", text),
        "GREEN" => format!("\x1b[32m{}\x1b[0m", text),
        "YELLOW" => format!("\x1b[33m{}\x1b[0m", text),
        "BOLD" => format!("\x1b[1m{}\x1b[0m", text),
        _ => text.to_string(),
    }
}

fn count_objects_in_file(file_path: &str, object_prefix: &str) -> Result<String, Box<dyn Error>> {
    if !Path::new(file_path).exists() {
        return Ok(colorize("0 (file not found)", "RED"));
    }

    let content = fs::read_to_string(file_path)?;
    let count = content.lines().filter(|line| line.trim().starts_with(object_prefix)).count();

    if count == 0 {
        Ok(colorize(&count.to_string(), "RED"))
    } else {
        Ok(count.to_string())
    }
}
