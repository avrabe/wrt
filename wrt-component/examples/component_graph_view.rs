use std::env;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io;

use wrt_component::component::{
    AliasInfo, Component, ComponentSummary, CoreInstanceInfo, CoreModuleInfo, ExtendedExportInfo,
    ExtendedImportInfo, ModuleExportInfo, ModuleImportInfo,
};
use wrt_decoder::ProducersSection;
use wrt_error::{Error, Result};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, StatefulWidget, Widget};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};
use tui_nodes::{Connection, LineType, NodeGraph, NodeLayout};
use wrt_format::module::ImportDesc;

/// Main state struct for the application
struct App {
    component_summary: ComponentSummary,
    imports: Vec<ExtendedImportInfo>,
    exports: Vec<ExtendedExportInfo>,
    module_imports: Vec<ModuleImportInfo>,
    module_exports: Vec<ModuleExportInfo>,
    producers_info: Vec<ProducersSection>,
    selected_view: SelectedView,
    focused_node: Option<usize>,
    mode: ViewMode,
    debug_mode: bool,
    component_bytes: Option<Vec<u8>>,
}

/// Enum to represent different views
#[derive(Debug, Clone, Copy, PartialEq, Display, EnumIter, FromRepr)]
enum SelectedView {
    Overview,
    Modules,
    Imports,
    Exports,
    Producers,
    Details,
    Debug,
}

/// Enum to represent different view modes
#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    Normal,
    Focused,
}

/// Wrapper widget around NodeGraph that implements Widget
struct NodeGraphWidget<'a>(NodeGraph<'a>);

impl<'a> Widget for NodeGraphWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = ();
        StatefulWidget::render(self.0, area, buf, &mut state);
    }
}

impl App {
    /// Create a new App with the given component analysis
    fn new(
        component_summary: ComponentSummary,
        imports: Vec<ExtendedImportInfo>,
        exports: Vec<ExtendedExportInfo>,
        module_imports: Vec<ModuleImportInfo>,
        module_exports: Vec<ModuleExportInfo>,
        producers_info: Vec<ProducersSection>,
        debug_mode: bool,
        component_bytes: Option<Vec<u8>>,
    ) -> Self {
        Self {
            component_summary,
            imports,
            exports,
            module_imports,
            module_exports,
            producers_info,
            selected_view: SelectedView::Overview,
            focused_node: None,
            mode: ViewMode::Normal,
            debug_mode,
            component_bytes,
        }
    }

    /// Build a node graph for the current view
    fn build_node_graph(&self) -> NodeGraph<'static> {
        // Calculate a reasonable size based on assumed terminal dimensions
        let width = 120;
        let height = 40;

