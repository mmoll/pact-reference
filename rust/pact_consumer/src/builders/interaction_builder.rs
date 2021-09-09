use log::debug;
use maplit::hashmap;
use serde_json::{json, Value};

use pact_models::provider_states::ProviderState;
use pact_models::sync_interaction::RequestResponseInteraction;
use pact_models::v4::synch_http::SynchronousHttp;

use super::request_builder::RequestBuilder;
use super::response_builder::ResponseBuilder;
use std::collections::HashMap;

/// Builder for `Interaction` objects. Normally created via
/// `PactBuilder::interaction`.
#[derive(Clone, Debug)]
pub struct InteractionBuilder {
    description: String,
    provider_states: Vec<ProviderState>,
    comments: Vec<String>,
    test_name: Option<String>,

    /// A builder for this interaction's `Request`.
    pub request: RequestBuilder,

    /// A builder for this interaction's `Response`.
    pub response: ResponseBuilder,
    /// The interaction type (as stored in the plugin catalogue)
    pub interaction_type: String,
}

impl InteractionBuilder {
  /// Create a new interaction.
  pub fn new<D: Into<String>>(description: D, interaction_type: D) -> Self {
    InteractionBuilder {
      interaction_type: interaction_type.into(),
      description: description.into(),
      provider_states: vec![],
      comments: vec![],
      test_name: None,
      request: RequestBuilder::default(),
      response: ResponseBuilder::default(),
    }
  }

    /// Specify a "provider state" for this interaction. This is normally use to
    /// set up database fixtures when using a pact to test a provider.
    pub fn given<G: Into<String>>(&mut self, given: G) -> &mut Self {
        self.provider_states.push(ProviderState::default(&given.into()));
        self
    }

  /// Adds a text comment to this interaction. This allows to specify just a bit more information
  /// about the interaction. It has no functional impact, but can be displayed in the broker HTML
  /// page, and potentially in the test output.
  pub fn comment<G: Into<String>>(&mut self, comment: G) -> &mut Self {
    self.comments.push(comment.into());
    self
  }

  /// Sets the test name for this interaction. This allows to specify just a bit more information
  /// about the interaction. It has no functional impact, but can be displayed in the broker HTML
  /// page, and potentially in the test output.
  pub fn test_name<G: Into<String>>(&mut self, name: G) -> &mut Self {
    self.test_name = Some(name.into());
    self
  }

  /// The interaction we've built.
  pub fn build(&self) -> RequestResponseInteraction {
    RequestResponseInteraction {
      id: None,
      description: self.description.clone(),
      provider_states: self.provider_states.clone(),
      request: self.request.build(),
      response: self.response.build(),
    }
  }

  /// The interaction we've built (in V4 format).
  pub fn build_v4(&self) -> SynchronousHttp {
    debug!("Building V4 HTTP interaction: {:?}", self);

    SynchronousHttp {
      id: None,
      key: None,
      description: self.description.clone(),
      provider_states: self.provider_states.clone(),
      request: self.request.build_v4(),
      response: self.response.build_v4(),
      comments: hashmap!{
        "text".to_string() => json!(self.comments),
        "testname".to_string() => json!(self.test_name)
      },
      pending: false,
      plugin_config: self.plugin_config(),
      interaction_markup: self.request.interaction_markup().merge(self.response.interaction_markup())
    }
  }

  /// Any plugin configuration returned from plugins
  pub fn plugin_config(&self) -> HashMap<String, HashMap<String, Value>> {
    let mut config = hashmap!{};
    let request_config = self.request.plugin_config();
    if !request_config.is_empty() {
      for (key, value) in request_config {
        config.insert(key.clone(), value.interaction_configuration.clone());
      }
    }
    let response_config = self.response.plugin_config();
    if !response_config.is_empty() {
      for (key, value) in response_config {
        config.insert(key.clone(), value.interaction_configuration.clone());
      }
    }
    config
  }
}
