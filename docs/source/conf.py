import os
import sys
import platform
import json
import subprocess
sys.path.insert(0, os.path.abspath('../..'))

project = 'WRT'
copyright = '2024, WRT Contributors'
author = 'WRT Contributors'
release = '0.1.0'

# Version configuration
# Read current version from environment or default to 'main'
current_version = os.environ.get('DOCS_VERSION', 'main')
version_path_prefix = os.environ.get('DOCS_VERSION_PATH_PREFIX', '/wrt')

# Function to get available versions
def get_versions():
    versions = ['main']
    try:
        # Get all tags
        result = subprocess.run(['git', 'tag'], stdout=subprocess.PIPE, universal_newlines=True)
        if result.returncode == 0:
            # Only include semantic version tags (x.y.z)
            import re
            tags = result.stdout.strip().split('\n')
            for tag in tags:
                if re.match(r'^\d+\.\d+\.\d+$', tag):
                    versions.append(tag)
    except Exception as e:
        print(f"Error getting versions: {e}")
    
    return sorted(versions, key=lambda v: v if v == 'main' else [int(x) for x in v.split('.')])

# Available versions for the switcher
versions = get_versions()

# Write versions data for the index page to use for redirection
versions_data = {
    'current_version': current_version,
    'versions': versions,
    'version_path_prefix': version_path_prefix
}

# Ensure _static directory exists
os.makedirs(os.path.join(os.path.dirname(__file__), '_static'), exist_ok=True)

# Write versions data to a JSON file
with open(os.path.join(os.path.dirname(__file__), '_static', 'versions.json'), 'w') as f:
    json.dump(versions_data, f)

# Add version data to the context for templates
html_context = {
    'current_version': current_version,
    'versions': versions,
    'version_path_prefix': version_path_prefix
}

# Custom monkeypatch to handle NoneType in names
def setup(app):
    from sphinx.domains.std import StandardDomain
    old_process_doc = StandardDomain.process_doc
    
    def patched_process_doc(self, env, docname, document):
        try:
            return old_process_doc(self, env, docname, document)
        except TypeError as e:
            if "'NoneType' object is not subscriptable" in str(e):
                print(f"WARNING: Caught TypeError in {docname}. This indicates a node with missing 'names' attribute.")
                return
            raise
    
    StandardDomain.process_doc = patched_process_doc
    
    # Add our custom CSS
    app.add_css_file('css/custom.css')
    
    return {'version': '0.1', 'parallel_read_safe': True}

extensions = [
    'sphinx.ext.autodoc',
    'sphinx.ext.viewcode',
    'sphinx.ext.napoleon',
    'sphinx_needs',  # Temporarily disabled to focus on diagrams
    'myst_parser',
    'sphinxcontrib.plantuml',
    "sphinxcontrib_rust",
]

templates_path = ['_templates']
exclude_patterns = []

# Change theme from sphinx_book_theme to pydata_sphinx_theme
html_theme = 'pydata_sphinx_theme'
html_static_path = ['_static']

# Configure theme options
html_theme_options = {
    # Configure the version switcher
    "switcher": {
        "json_url": f"{version_path_prefix}switcher.json",
        "version_match": current_version,
    },
    # Place the version switcher in the navbar
    "navbar_start": ["navbar-logo", "version-switcher"],
    # Test configuration - disable in production
    "check_switcher": False,
    # Control navigation bar behavior
    "navbar_align": "content",
    "use_navbar_nav_drop_shadow": True,
    # Control the sidebar navigation
    "navigation_with_keys": True,
    "show_nav_level": 1,
    "show_toc_level": 2,
    # Only show in the sidebar the current page's TOC
    "collapse_navigation": True,
    "show_prev_next": True,
}

# PlantUML configuration
# Using the installed plantuml executable
plantuml = 'plantuml'
plantuml_output_format = 'svg'
plantuml_latex_output_format = 'pdf'

# Make PlantUML work cross-platform
if platform.system() == "Windows":
    # Windows may need the full path to the plantuml.jar or plantuml.bat
    plantuml = os.environ.get('PLANTUML_PATH', 'plantuml')
elif platform.system() == "Darwin":  # macOS
    # macOS typically uses Homebrew installation
    plantuml = os.environ.get('PLANTUML_PATH', 'plantuml')
    # Add debug info
    print(f"PlantUML path on macOS: {plantuml}")
    print(f"PlantUML exists: {os.path.exists(plantuml) if os.path.isabs(plantuml) else 'checking PATH'}")