        match self.selected_view {
            SelectedView::Overview => self.build_overview_graph(width, height),
            SelectedView::Modules => self.build_modules_graph(width, height),
            SelectedView::Imports => self.build_imports_graph(width, height),
            SelectedView::Exports => self.build_exports_graph(width, height),
            SelectedView::Producers => self.build_producers_graph(width, height),
            SelectedView::Details => self.build_details_graph(width, height),
            SelectedView::Debug => self.build_debug_graph(width, height),
        }
    }

    /// Build the overview graph showing main component sections
    fn build_overview_graph(&self, width: u16, height: u16) -> NodeGraph<'static> {
        // Create static strings first
        let component_title = format!("Component: {}", self.component_summary.name);
        let modules_title = format!("Modules ({})", self.component_summary.core_modules_count);
        let instances_title =
            format!("Instances ({})", self.component_summary.core_instances_count);
        let imports_title = format!("Imports ({})", self.component_summary.imports_count);
        let exports_title = format!("Exports ({})", self.component_summary.exports_count);
        let aliases_title = format!("Aliases ({})", self.component_summary.aliases_count);
        let producers_title = format!("Producers ({})", self.producers_info.len());

        // Create static string references
        let component_title_static = Box::leak(component_title.into_boxed_str());
        let modules_title_static = Box::leak(modules_title.into_boxed_str());
        let instances_title_static = Box::leak(instances_title.into_boxed_str());
        let imports_title_static = Box::leak(imports_title.into_boxed_str());
        let exports_title_static = Box::leak(exports_title.into_boxed_str());
        let aliases_title_static = Box::leak(aliases_title.into_boxed_str());
        let producers_title_static = Box::leak(producers_title.into_boxed_str());

        let mut nodes = Vec::new();
        let mut connections = Vec::new();

        // Create the root component node
        let component_node = nodes.len();
        nodes.push(
            NodeLayout::new((30, 5))
                .with_title(component_title_static)
                .with_border_style(Style::default().fg(Color::Yellow)),
        );

        // Create nodes for main sections
        let modules_node = nodes.len();
        nodes.push(
            NodeLayout::new((25, 4))
                .with_title(modules_title_static)
                .with_border_style(Style::default().fg(Color::Cyan)),
        );

        let instances_node = nodes.len();
        nodes.push(
            NodeLayout::new((25, 4))
                .with_title(instances_title_static)
                .with_border_style(Style::default().fg(Color::Cyan)),
        );

        let imports_node = nodes.len();
        nodes.push(
            NodeLayout::new((25, 4))
                .with_title(imports_title_static)
                .with_border_style(Style::default().fg(Color::Green)),
        );

        let exports_node = nodes.len();
        nodes.push(
            NodeLayout::new((25, 4))
                .with_title(exports_title_static)
                .with_border_style(Style::default().fg(Color::Green)),
        );

        let aliases_node = nodes.len();
        nodes.push(
            NodeLayout::new((25, 4))
                .with_title(aliases_title_static)
                .with_border_style(Style::default().fg(Color::Magenta)),
        );

        let producers_node = nodes.len();
        nodes.push(
            NodeLayout::new((25, 4))
                .with_title(producers_title_static)
                .with_border_style(Style::default().fg(Color::Blue)),
        );

        // Connect nodes to the root
        connections.push(
            Connection::new(component_node, 0, modules_node, 0)
                .with_line_style(Style::default().fg(Color::White)),
        );

        connections.push(
            Connection::new(component_node, 0, instances_node, 0)
                .with_line_style(Style::default().fg(Color::White)),
        );

        connections.push(
            Connection::new(component_node, 0, imports_node, 0)
                .with_line_style(Style::default().fg(Color::White)),
        );

        connections.push(
            Connection::new(component_node, 0, exports_node, 0)
                .with_line_style(Style::default().fg(Color::White)),
        );

        connections.push(
            Connection::new(component_node, 0, aliases_node, 0)
                .with_line_style(Style::default().fg(Color::White)),
        );

        connections.push(
            Connection::new(component_node, 0, producers_node, 0)
                .with_line_style(Style::default().fg(Color::White)),
        );

        // Create and calculate the node graph with the specified dimensions
        let mut graph = NodeGraph::new(nodes, connections, width as usize, height as usize);
        graph.calculate();
        graph
    }

    /// Build the modules graph showing modules and their relationships
    fn build_modules_graph(&self, width: u16, height: u16) -> NodeGraph<'static> {
        let mut nodes = Vec::new();
        let mut connections = Vec::new();

        if self.component_summary.core_modules.is_empty() {
            let empty_title = "No modules found".to_string();
            let static_title = Box::leak(empty_title.into_boxed_str());
            nodes.push(
                NodeLayout::new((25, 4))
                    .with_title(static_title)
                    .with_border_style(Style::default().fg(Color::Red)),
            );
            let mut graph = NodeGraph::new(nodes, connections, width as usize, height as usize);
            graph.calculate();
            return graph;
        }

        // Create a node for each module
        for (idx, module) in self.component_summary.core_modules.iter().enumerate() {
            let module_title = format!("Module {} ({}b)", module.idx, module.size);
            let static_title = Box::leak(module_title.into_boxed_str());

            // Create a content string with module information
            let import_count =
                self.module_imports.iter().filter(|imp| imp.module_idx == module.idx).count();
            let export_count =
                self.module_exports.iter().filter(|exp| exp.module_idx == module.idx).count();
            let content = format!(
                "Size: {} bytes\nImports: {}\nExports: {}",
                module.size, import_count, export_count
            );
            let static_content = Box::leak(content.into_boxed_str());

            let module_node = nodes.len();
            nodes.push(
                NodeLayout::new((30, 6))
                    .with_title(static_title)
                    .with_border_style(Style::default().fg(Color::Cyan)),
            );

            // If this is a focused view on a specific module, add more details
            if self.mode == ViewMode::Focused
                && self.focused_node.is_some()
                && self.focused_node.unwrap() == idx
            {
                // Find all imports for this module
                let module_imports: Vec<_> =
                    self.module_imports.iter().filter(|imp| imp.module_idx == module.idx).collect();

                // Find all exports for this module
                let module_exports: Vec<_> =
                    self.module_exports.iter().filter(|exp| exp.module_idx == module.idx).collect();

                // Add import nodes
                for (i, import) in module_imports.iter().enumerate() {
                    let import_title = format!("Import: {}.{}", import.module, import.name);
                    let static_title = Box::leak(import_title.into_boxed_str());

                    // Add import details as content
                    let import_content = format!("Type: {}", import.kind);
                    let static_import_content = Box::leak(import_content.into_boxed_str());

                    let import_node = nodes.len();
                    nodes.push(
                        NodeLayout::new((30, 5))
                            .with_title(static_title)
                            .with_border_style(Style::default().fg(Color::Green)),
                    );

                    connections.push(
                        Connection::new(import_node, 0, module_node, i)
                            .with_line_style(Style::default().fg(Color::White)),
                    );
                }

                // Add export nodes
                for (i, export) in module_exports.iter().enumerate() {
                    let export_title = format!("Export: {}", export.name);
                    let static_title = Box::leak(export_title.into_boxed_str());

                    // Add export details as content
                    let export_content = format!("Type: {}", export.kind);
                    let static_export_content = Box::leak(export_content.into_boxed_str());

                    let export_node = nodes.len();
                    nodes.push(
                        NodeLayout::new((30, 5))
                            .with_title(static_title)
                            .with_border_style(Style::default().fg(Color::Blue)),
                    );

                    connections.push(
                        Connection::new(module_node, i, export_node, 0)
                            .with_line_style(Style::default().fg(Color::White)),
                    );
                }
            }
        }

        let mut graph = NodeGraph::new(nodes, connections, width as usize, height as usize);
        graph.calculate();
        graph
    }

    /// Build the imports graph showing component imports
    fn build_imports_graph(&self, width: u16, height: u16) -> NodeGraph<'static> {
        let mut nodes = Vec::new();
        let mut connections = Vec::new();

        if self.imports.is_empty() {
            let empty_title = "No imports found".to_string();
            let static_title = Box::leak(empty_title.into_boxed_str());
            nodes.push(
                NodeLayout::new((25, 4))
                    .with_title(static_title)
                    .with_border_style(Style::default().fg(Color::Red)),
            );
            let mut graph = NodeGraph::new(nodes, connections, width as usize, height as usize);
            graph.calculate();
            return graph;
        }

        // Create a center node for the component
        let component_title = format!("Component: {}", self.component_summary.name);
        let static_component_title = Box::leak(component_title.into_boxed_str());

        let component_content = format!("Total imports: {}", self.imports.len());
        let static_component_content = Box::leak(component_content.into_boxed_str());

        let component_node = nodes.len();
        nodes.push(
            NodeLayout::new((30, 5))
                .with_title(static_component_title)
                .with_border_style(Style::default().fg(Color::Yellow)),
        );

        // Group imports by namespace
        let mut namespaces: Vec<&str> = self
            .imports
            .iter()
            .filter_map(|imp| {
                if !imp.name.is_empty() {
                    let parts: Vec<&str> = imp.name.splitn(2, '.').collect();
                    if parts.len() > 1 {
                        Some(parts[0])
                    } else {
                        Some("default")
                    }
                } else {
                    Some("default")
                }
            })
            .collect::<std::collections::HashSet<&str>>()
            .into_iter()
            .collect();
        namespaces.sort();

        // Create nodes for namespaces and imports
        for (ns_idx, namespace) in namespaces.iter().enumerate() {
            let ns_title = format!("Namespace: {}", namespace);
            let static_ns_title = Box::leak(ns_title.into_boxed_str());

            // Count imports in this namespace
            let namespace_str = *namespace;
            let ns_imports_count = self
                .imports
                .iter()
                .filter(|imp| {
                    if !imp.name.is_empty() {
                        let parts: Vec<&str> = imp.name.splitn(2, '.').collect();
                        if parts.len() > 1 {
                            parts[0] == namespace_str
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .count();

            let ns_content = format!("Import count: {}", ns_imports_count);
            let static_ns_content = Box::leak(ns_content.into_boxed_str());

            let ns_node = nodes.len();
            nodes.push(
                NodeLayout::new((25, 4))
                    .with_title(static_ns_title)
                    .with_border_style(Style::default().fg(Color::Magenta)),
            );

            connections.push(
                Connection::new(component_node, ns_idx, ns_node, 0)
                    .with_line_style(Style::default().fg(Color::White)),
            );

            // Add import nodes for this namespace
            let namespace_str = *namespace;
            let ns_imports: Vec<&ExtendedImportInfo> = self
                .imports
                .iter()
                .filter(|imp| {
                    if !imp.name.is_empty() {
                        let parts: Vec<&str> = imp.name.splitn(2, '.').collect();
                        if parts.len() > 1 {
                            parts[0] == namespace_str
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .collect();

            for (imp_idx, import) in ns_imports.iter().enumerate() {
                let import_title = format!(
                    "{}: {}",
                    import.name,
                    import.kind.split('(').next().unwrap_or(&import.kind)
                );
                let static_import_title = Box::leak(import_title.into_boxed_str());

                // Add import type information as content
                let import_content = format!("Type: {}", import.kind);
                let static_import_content = Box::leak(import_content.into_boxed_str());

                let import_node = nodes.len();
                nodes.push(
                    NodeLayout::new((30, 4))
                        .with_title(static_import_title)
                        .with_border_style(Style::default().fg(Color::Green)),
                );

                connections.push(
                    Connection::new(import_node, 0, ns_node, imp_idx)
                        .with_line_style(Style::default().fg(Color::White)),
                );
            }
        }

        let mut graph = NodeGraph::new(nodes, connections, width as usize, height as usize);
        graph.calculate();
        graph
    }

    /// Build the exports graph showing component exports
    fn build_exports_graph(&self, width: u16, height: u16) -> NodeGraph<'static> {
        let mut nodes = Vec::new();
        let mut connections = Vec::new();

        if self.exports.is_empty() {
            let empty_title = "No exports found".to_string();
            let static_title = Box::leak(empty_title.into_boxed_str());
            nodes.push(
                NodeLayout::new((25, 4))
                    .with_title(static_title)
                    .with_border_style(Style::default().fg(Color::Red)),
            );
            let mut graph = NodeGraph::new(nodes, connections, width as usize, height as usize);
            graph.calculate();
            return graph;
        }

        // Create a center node for the component
        let component_title = format!("Component: {}", self.component_summary.name);
        let static_component_title = Box::leak(component_title.into_boxed_str());

        let component_content = format!("Total exports: {}", self.exports.len());
        let static_component_content = Box::leak(component_content.into_boxed_str());

        let component_node = nodes.len();
        nodes.push(
            NodeLayout::new((30, 5))
                .with_title(static_component_title)
                .with_border_style(Style::default().fg(Color::Yellow)),
        );

        // Group exports by namespace
        let mut namespaces: Vec<&str> = self
            .exports
            .iter()
            .filter_map(|exp| {
                if !exp.name.is_empty() {
                    let parts: Vec<&str> = exp.name.splitn(2, '.').collect();
                    if parts.len() > 1 {
                        Some(parts[0])
                    } else {
                        Some("default")
                    }
                } else {
                    Some("default")
                }
            })
            .collect::<std::collections::HashSet<&str>>()
            .into_iter()
            .collect();
        namespaces.sort();

        // Create nodes for namespaces and exports
        for (ns_idx, namespace) in namespaces.iter().enumerate() {
            let ns_title = format!("Namespace: {}", namespace);
            let static_ns_title = Box::leak(ns_title.into_boxed_str());

            // Count exports in this namespace
            let namespace_str = *namespace;
            let ns_exports_count = self
                .exports
                .iter()
                .filter(|exp| {
                    if !exp.name.is_empty() {
                        let parts: Vec<&str> = exp.name.splitn(2, '.').collect();
                        if parts.len() > 1 {
                            parts[0] == namespace_str
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .count();

            let ns_content = format!("Export count: {}", ns_exports_count);
            let static_ns_content = Box::leak(ns_content.into_boxed_str());

            let ns_node = nodes.len();
            nodes.push(
                NodeLayout::new((25, 4))
                    .with_title(static_ns_title)
                    .with_border_style(Style::default().fg(Color::Magenta)),
            );

            connections.push(
                Connection::new(component_node, ns_idx, ns_node, 0)
                    .with_line_style(Style::default().fg(Color::White)),
            );

            // Add export nodes for this namespace
            let namespace_str = *namespace;
            let ns_exports: Vec<&ExtendedExportInfo> = self
                .exports
                .iter()
                .filter(|exp| {
                    if !exp.name.is_empty() {
                        let parts: Vec<&str> = exp.name.splitn(2, '.').collect();
                        if parts.len() > 1 {
                            parts[0] == namespace_str
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .collect();

            for (exp_idx, export) in ns_exports.iter().enumerate() {
                let export_title = format!(
                    "{}: {}",
                    export.name,
                    export.kind.split('(').next().unwrap_or(&export.kind)
                );
                let static_export_title = Box::leak(export_title.into_boxed_str());

                // Add export type information as content
                let export_content = format!("Type: {}", export.kind);
                let static_export_content = Box::leak(export_content.into_boxed_str());

                let export_node = nodes.len();
                nodes.push(
                    NodeLayout::new((30, 4))
                        .with_title(static_export_title)
                        .with_border_style(Style::default().fg(Color::Blue)),
                );

                connections.push(
                    Connection::new(ns_node, exp_idx, export_node, 0)
                        .with_line_style(Style::default().fg(Color::White)),
                );
            }
        }

        let mut graph = NodeGraph::new(nodes, connections, width as usize, height as usize);
        graph.calculate();
        graph
    }

    /// Build the producers graph showing producer metadata
    fn build_producers_graph(&self, width: u16, height: u16) -> NodeGraph<'static> {
        let mut nodes = Vec::new();
        let mut connections = Vec::new();

        if self.producers_info.is_empty() {
            let empty_title = "No producers found".to_string();
            let static_title = Box::leak(empty_title.into_boxed_str());
            nodes.push(
                NodeLayout::new((25, 4))
                    .with_title(static_title)
                    .with_border_style(Style::default().fg(Color::Red)),
            );
            let mut graph = NodeGraph::new(nodes, connections, width as usize, height as usize);
            graph.calculate();
            return graph;
        }

        // Create a center node for the component
        let component_title = format!("Component: {}", self.component_summary.name);
        let static_component_title = Box::leak(component_title.into_boxed_str());

        let component_content = format!("Producer sections: {}", self.producers_info.len());
        let static_component_content = Box::leak(component_content.into_boxed_str());

        let component_node = nodes.len();
        nodes.push(
            NodeLayout::new((30, 5))
                .with_title(static_component_title)
                .with_border_style(Style::default().fg(Color::Yellow)),
        );

        // Create nodes for each producer section
        for (prod_idx, producer) in self.producers_info.iter().enumerate() {
            let producer_title = format!("Module #{} Producers", prod_idx);
            let static_producer_title = Box::leak(producer_title.into_boxed_str());

            // Build a summary content string
            let mut content = String::new();
            if !producer.languages.is_empty() {
                content.push_str(&format!("Languages: {}\n", producer.languages.len()));
            }
            if !producer.processed_by.is_empty() {
                content.push_str(&format!("Tools: {}\n", producer.processed_by.len()));
            }
            if !producer.sdks.is_empty() {
                content.push_str(&format!("SDKs: {}", producer.sdks.len()));
            }

            let static_content = Box::leak(content.into_boxed_str());

            let producer_node = nodes.len();
            nodes.push(
                NodeLayout::new((25, 5))
                    .with_title(static_producer_title)
                    .with_border_style(Style::default().fg(Color::Blue)),
            );

            connections.push(
                Connection::new(component_node, prod_idx, producer_node, 0)
                    .with_line_style(Style::default().fg(Color::White)),
            );

            // If in focused mode and this producer is selected, show details
            if self.mode == ViewMode::Focused
                && self.focused_node.is_some()
                && self.focused_node.unwrap() == prod_idx
            {
                // Create language nodes
                if !producer.languages.is_empty() {
                    let languages_title = "Languages".to_string();
                    let static_lang_title = Box::leak(languages_title.into_boxed_str());

                    // Create a content string with all languages
                    let mut lang_content = String::new();
                    for language in &producer.languages {
                        lang_content
                            .push_str(&format!("{} ({})\n", language.name, language.version));
                    }
                    let static_lang_content = Box::leak(lang_content.into_boxed_str());

                    let lang_node = nodes.len();
                    nodes.push(
                        NodeLayout::new((30, 5 + producer.languages.len() as u16))
                            .with_title(static_lang_title)
                            .with_border_style(Style::default().fg(Color::Green)),
                    );

                    connections.push(
                        Connection::new(producer_node, 0, lang_node, 0)
                            .with_line_style(Style::default().fg(Color::White)),
                    );
                }

                // Create tools nodes
                if !producer.processed_by.is_empty() {
                    let tools_title = "Tools".to_string();
                    let static_tools_title = Box::leak(tools_title.into_boxed_str());

                    // Create a content string with all tools
                    let mut tools_content = String::new();
                    for tool in &producer.processed_by {
                        tools_content.push_str(&format!("{} ({})\n", tool.name, tool.version));
                    }
                    let static_tools_content = Box::leak(tools_content.into_boxed_str());

                    let tools_node = nodes.len();
                    nodes.push(
                        NodeLayout::new((30, 5 + producer.processed_by.len() as u16))
                            .with_title(static_tools_title)
                            .with_border_style(Style::default().fg(Color::Cyan)),
                    );

                    connections.push(
                        Connection::new(producer_node, 1, tools_node, 0)
                            .with_line_style(Style::default().fg(Color::White)),
                    );
                }

                // Create SDKs nodes
                if !producer.sdks.is_empty() {
                    let sdks_title = "SDKs".to_string();
                    let static_sdks_title = Box::leak(sdks_title.into_boxed_str());

                    // Create a content string with all SDKs
                    let mut sdks_content = String::new();
                    for sdk in &producer.sdks {
                        sdks_content.push_str(&format!("{} ({})\n", sdk.name, sdk.version));
                    }
                    let static_sdks_content = Box::leak(sdks_content.into_boxed_str());

                    let sdks_node = nodes.len();
                    nodes.push(
                        NodeLayout::new((30, 5 + producer.sdks.len() as u16))
                            .with_title(static_sdks_title)
                            .with_border_style(Style::default().fg(Color::Magenta)),
                    );

                    connections.push(
                        Connection::new(producer_node, 2, sdks_node, 0)
                            .with_line_style(Style::default().fg(Color::White)),
                    );
                }
            }
        }

        let mut graph = NodeGraph::new(nodes, connections, width as usize, height as usize);
        graph.calculate();
        graph
    }

    /// Build the details graph showing detailed information about a specific entity
    fn build_details_graph(&self, width: u16, height: u16) -> NodeGraph<'static> {
        let mut nodes = Vec::new();
        let mut connections = Vec::new();

        // Create the root component node
        let component_title = format!("Component: {}", self.component_summary.name);
        let static_title = Box::leak(component_title.into_boxed_str());

        // Build component details content
        let content = format!(
            "Modules: {}\nInstances: {}\nImports: {}\nExports: {}\nAliases: {}",
            self.component_summary.core_modules_count,
            self.component_summary.core_instances_count,
            self.component_summary.imports_count,
            self.component_summary.exports_count,
            self.component_summary.aliases_count
        );
        let static_content = Box::leak(content.into_boxed_str());

        let root_node = nodes.len();
        nodes.push(
            NodeLayout::new((30, 8))
                .with_title(static_title)
                .with_border_style(Style::default().fg(Color::Yellow)),
        );

        // Add module nodes
        if !self.component_summary.core_modules.is_empty() {
            let modules_title = format!("Modules ({})", self.component_summary.core_modules_count);
            let static_modules_title = Box::leak(modules_title.into_boxed_str());

            let modules_node = nodes.len();
            nodes.push(
                NodeLayout::new((25, 5))
                    .with_title(static_modules_title)
                    .with_border_style(Style::default().fg(Color::Cyan)),
            );

            connections.push(
                Connection::new(root_node, 0, modules_node, 0)
                    .with_line_style(Style::default().fg(Color::White)),
            );

            // Add individual module nodes
            for (i, module) in self.component_summary.core_modules.iter().enumerate() {
                let module_title = format!("Module #{}", module.idx);
                let static_module_title = Box::leak(module_title.into_boxed_str());

                // Find imports and exports for this module
                let import_count =
                    self.module_imports.iter().filter(|imp| imp.module_idx == module.idx).count();

                let export_count =
                    self.module_exports.iter().filter(|exp| exp.module_idx == module.idx).count();

                let module_content = format!(
                    "Size: {} bytes\nImports: {}\nExports: {}",
                    module.size, import_count, export_count
                );
                let static_module_content = Box::leak(module_content.into_boxed_str());

                let module_node = nodes.len();
                nodes.push(
                    NodeLayout::new((25, 6))
                        .with_title(static_module_title)
                        .with_border_style(Style::default().fg(Color::Cyan)),
                );

                connections.push(
                    Connection::new(modules_node, i, module_node, 0)
                        .with_line_style(Style::default().fg(Color::White)),
                );
            }
        }

        // Add instance nodes
        if !self.component_summary.core_instances.is_empty() {
            let instances_title =
                format!("Instances ({})", self.component_summary.core_instances_count);
            let static_instances_title = Box::leak(instances_title.into_boxed_str());

            let instances_node = nodes.len();
            nodes.push(
                NodeLayout::new((25, 5))
                    .with_title(static_instances_title)
                    .with_border_style(Style::default().fg(Color::Cyan)),
            );

            connections.push(
                Connection::new(root_node, 1, instances_node, 0)
                    .with_line_style(Style::default().fg(Color::White)),
            );

            // Add individual instance nodes
            for (i, instance) in self.component_summary.core_instances.iter().enumerate() {
                let instance_title = format!("Instance #{}", i);
                let static_instance_title = Box::leak(instance_title.into_boxed_str());

                let instance_content = format!("Module idx: {}", instance.module_idx);
                let static_instance_content = Box::leak(instance_content.into_boxed_str());

                let instance_node = nodes.len();
                nodes.push(
                    NodeLayout::new((25, 4))
                        .with_title(static_instance_title)
                        .with_border_style(Style::default().fg(Color::Cyan)),
                );

                connections.push(
                    Connection::new(instances_node, i, instance_node, 0)
                        .with_line_style(Style::default().fg(Color::White)),
                );
            }
        }

        // Add imports node
        if !self.imports.is_empty() {
            let imports_title = format!("Imports ({})", self.imports.len());
            let static_imports_title = Box::leak(imports_title.into_boxed_str());

            let imports_node = nodes.len();
            nodes.push(
                NodeLayout::new((25, 5))
                    .with_title(static_imports_title)
                    .with_border_style(Style::default().fg(Color::Green)),
            );

            connections.push(
                Connection::new(root_node, 2, imports_node, 0)
                    .with_line_style(Style::default().fg(Color::White)),
            );
        }

        // Add exports node
        if !self.exports.is_empty() {
            let exports_title = format!("Exports ({})", self.exports.len());
            let static_exports_title = Box::leak(exports_title.into_boxed_str());

            let exports_node = nodes.len();
            nodes.push(
                NodeLayout::new((25, 5))
                    .with_title(static_exports_title)
                    .with_border_style(Style::default().fg(Color::Blue)),
            );

            connections.push(
                Connection::new(root_node, 3, exports_node, 0)
                    .with_line_style(Style::default().fg(Color::White)),
            );
        }

        let mut graph = NodeGraph::new(nodes, connections, width as usize, height as usize);
        graph.calculate();
        graph
    }

    /// Build the debug graph showing raw information about a component
    fn build_debug_graph(&self, width: u16, height: u16) -> NodeGraph<'static> {
        let mut nodes = Vec::new();
        let mut connections = Vec::new();

        if !self.debug_mode {
            let title = "Debug Mode Disabled".to_string();
            let static_title = Box::leak(title.into_boxed_str());

            let content = "Run with --debug flag to enable debug information";
            // Fix for invalid String to str conversion
            let content_string = content.to_string();
            let static_content = Box::leak(content_string.into_boxed_str());

            nodes.push(
                NodeLayout::new((40, 5))
                    .with_title(static_title)
                    .with_border_style(Style::default().fg(Color::Red)),
            );

            let mut graph = NodeGraph::new(nodes, connections, width as usize, height as usize);
            graph.calculate();
            return graph;
        }

        // Create root debug node
        let debug_title = "Debug Information".to_string();
        let static_debug_title = Box::leak(debug_title.into_boxed_str());

        // Create detailed debug content
        let component_info = format!(
            "Component file size: {}\nModules: {}\nInstances: {}\nImports: {}\nExports: {}\nAliases: {}",
            self.component_bytes.as_ref().map_or(0, |b| b.len()),
            self.component_summary.core_modules_count,
            self.component_summary.core_instances_count,
            self.component_summary.imports_count,
            self.component_summary.exports_count,
            self.component_summary.aliases_count
        );
        let static_component_info = Box::leak(component_info.into_boxed_str());

        let debug_node = nodes.len();
        nodes.push(
            NodeLayout::new((40, 8))
                .with_title(static_debug_title)
                .with_border_style(Style::default().fg(Color::Yellow)),
        );

        // Add modules debug node
        if !self.component_summary.core_modules.is_empty() {
            let modules_title = "Core Modules Debug".to_string();
            let static_modules_title = Box::leak(modules_title.into_boxed_str());

            // Create content with module details
            let mut modules_content = String::new();
            for (i, module) in self.component_summary.core_modules.iter().enumerate().take(5) {
                modules_content.push_str(&format!(
                    "Module #{}: idx={}, size={}b\n",
                    i, module.idx, module.size
                ));
            }
            if self.component_summary.core_modules.len() > 5 {
                modules_content.push_str(&format!(
                    "...and {} more modules",
                    self.component_summary.core_modules.len() - 5
                ));
            }
            let static_modules_content = Box::leak(modules_content.into_boxed_str());

            let modules_node = nodes.len();
            nodes.push(
                NodeLayout::new((40, 8))
                    .with_title(static_modules_title)
                    .with_border_style(Style::default().fg(Color::Cyan)),
            );

            connections.push(
                Connection::new(debug_node, 0, modules_node, 0)
                    .with_line_style(Style::default().fg(Color::White)),
            );
        }

        // Add imports debug node
        if !self.imports.is_empty() {
            let imports_title = "Component Imports Debug".to_string();
            let static_imports_title = Box::leak(imports_title.into_boxed_str());

            // Create content with import details
            let mut imports_content = String::new();
            for (i, import) in self.imports.iter().enumerate().take(5) {
                imports_content
                    .push_str(&format!("Import #{}: {} ({})\n", i, import.name, import.kind));
            }
            if self.imports.len() > 5 {
                imports_content
                    .push_str(&format!("...and {} more imports", self.imports.len() - 5));
            }
            let static_imports_content = Box::leak(imports_content.into_boxed_str());

            let imports_node = nodes.len();
            nodes.push(
                NodeLayout::new((40, 8))
                    .with_title(static_imports_title)
                    .with_border_style(Style::default().fg(Color::Green)),
            );

            connections.push(
                Connection::new(debug_node, 1, imports_node, 0)
                    .with_line_style(Style::default().fg(Color::White)),
            );
        }

        // Add exports debug node
        if !self.exports.is_empty() {
            let exports_title = "Component Exports Debug".to_string();
            let static_exports_title = Box::leak(exports_title.into_boxed_str());

            // Create content with export details
            let mut exports_content = String::new();
            for (i, export) in self.exports.iter().enumerate().take(5) {
                exports_content
                    .push_str(&format!("Export #{}: {} ({})\n", i, export.name, export.kind));
            }
            if self.exports.len() > 5 {
                exports_content
                    .push_str(&format!("...and {} more exports", self.exports.len() - 5));
            }
            let static_exports_content = Box::leak(exports_content.into_boxed_str());

            let exports_node = nodes.len();
            nodes.push(
                NodeLayout::new((40, 8))
                    .with_title(static_exports_title)
                    .with_border_style(Style::default().fg(Color::Blue)),
            );

            connections.push(
                Connection::new(debug_node, 2, exports_node, 0)
                    .with_line_style(Style::default().fg(Color::White)),
            );
        }

        let mut graph = NodeGraph::new(nodes, connections, width as usize, height as usize);
        graph.calculate();
        graph
    }

    /// Handle user input and update state
    fn handle_input(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Char('1') => {
                self.selected_view = SelectedView::Overview;
                self.mode = ViewMode::Normal;
            }
            KeyCode::Char('2') => {
                self.selected_view = SelectedView::Modules;
                self.mode = ViewMode::Normal;
            }
            KeyCode::Char('3') => {
                self.selected_view = SelectedView::Imports;
                self.mode = ViewMode::Normal;
            }
            KeyCode::Char('4') => {
                self.selected_view = SelectedView::Exports;
                self.mode = ViewMode::Normal;
            }
            KeyCode::Char('5') => {
                self.selected_view = SelectedView::Producers;
                self.mode = ViewMode::Normal;
            }
            KeyCode::Char('6') => {
                self.selected_view = SelectedView::Details;
            }
            KeyCode::Char('7') => {
                if self.debug_mode {
                    self.selected_view = SelectedView::Debug;
                }
            }
            KeyCode::Char('f') => {
                if self.mode == ViewMode::Normal {
                    self.mode = ViewMode::Focused;
                    // Set a default focused node if none is selected
                    if self.focused_node.is_none() {
                        self.focused_node = Some(0);
                    }
                } else {
                    self.mode = ViewMode::Normal;
                }
            }
            KeyCode::Tab | KeyCode::Right => {
                let current_idx = self.selected_view as usize;
                let next_idx = (current_idx + 1)
                    % if self.debug_mode {
                        SelectedView::iter().count()
                    } else {
                        SelectedView::iter().count() - 1 // Skip Debug view when not in debug mode
                    };
                self.selected_view =
                    SelectedView::from_repr(next_idx).unwrap_or(self.selected_view);
            }
            KeyCode::BackTab | KeyCode::Left => {
                let current_idx = self.selected_view as usize;
                let max_idx = if self.debug_mode {
                    SelectedView::iter().count()
                } else {
                    SelectedView::iter().count() - 1 // Skip Debug view when not in debug mode
                };
                let prev_idx = if current_idx == 0 { max_idx - 1 } else { current_idx - 1 };
                self.selected_view =
                    SelectedView::from_repr(prev_idx).unwrap_or(self.selected_view);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // If we have a focused node, move to the next one
                if let Some(current) = self.focused_node {
                    let max_idx = match self.selected_view {
                        SelectedView::Modules => self.component_summary.core_modules.len(),
                        SelectedView::Imports => self.imports.len(),
                        SelectedView::Exports => self.exports.len(),
                        SelectedView::Producers => self.producers_info.len(),
                        _ => 0,
                    };

                    if max_idx > 0 {
                        self.focused_node = Some((current + 1) % max_idx);
                    }
                } else if match self.selected_view {
                    SelectedView::Modules => !self.component_summary.core_modules.is_empty(),
                    SelectedView::Imports => !self.imports.is_empty(),
                    SelectedView::Exports => !self.exports.is_empty(),
                    SelectedView::Producers => !self.producers_info.is_empty(),
                    _ => false,
                } {
                    // No node focused yet, focus the first one
                    self.focused_node = Some(0);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                // If we have a focused node, move to the previous one
                if let Some(current) = self.focused_node {
                    let max_idx = match self.selected_view {
                        SelectedView::Modules => self.component_summary.core_modules.len(),
                        SelectedView::Imports => self.imports.len(),
                        SelectedView::Exports => self.exports.len(),
                        SelectedView::Producers => self.producers_info.len(),
                        _ => 0,
                    };

                    if max_idx > 0 {
                        self.focused_node =
                            Some(if current == 0 { max_idx - 1 } else { current - 1 });
                    }
                } else if match self.selected_view {
                    SelectedView::Modules => !self.component_summary.core_modules.is_empty(),
                    SelectedView::Imports => !self.imports.is_empty(),
                    SelectedView::Exports => !self.exports.is_empty(),
                    SelectedView::Producers => !self.producers_info.is_empty(),
                    _ => false,
                } {
                    // No node focused yet, focus the last one
                    let max_idx = match self.selected_view {
                        SelectedView::Modules => self.component_summary.core_modules.len(),
                        SelectedView::Imports => self.imports.len(),
                        SelectedView::Exports => self.exports.len(),
                        SelectedView::Producers => self.producers_info.len(),
                        _ => 0,
                    };

                    if max_idx > 0 {
                        self.focused_node = Some(max_idx - 1);
                    }
                }
            }
            KeyCode::Enter => {
                if self.selected_view != SelectedView::Details && self.focused_node.is_some() {
                    self.selected_view = SelectedView::Details;
                }
            }
            _ => {}
        }

        true
    }

    /// Run the application event loop
    fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        let mut running = true;

        while running {
            terminal.draw(|frame| self.render(frame))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    running = self.handle_input(key.code);
                }
            }
        }

        Ok(())
    }

    /// Render the application UI
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Create a layout with a title area and main content area
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Content
                Constraint::Length(3), // Status bar
            ])
            .split(area);

        // Render title
        let title_text = format!("WASM Component Graph View - {}", self.selected_view);
        let title = Paragraph::new(title_text)
            .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);

        frame.render_widget(title, chunks[0]);

        // Render node graph in the main area
        let graph_area = chunks[1];

        // Build the graph for the current view
        // Calculate appropriate graph size based on available area
        let graph_width = graph_area.width.saturating_sub(4).max(80);
        let graph_height = graph_area.height.saturating_sub(2).max(20);

        // Use the computed width and height when creating the graph
        let nodes_connections = self.build_node_graph();
        // Instead of resizing an existing graph, create it with the right size

        frame.render_widget(NodeGraphWidget(nodes_connections), graph_area);

        // Render status bar
        let mut status_text = match self.selected_view {
            SelectedView::Overview =>
                "1-6: Switch Views | Tab/←→: Navigate Views | f: Focus Mode | q: Quit".to_string(),
            SelectedView::Modules =>
                "1-6: Switch Views | Tab/←→: Navigate Views | f: Focus Mode | ↑↓: Select Module | Enter: Details | q: Quit".to_string(),
            SelectedView::Imports =>
                "1-6: Switch Views | Tab/←→: Navigate Views | f: Focus Mode | ↑↓: Select Import | Enter: Details | q: Quit".to_string(),
            SelectedView::Exports =>
                "1-6: Switch Views | Tab/←→: Navigate Views | f: Focus Mode | ↑↓: Select Export | Enter: Details | q: Quit".to_string(),
            SelectedView::Producers =>
                "1-6: Switch Views | Tab/←→: Navigate Views | f: Focus Mode | ↑↓: Select Producer | Enter: View Producer Details (Languages, Tools, SDKs) | q: Quit".to_string(),
            SelectedView::Details =>
                "1-6: Switch Views | Tab/←→: Navigate Views | q: Quit".to_string(),
            SelectedView::Debug =>
                "1-7: Switch Views | Tab/←→: Navigate Views | q: Quit".to_string(),
        };

        // Modify status bar text to include debug key when debug mode is enabled
        if self.debug_mode && self.selected_view != SelectedView::Debug {
            status_text = format!("{} | 7: Debug", status_text);
        }

        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);

        frame.render_widget(status, chunks[2]);
    }
}

/// Initialize the terminal for TUI rendering
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, io::Error> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    Terminal::new(CrosstermBackend::new(stdout))
}

/// Restore the terminal to its original state
fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), io::Error> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()
}

