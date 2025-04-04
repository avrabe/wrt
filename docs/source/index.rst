Welcome to WRT!
===============

.. toctree::
   :maxdepth: 2
   :caption: Contents:

   requirements
   architecture
   changelog
   wrt/lib
   wrtd/main

Overview
--------

WRT is project that implements a complete WebAssembly runtime with support for:
- Bare-metal environments
- Bounded execution
- State migration
- Comprehensive instrumentation
- Certification support
- WebAssembly Component Model Preview 2
- WASI logging

The project follows strict requirements for safety, performance, and portability, making it suitable for embedded systems and safety-critical applications.

Key Features:
-------------

- **WebAssembly Core Implementation**: Complete implementation of the WebAssembly Core specification
- **Component Model Support**: Full implementation of WebAssembly Component Model Preview 2, including resource types and interface types
- **Platform-Specific Logging**: Native integration with system logging
- **Safety Features**: Stackless design, bounded execution, and state migration capabilities
- **Certification Support**: Comprehensive instrumentation for safety-critical applications

For detailed requirements and their relationships, see the :doc:`requirements` section.