elif platform.system() == "Linux":
    # Linux installation path
    plantuml = os.environ.get('PLANTUML_PATH', 'plantuml')

# Allow customization through environment variables
plantuml_output_format = os.environ.get('PLANTUML_FORMAT', 'svg')

# Sphinx-needs configuration
needs_types = [
    dict(directive="req", title="Requirement", prefix="REQ_", color="#BFD8D2", style="node"),
    dict(directive="spec", title="Specification", prefix="SPEC_", color="#FEDCD2", style="node"),
    dict(directive="impl", title="Implementation", prefix="IMPL_", color="#DF744A", style="node"),
    dict(directive="test", title="Test Case", prefix="T_", color="#DCB239", style="node"),
    dict(directive="safety", title="Safety", prefix="SAFETY_", color="#FF5D73", style="node"),
    dict(directive="qual", title="Qualification", prefix="QUAL_", color="#9370DB", style="node"),
    dict(directive="constraint", title="Constraint", prefix="CNST_", color="#4682B4", style="node"),
    dict(directive="panic", title="Panic", prefix="WRTQ_", color="#E74C3C", style="node"),
]

# Add ID regex pattern for sphinx-needs
needs_id_regex = '^[A-Z0-9_]{5,}$'

# Add option specs to register additional options for directives
needs_extra_options = [
    'rationale', 
    'verification', 
    'mitigation', 
    'implementation', 
    'safety_impact',
    'item_status',
    'handling_strategy',
    'last_updated'
]

# Allow all sphinx-needs options for all directives
needs_allow_unsafe_options = True

# Disable warnings for unknown link targets to avoid the many outgoing link warnings
needs_warnings_always_warn = False

# Custom sphinx-needs templates for qualification and safety
needs_templates = {
    'safety_template': '**Hazard**: {{content}}\n\n**Mitigation**: {{mitigation}}',
    'qualification_template': '**Status**: {{item_status}}\n\n**Implementation**: {{implementation}}',
    'constraint_template': '**Constraint**: {{content}}\n\n**Rationale**: {{rationale}}\n\n**Verification**: {{verification}}',
    'panic_template': '**Panic Condition**: {{content}}\n\n**Safety Impact**: {{safety_impact}}\n\n**Status**: {{item_status}}\n\n**Handling Strategy**: {{handling_strategy}}',
}

# Tags for filtering and displaying panic entries
needs_tags = [
    dict(name="panic", description="Panic documentation entry", bgcolor="#E74C3C"),
    dict(name="low", description="Low safety impact", bgcolor="#2ECC71"),
    dict(name="medium", description="Medium safety impact", bgcolor="#F39C12"),
    dict(name="high", description="High safety impact", bgcolor="#E74C3C"),
    dict(name="unknown", description="Unknown safety impact", bgcolor="#95A5A6"),
]

# Configure needs roles for referencing 
needs_role_need_template = "{title} ({id})"
needs_role_need_max_title_length = 30

needs_id_length = 7
needs_title_optional = True
needs_file_pattern = '**/*.rst'

# Additional debug settings for sphinx-needs
needs_debug_processing = True
needs_debug_event_handler = True

source_suffix = {
    ".rst": "restructuredtext",
    ".md": "markdown",
    ".txt": "markdown", # Optional
}

# See docs/compatibility for details on these extensions.
myst_enable_extensions = {
    "attrs_block",
    "colon_fence",
    "html_admonition",
    "replacements",
    "smartquotes",
    "strikethrough",
    "tasklist",
}
# Rust documentation configuration
rust_crates = {
    "wrt-error": os.path.abspath("../../wrt-error"),
    "wrt": os.path.abspath("../../wrt"),
    "wrt-sync": os.path.abspath("../../wrt-sync"),
    "wrt-format": os.path.abspath("../../wrt-format"),
    "wrt-decoder": os.path.abspath("../../wrt-decoder"),
    "wrt-common": os.path.abspath("../../wrt-common"),
    "wrt-component": os.path.abspath("../../wrt-component"),
    "wrt-host": os.path.abspath("../../wrt-host"),
    "wrt-instructions": os.path.abspath("../../wrt-instructions"),
    "wrt-intercept": os.path.abspath("../../wrt-intercept"),
    "wrt-logging": os.path.abspath("../../wrt-logging"),
    "wrt-runtime": os.path.abspath("../../wrt-runtime"),
    "wrt-types": os.path.abspath("../../wrt-types"),
    "wrtd": os.path.abspath("../../wrtd"),
}
rust_doc_dir = "docs/source/"
rust_rustdoc_fmt = "md"