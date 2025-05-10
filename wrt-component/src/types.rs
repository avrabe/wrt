//! Types for the WebAssembly Component Model
//!
//! This module provides component model type definitions.

use crate::component::Component;
use crate::prelude::*;
use wrt_error::kinds::InvalidState;

/// Represents an instantiated component
#[derive(Debug)]
pub struct ComponentInstance {
    /// Reference to the component
    component: Arc<Component>,
    /// Instance ID
    id: String,
    /// Instance state
    state: ComponentInstanceState,
}

/// State of a component instance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentInstanceState {
    /// Instance is initialized but not started
    Initialized,
    /// Instance is running
    Running,
    /// Instance is paused
    Paused,
    /// Instance has been stopped or exited
    Stopped,
    /// Instance encountered an error
    Error,
}

impl Default for ComponentInstanceState {
    fn default() -> Self {
        Self::Initialized
    }
}

impl ComponentInstance {
    /// Create a new component instance
    pub fn new(component: Arc<Component>, id: &str) -> Self {
        Self { component, id: id.to_string(), state: ComponentInstanceState::Initialized }
    }

    /// Get the component reference
    pub fn component(&self) -> &Arc<Component> {
        &self.component
    }

    /// Get the instance ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the instance state
    pub fn state(&self) -> ComponentInstanceState {
        self.state
    }

    /// Set the instance state
    pub fn set_state(&mut self, state: ComponentInstanceState) {
        self.state = state;
    }

    /// Start the component instance
    pub fn start(&mut self) -> Result<()> {
        if self.state == ComponentInstanceState::Initialized
            || self.state == ComponentInstanceState::Paused
        {
            self.state = ComponentInstanceState::Running;
            Ok(())
        } else {
            Err(Error::new(
                ErrorCategory::Component,
                codes::INVALID_STATE,
                InvalidState("Component is not in a startable state".to_string()),
            ))
        }
    }

    /// Pause the component instance
    pub fn pause(&mut self) -> Result<()> {
        if self.state == ComponentInstanceState::Running {
            self.state = ComponentInstanceState::Paused;
            Ok(())
        } else {
            Err(Error::new(
                ErrorCategory::Component,
                codes::INVALID_STATE,
                InvalidState("Component is not running".to_string()),
            ))
        }
    }

    /// Stop the component instance
    pub fn stop(&mut self) -> Result<()> {
        if self.state != ComponentInstanceState::Stopped
            && self.state != ComponentInstanceState::Error
        {
            self.state = ComponentInstanceState::Stopped;
            Ok(())
        } else {
            Err(Error::new(
                ErrorCategory::Component,
                codes::INVALID_STATE,
                InvalidState("Component is already stopped or in error state".to_string()),
            ))
        }
    }
}
