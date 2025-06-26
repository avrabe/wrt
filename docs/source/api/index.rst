API Documentation
=================

This section contains the API documentation for all PulseEngine libraries and components.

.. warning::
   **Development Status**: Many APIs shown here represent the intended design. 
   Implementation status varies by component - see individual crate documentation for details.

.. note::
   The following references are automatically generated during the complete documentation build process.
   Missing references are normal if you're viewing a partial build without Rust documentation generation.

.. toctree::
   :maxdepth: 2
   :caption: Core Libraries:

   wrt-error <../_generated_rust_docs/wrt-error/lib>
   wrt-foundation <../_generated_rust_docs/wrt-foundation/lib>
   wrt-sync <../_generated_rust_docs/wrt-sync/lib>
   wrt-math <../_generated_rust_docs/wrt-math/lib>
   wrt-helper <../_generated_rust_docs/wrt-helper/lib>

.. toctree::
   :maxdepth: 2
   :caption: Format and Parsing:

   wrt-format <../_generated_rust_docs/wrt-format/lib>
   wrt-decoder <../_generated_rust_docs/wrt-decoder/lib>

.. toctree::
   :maxdepth: 2
   :caption: Runtime and Execution:

   wrt-instructions <../_generated_rust_docs/wrt-instructions/lib>

.. toctree::
   :maxdepth: 2
   :caption: Platform Support:

   wrt-platform <../_generated_rust_docs/wrt-platform/lib>

.. toctree::
   :maxdepth: 2
   :caption: Host Integration:

   wrt-host <../_generated_rust_docs/wrt-host/lib>
   wrt-intercept <../_generated_rust_docs/wrt-intercept/lib>
   wrt-logging <../_generated_rust_docs/wrt-logging/lib>

.. note::
   Additional crate documentation will be enabled progressively as we resolve 
   build dependencies and improve the rust documentation generation pipeline.
   
   Planned additions:
   - wrt-foundation (core types and collections)
   - wrt-runtime (execution engine)
   - wrt-component (Component Model implementation)
   - wrt-platform (platform abstraction)
   - wrt-decoder (binary format parsing)
   - And more... 