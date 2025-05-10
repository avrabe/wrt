#![allow(dead_code)]

use std::error::Error;
use std::fs;
use std::path::Path;

use crate::qualification::{parse_requirements, parse_specifications, Relationship};

pub fn run(output_path: &Path, format: &str) -> Result<(), Box<dyn Error>> {
    println!("Generating traceability matrix in {} format...", format);

    // Read requirements from requirements.rst
    let requirements_content = match fs::read_to_string("docs/source/requirements.rst") {
        Ok(content) => content,
        Err(_) => {
            println!("Warning: Could not read requirements.rst. Creating placeholder file.");
            String::new()
        }
    };

    // Parse requirements and their attributes
    let requirements = parse_requirements(&requirements_content);

    // Read specifications from architecture.rst
    let architecture_content = match fs::read_to_string("docs/source/architecture.rst") {
        Ok(content) => content,
        Err(_) => {
            println!("Warning: Could not read architecture.rst. Creating placeholder file.");
            String::new()
        }
    };

    // Parse specifications and their attributes
    let specifications = parse_specifications(&architecture_content);

    // Read qualification specifications
    let qualification_content = match fs::read_to_string("docs/source/qualification.rst") {
        Ok(content) => content,
        Err(_) => {
            println!("Warning: Could not read qualification.rst. Creating placeholder file.");
            String::new()
        }
    };

    // Parse qualification specifications
    let qualification_specs = parse_specifications(&qualification_content);

    // Build relationships (placeholder for now)
    let relationships = build_relationships(&requirements, &specifications, &qualification_specs);

    // Generate output based on format
    let output_content = match format {
        "rst" => generate_rst_traceability(&relationships),
        "md" => generate_md_traceability(&relationships),
        "csv" => generate_csv_traceability(&relationships),
        _ => return Err(format!("Unsupported format: {}", format).into()),
    };

    // Ensure output directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write output to file
    fs::write(output_path, output_content)?;

    println!("Traceability matrix generated at: {}", output_path.display());
    Ok(())
}

fn build_relationships(
    _requirements: &[crate::qualification::Requirement],
    _specifications: &[crate::qualification::Specification],
    _qualification_specs: &[crate::qualification::Specification],
) -> Vec<crate::qualification::Relationship> {
    // This would build the relationships between requirements and specifications
    // Placeholder implementation
    vec![Relationship { requirement: "REQ_001".to_string(), specification: "SPEC_001".to_string() }]
}

fn generate_rst_traceability(relationships: &[Relationship]) -> String {
    let mut content = String::from("Traceability Matrix\n=================\n\n");

    content.push_str(
        "This traceability matrix maps requirements to specifications and implementations.\n\n",
    );

    content.push_str(".. list-table:: Requirement to Specification Mapping\n");
    content.push_str("   :widths: 30 70\n");
    content.push_str("   :header-rows: 1\n\n");

    content.push_str("   * - Requirement\n     - Specifications\n");

    // Group relationships by requirement
    let mut req_to_specs: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for rel in relationships {
        req_to_specs.entry(rel.requirement.clone()).or_default().push(rel.specification.clone());
    }

    // Sort by requirement ID
    let mut reqs: Vec<&String> = req_to_specs.keys().collect();
    reqs.sort();

    // Output each requirement with its specifications
    for req in reqs {
        if let Some(specs) = req_to_specs.get(req) {
            let specs_str = specs.join(", ");
            content.push_str(&format!("   * - {}\n     - {}\n", req, specs_str));
        }
    }

    content
}

fn generate_md_traceability(relationships: &[Relationship]) -> String {
    let mut content = String::from("# Traceability Matrix\n\n");

    content.push_str(
        "This traceability matrix maps requirements to specifications and implementations.\n\n",
    );

    content.push_str("| Requirement | Specifications |\n");
    content.push_str("|------------|----------------|\n");

    // Group relationships by requirement
    let mut req_to_specs: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for rel in relationships {
        req_to_specs.entry(rel.requirement.clone()).or_default().push(rel.specification.clone());
    }

    // Sort by requirement ID
    let mut reqs: Vec<&String> = req_to_specs.keys().collect();
    reqs.sort();

    // Output each requirement with its specifications
    for req in reqs {
        if let Some(specs) = req_to_specs.get(req) {
            let specs_str = specs.join(", ");
            content.push_str(&format!("| {} | {} |\n", req, specs_str));
        }
    }

    content
}

fn generate_csv_traceability(relationships: &[Relationship]) -> String {
    let mut content = String::from("Requirement,Specification\n");

    for rel in relationships {
        content.push_str(&format!("{},{}\n", rel.requirement, rel.specification));
    }

    content
}