/// Prints a hexdump of a byte slice for debugging
fn hexdump(bytes: &[u8], max_bytes: usize) {
    let bytes_to_show = std::cmp::min(bytes.len(), max_bytes);
    const BYTES_PER_LINE: usize = 16;

    let mut offset = 0;

    while offset < bytes_to_show {
        let end = std::cmp::min(offset + BYTES_PER_LINE, bytes_to_show);
        let line_bytes = &bytes[offset..end];

        // Print offset
        print!("{:08x}  ", offset);

        // Print hex values
        let mut hex_string = String::new();
        for (i, &byte) in line_bytes.iter().enumerate() {
            write!(hex_string, "{:02x} ", byte).unwrap();
            if i == 7 {
                hex_string.push(' '); // extra space in the middle
            }
        }

        // Pad hex output to align ASCII view
        if line_bytes.len() < BYTES_PER_LINE {
            for _ in 0..(BYTES_PER_LINE - line_bytes.len()) {
                write!(hex_string, "   ").unwrap();
            }
            if line_bytes.len() <= 8 {
                hex_string.push(' '); // adjust for the middle space
            }
        }

        print!("{}", hex_string);

        // Print ASCII representation
        print!(" |");
        for &byte in line_bytes {
            if byte >= 32 && byte <= 126 {
                print!("{}", byte as char);
            } else {
                print!(".");
            }
        }

        // Pad ASCII view if needed
        for _ in 0..(BYTES_PER_LINE - line_bytes.len()) {
            print!(" ");
        }

        println!("|");

        offset += BYTES_PER_LINE;
    }

    if bytes.len() > max_bytes {
        println!("... ({} more bytes not shown)", bytes.len() - max_bytes);
    }
}

fn main() -> io::Result<()> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    // Check for debug flag
    let debug_mode = args.iter().any(|arg| arg == "--debug" || arg == "-d");

    // Check for the required component path argument
    if args.len() < 2 || (args.len() == 2 && debug_mode) {
        println!("Usage: component_graph_view [--debug | -d] <component_path>");
        return Ok(());
    }

    // Find the component path (first argument that's not a flag or the program name)
    let component_path = match args
        .iter()
        .find(|arg| !arg.starts_with('-') && !arg.ends_with("component_graph_view"))
    {
        Some(path) => path,
        None => {
            println!("Error: No component path provided");
            println!("Usage: component_graph_view [--debug | -d] <component_path>");
            return Ok(());
        }
    };

    println!("Reading component from '{}'...", component_path);

    // Read the component file
    let component_bytes = match fs::read(component_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            println!("Error reading component file: {}", e);
            return Ok(());
        }
    };

    println!("Component size: {} bytes", component_bytes.len());

    if debug_mode {
        // Print hexdump of first part of the file
        println!("Hexdump of first 64 bytes:");
        hexdump(&component_bytes, 64);
    }

    // Check for Wasm/Component magic bytes
    if component_bytes.len() >= 8 {
        let magic = &component_bytes[0..4];
        let version = &component_bytes[4..8];

        if magic == [0x00, 0x61, 0x73, 0x6D] {
            // \0asm
            if version == [0x01, 0x00, 0x00, 0x00] {
                println!("Detected WebAssembly Module (version 1)");
            } else if version == [0x0D, 0x00, 0x01, 0x00] {
                println!("Detected WebAssembly Component (version 1)");
            } else {
                println!("Detected WebAssembly binary with unknown version: {:?}", version);
            }
        } else {
            println!("Warning: File does not have WebAssembly magic bytes");
            println!("First 8 bytes: {:?}", &component_bytes[0..8]);
        }
    }

    println!("Analyzing component...");

    // Try to analyze the component
    let analysis_result =
        wrt_component::component::Component::analyze_component_extended(&component_bytes);

    // Setup terminal
    let mut terminal = match setup_terminal() {
        Ok(term) => term,
        Err(e) => {
            println!("Error setting up terminal: {}", e);
            return Ok(());
        }
    };

    // Create and run the app
    let result = match analysis_result {
        Ok((summary, imports, exports, module_imports, module_exports)) => {
            println!("Component analysis successful.");

            if debug_mode {
                println!("Summary: {}", summary);
                println!(
                    "Found {} imports, {} exports, {} module imports, {} module exports",
                    imports.len(),
                    exports.len(),
                    module_imports.len(),
                    module_exports.len()
                );

                // Print information about modules
                println!("Core modules: {}", summary.core_modules.len());
                for module in &summary.core_modules {
                    println!("  Module {}: {} bytes", module.idx, module.size);
                }
            }

            println!("Starting UI...");

            // Extract producers info directly
            let mut producers_info = Vec::new();

            // Try to extract embedded modules from the component binary
            if let Ok(modules) =
                wrt_component::component::extract_embedded_modules(&component_bytes)
            {
                if debug_mode {
                    println!("Found {} embedded WebAssembly module(s)", modules.len());

                    for (idx, module_binary) in modules.iter().enumerate() {
                        println!("Module #{}: {} bytes", idx, module_binary.len());

                        // Try to decode each module
                        match wrt_decoder::decode(module_binary) {
                            Ok(decoded_module) => {
                                println!("  Successfully decoded module #{}", idx);

                                // Extract producers section if present
                                match wrt_decoder::extract_producers_section(&decoded_module) {
                                    Ok(Some(producers)) => {
                                        println!("  Found producers section in module #{}", idx);
                                        println!("  Languages: {}", producers.languages.len());
                                        for lang in &producers.languages {
                                            println!("    - {} ({})", lang.name, lang.version);
                                        }

                                        println!("  Tools: {}", producers.processed_by.len());
                                        for tool in &producers.processed_by {
                                            println!("    - {} ({})", tool.name, tool.version);
                                        }

                                        println!("  SDKs: {}", producers.sdks.len());
                                        for sdk in &producers.sdks {
                                            println!("    - {} ({})", sdk.name, sdk.version);
                                        }

                                        producers_info.push(producers);
                                    }
                                    Ok(None) => {
                                        println!("  No producers section found in module #{}", idx);
                                    }
                                    Err(e) => {
                                        println!("  Error extracting producers section from module #{}: {}", idx, e);
                                    }
                                }
                            }
                            Err(e) => {
                                println!("  Failed to decode module #{}: {}", idx, e);
                            }
                        }
                    }
                } else {
                    // In non-debug mode, just extract producers without detailed logging
                    for module_binary in &modules {
                        if let Ok(decoded_module) = wrt_decoder::decode(module_binary) {
                            if let Ok(Some(producers)) =
                                wrt_decoder::extract_producers_section(&decoded_module)
                            {
                                producers_info.push(producers);
                            }
                        }
                    }
                }
            } else if debug_mode {
                println!("No embedded WebAssembly modules found in component");
            }

            if debug_mode {
                if producers_info.is_empty() {
                    println!("No producer information found in any module");
                } else {
                    println!("Found producer information in {} module(s)", producers_info.len());
                }
            }

            let mut app = App::new(
                summary,
                imports,
                exports,
                module_imports,
                module_exports,
                producers_info,
                debug_mode,
                Some(component_bytes),
            );
            app.run(&mut terminal)
        }
        Err(e) => {
            println!("Error extracting component info: {}", e);
            restore_terminal(&mut terminal)?;
            return Ok(());
        }
    };

    // Restore terminal
    let _ = restore_terminal(&mut terminal);

    result
}
