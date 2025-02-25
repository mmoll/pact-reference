//! `matchingrules` module includes all the classes to deal with V3 format matchers

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::{Debug, Display};
use std::str::from_utf8;

use anyhow::anyhow;
use onig::Regex;
use pact_models::matchingrules::{MatchingRule, MatchingRuleCategory, RuleList, RuleLogic};
use pact_models::path_exp::DocPath;
use serde_json::{self, json, Value};
use tracing::debug;

use crate::{Either, MatchingContext, merge_result, Mismatch};
use crate::binary_utils::match_content_type;
use crate::matchers::{match_values, Matches};

impl <T: Debug + Display + PartialEq + Clone> Matches<&Vec<T>> for &Vec<T> {
  fn matches_with(&self, actual: &Vec<T>, matcher: &MatchingRule, cascaded: bool) -> anyhow::Result<()> {
    self.as_slice().matches_with(actual.as_slice(), matcher, cascaded)
  }
}

impl <T: Debug + Display + PartialEq + Clone> Matches<&[T]> for &[T] {
  fn matches_with(&self, actual: &[T], matcher: &MatchingRule, cascaded: bool) -> anyhow::Result<()> {
    debug!("slice -> slice: comparing [{}] to [{}] using {:?}", std::any::type_name::<T>(), std::any::type_name::<T>(), matcher);
    let result = match matcher {
      MatchingRule::Regex(ref regex) => {
        match Regex::new(regex) {
          Ok(re) => {
            let text: String = actual.iter().map(|v| v.to_string()).collect();
            if re.is_match(text.as_str()) {
              Ok(())
            } else {
              Err(anyhow!("Expected '{}' to match '{}'", text, regex))
            }
          }
          Err(err) => Err(anyhow!("'{}' is not a valid regular expression - {}", regex, err))
        }
      }
      MatchingRule::Type => Ok(()),
      MatchingRule::MinType(min) => {
        if !cascaded && actual.len() < *min {
          Err(anyhow!("Expected list with length {} to have a minimum length of {}", actual.len(), min))
        } else {
          Ok(())
        }
      }
      MatchingRule::MaxType(max) => {
        if !cascaded && actual.len() > *max {
          Err(anyhow!("Expected list with length {} to have a maximum length of {}", actual.len(), max))
        } else {
          Ok(())
        }
      }
      MatchingRule::MinMaxType(min, max) => {
        if !cascaded && actual.len() < *min {
          Err(anyhow!("Expected list with length {} to have a minimum length of {}", actual.len(), min))
        } else if !cascaded && actual.len() > *max {
          Err(anyhow!("Expected list with length {} to have a maximum length of {}", actual.len(), max))
        } else {
          Ok(())
        }
      }
      MatchingRule::Equality => {
        if *self == actual {
          Ok(())
        } else {
          Err(anyhow!("Expected {} to be equal to {}", actual.for_mismatch(), self.for_mismatch()))
        }
      }
      MatchingRule::NotEmpty => {
        if actual.is_empty() {
          Err(anyhow!("Expected an non-empty list"))
        } else {
          Ok(())
        }
      }
      MatchingRule::ArrayContains(_) => Ok(()),
      MatchingRule::EachKey(_) => Ok(()),
      MatchingRule::EachValue(_) => Ok(()),
      MatchingRule::Values => Ok(()),
      _ => Err(anyhow!("Unable to match {} using {:?}", self.for_mismatch(), matcher))
    };
    debug!("Comparing '{:?}' to '{:?}' using {:?} -> {:?}", self, actual, matcher, result);
    result
  }
}

impl Matches<Vec<u8>> for Vec<u8> {
  fn matches_with(&self, actual: Vec<u8>, matcher: &MatchingRule, cascaded: bool) -> anyhow::Result<()> {
    self.matches_with(actual.as_slice(), matcher, cascaded)
  }
}

impl Matches<&Vec<u8>> for Vec<u8> {
  fn matches_with(&self, actual: &Vec<u8>, matcher: &MatchingRule, cascaded: bool) -> anyhow::Result<()> {
    self.matches_with(actual.as_slice(), matcher, cascaded)
  }
}

impl Matches<&[u8]> for Vec<u8> {
  fn matches_with(&self, actual: &[u8], matcher: &MatchingRule, cascaded: bool) -> anyhow::Result<()> {
    debug!("byte slice -> byte slice: comparing {:?} to {:?} using {:?}", self, actual, matcher);
    let result = match matcher {
      MatchingRule::Regex(regex) => {
        match Regex::new(regex) {
          Ok(re) => {
            let text = from_utf8(actual).unwrap_or_default();
            if re.is_match(text) {
              Ok(())
            } else {
              Err(anyhow!("Expected '{}' to match '{}'", text, regex))
            }
          }
          Err(err) => Err(anyhow!("'{}' is not a valid regular expression - {}", regex, err))
        }
      }
      MatchingRule::Type => Ok(()),
      MatchingRule::MinType(min) => {
        if !cascaded && actual.len() < *min {
          Err(anyhow!("Expected list with length {} to have a minimum length of {}", actual.len(), min))
        } else {
          Ok(())
        }
      }
      MatchingRule::MaxType(max) => {
        if !cascaded && actual.len() > *max {
          Err(anyhow!("Expected list with length {} to have a maximum length of {}", actual.len(), max))
        } else {
          Ok(())
        }
      }
      MatchingRule::MinMaxType(min, max) => {
        if !cascaded && actual.len() < *min {
          Err(anyhow!("Expected list with length {} to have a minimum length of {}", actual.len(), min))
        } else if !cascaded && actual.len() > *max {
          Err(anyhow!("Expected list with length {} to have a maximum length of {}", actual.len(), max))
        } else {
          Ok(())
        }
      }
      MatchingRule::Equality => {
        if *self == actual {
          Ok(())
        } else {
          Err(anyhow!("Expected {:?} to be equal to {:?}", actual, self))
        }
      }
      MatchingRule::ContentType(ref expected_content_type) => {
        match_content_type(actual, expected_content_type)
          .map_err(|err| anyhow!("Expected data to have a content type of '{}' but was {}", expected_content_type, err))
      }
      MatchingRule::NotEmpty => {
        if actual.is_empty() {
          Err(anyhow!("Expected an non-empty list"))
        } else {
          Ok(())
        }
      }
      _ => Err(anyhow!("Unable to match {:?} using {:?}", self, matcher))
    };
    debug!("Comparing list with {} items to one with {} items using {:?} -> {:?}", self.len(), actual.len(), matcher, result);
    result
  }
}

impl Matches<&[u8]> for &Vec<u8> {
  fn matches_with(&self, actual: &[u8], matcher: &MatchingRule, cascaded: bool) -> anyhow::Result<()> {
    (*self).matches_with(actual, matcher, cascaded)
  }
}

/// Trait to convert a expected or actual complex object into a string that can be used for a mismatch
pub trait DisplayForMismatch {
  /// Return a string representation that can be used in a mismatch to display to the user
  fn for_mismatch(&self) -> String;
}

impl <T: Display> DisplayForMismatch for HashMap<String, T> {
  fn for_mismatch(&self) -> String {
    Value::Object(self.iter().map(|(k, v)| (k.clone(), json!(v.to_string()))).collect()).to_string()
  }
}

impl <T: Display> DisplayForMismatch for Vec<T> {
  fn for_mismatch(&self) -> String {
    Value::Array(self.iter().map(|v| json!(v.to_string())).collect()).to_string()
  }
}

impl <T: Display> DisplayForMismatch for &[T] {
  fn for_mismatch(&self) -> String {
    Value::Array(self.iter().map(|v| json!(v.to_string())).collect()).to_string()
  }
}

impl <T: Display> DisplayForMismatch for HashSet<T> {
  fn for_mismatch(&self) -> String {
    let mut values = self.iter().map(|v| v.to_string()).collect::<Vec<String>>();
    values.sort();
    values.for_mismatch()
  }
}

impl <T: Display> DisplayForMismatch for BTreeSet<T> {
  fn for_mismatch(&self) -> String {
    let mut values = self.iter().map(|v| v.to_string()).collect::<Vec<String>>();
    values.sort();
    values.for_mismatch()
  }
}

/// Delegate to the matching rule defined at the given path to compare the key/value maps.
pub fn compare_maps_with_matchingrule<T: Display + Debug>(
  rule: &MatchingRule,
  cascaded: bool,
  path: &DocPath,
  expected: &BTreeMap<String, T>,
  actual: &BTreeMap<String, T>,
  context: &dyn MatchingContext,
  callback: &mut dyn FnMut(&DocPath, &T, &T) -> Result<(), Vec<Mismatch>>
) -> Result<(), Vec<Mismatch>> {
  let mut result = Ok(());
  if !cascaded && rule.is_values_matcher() {
    debug!("Values matcher is defined for path {}", path);
    for (key, value) in actual.iter() {
      let p = path.join(key);
      if expected.contains_key(key) {
        result = merge_result(result, callback(&p, &expected[key], value));
      } else if let Some(first) = expected.values().next() {
        result = merge_result(result, callback(&p, first, value));
      }
    }
  } else {
    let expected_keys = expected.keys().cloned().collect();
    let actual_keys = actual.keys().cloned().collect();
    result = merge_result(result, context.match_keys(path, &expected_keys, &actual_keys));
    for (key, value) in expected.iter() {
      if actual.contains_key(key) {
        let p = path.join(key);
        result = merge_result(result, callback(&p, value, &actual[key]));
      }
    }
  }
  result
}

/// Compare the expected and actual lists using the matching rule's logic
pub fn compare_lists_with_matchingrule<T: Display + Debug + PartialEq + Clone + Sized>(
  rule: &MatchingRule,
  path: &DocPath,
  expected: &[T],
  actual: &[T],
  context: &dyn MatchingContext,
  cascaded: bool,
  callback: &mut dyn FnMut(&DocPath, &T, &T, &dyn MatchingContext) -> Result<(), Vec<Mismatch>>
) -> Result<(), Vec<Mismatch>> {
  let mut result = vec![];

  if !expected.is_empty() {
    match rule {
      // TODO: need to implement the ignore order matchers (See Pact-JVM core/matchers/src/main/kotlin/au/com/dius/pact/core/matchers/Matchers.kt:133)
      // is EqualsIgnoreOrderMatcher,
      //         is MinEqualsIgnoreOrderMatcher,
      //         is MaxEqualsIgnoreOrderMatcher,
      //         is MinMaxEqualsIgnoreOrderMatcher -> {
      MatchingRule::ArrayContains(variants) => {
        debug!("Matching {} with ArrayContains", path);
        let variants = if variants.is_empty() {
          expected.iter().enumerate().map(|(index, _)| {
            (index, MatchingRuleCategory::equality("body"), HashMap::default())
          }).collect()
        } else {
          variants.clone()
        };
        for (index, rules, _) in variants {
          match expected.get(index) {
            Some(expected_value) => {
              let context = context.clone_with(&rules);
              if actual.iter().enumerate().find(|&(actual_index, value)| {
                debug!("Comparing list item {} with value '{:?}' to '{:?}'", actual_index, value, expected_value);
                callback(&DocPath::root(), expected_value, value, context.as_ref()).is_ok()
              }).is_none() {
                result.push(Mismatch::BodyMismatch {
                  path: path.to_string(),
                  expected: Some(expected_value.to_string().into()),
                  actual: Some(actual.for_mismatch().into()),
                  mismatch: format!("Variant at index {} ({}) was not found in the actual list", index, expected_value)
                });
              };
            },
            None => {
              result.push(Mismatch::BodyMismatch {
                path: path.to_string(),
                expected: Some(expected.for_mismatch().into()),
                actual: Some(actual.for_mismatch().into()),
                mismatch: format!("ArrayContains: variant {} is missing from the expected list, which has {} items",
                                  index, expected.len())
              });
            }
          }
        }
      }
      MatchingRule::EachValue(definition) => if !cascaded {
        debug!("Matching {} with EachValue", path);
        let associated_rules = definition.rules.iter().filter_map(|rule| {
          match rule {
            Either::Left(rule) => Some(rule.clone()),
            Either::Right(reference) => {
              result.push(Mismatch::BodyMismatch {
                path: path.to_string(),
                expected: Some(expected.for_mismatch().into()),
                actual: Some(actual.for_mismatch().into()),
                mismatch: format!("Found an un-resolved reference {}", reference.name)
              });
              None
            }
          }
        }).collect();
        if let Err(mismatches) = match_values(path, &RuleList {
          rules: associated_rules,
          rule_logic: RuleLogic::And,
          cascaded
        }, expected, actual) {
          for mismatch in mismatches {
            result.push(Mismatch::BodyMismatch {
              path: path.to_string(),
              expected: Some(expected.for_mismatch().into()),
              actual: Some(actual.for_mismatch().into()),
              mismatch: mismatch.to_string()
            });
          }
        }
      }
      _ => {
        if let Err(mismatch) = expected.matches_with(actual, rule, cascaded) {
          result.push(Mismatch::BodyMismatch {
            path: path.to_string(),
            expected: Some(expected.for_mismatch().into()),
            actual: Some(actual.for_mismatch().into()),
            mismatch: mismatch.to_string()
          });
        }

        result.extend(match_list_contents(path, expected, actual, context, callback));
      }
    }
  }

  if result.is_empty() {
    Ok(())
  } else {
    Err(result)
  }
}

fn match_list_contents<T: Display + Debug + PartialEq + Clone + Sized>(
  path: &DocPath,
  expected: &[T],
  actual: &[T],
  context: &dyn MatchingContext,
  callback: &mut dyn FnMut(&DocPath, &T, &T, &dyn MatchingContext) -> Result<(), Vec<Mismatch>>
) -> Vec<Mismatch> {
  let mut result = vec![];

  let mut expected_list = expected.to_vec();
  if actual.len() > expected.len() {
    if let Some(first) = expected.first() {
      expected_list.resize(actual.len(), first.clone());
    }
  }

  for (index, value) in expected_list.iter().enumerate() {
    let ps = index.to_string();
    debug!("Comparing list item {} with value '{:?}' to '{:?}'", index, actual.get(index), value);
    let p = path.join(ps);
    if index < actual.len() {
      if let Err(mismatches) = callback(&p, value, &actual[index], context) {
        result.extend(mismatches);
      }
    } else if !context.matcher_is_defined(&p) {
      result.push(Mismatch::BodyMismatch {
        path: path.to_string(),
        expected: Some(expected.for_mismatch().into()),
        actual: Some(actual.for_mismatch().into()),
        mismatch: format!("Expected {} ({}) but was missing", value, index)
      });
    }
  }

  result
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;
  use std::collections::{BTreeSet, HashMap, HashSet};
  use std::rc::Rc;

  use expectest::prelude::*;
  use maplit::btreemap;
  use pact_models::matchingrules::{MatchingRule, MatchingRuleCategory, RuleList};
  use pact_models::matchingrules::expressions::{MatchingRuleDefinition, ValueType};
  use pact_models::path_exp::DocPath;
  use pact_plugin_driver::plugin_models::PluginInteractionConfig;

  use crate::{DiffConfig, MatchingContext, Mismatch};
  use crate::matchingrules::{compare_lists_with_matchingrule, compare_maps_with_matchingrule};

  struct MockContext {
    pub calls: Rc<RefCell<Vec<String>>>
  }

  impl MatchingContext for MockContext {
    fn matcher_is_defined(&self, path: &DocPath) -> bool {
      self.calls.borrow_mut().push(format!("matcher_is_defined({})", path));
      true
    }

    fn select_best_matcher(&self, _path: &DocPath) -> RuleList {
      todo!()
    }

    fn type_matcher_defined(&self, _path: &DocPath) -> bool {
      todo!()
    }

    fn values_matcher_defined(&self, _path: &DocPath) -> bool {
      todo!()
    }

    fn direct_matcher_defined(&self, _path: &DocPath, _matchers: &HashSet<&str>) -> bool {
      todo!()
    }

    fn match_keys(&self, path: &DocPath, expected: &BTreeSet<String>, actual: &BTreeSet<String>) -> Result<(), Vec<Mismatch>> {
      self.calls.borrow_mut().push(format!("match_keys({}, {:?}, {:?})", path, expected, actual));
      Ok(())
    }

    fn plugin_configuration(&self) -> &HashMap<String, PluginInteractionConfig> {
      todo!()
    }

    fn matchers(&self) -> &MatchingRuleCategory {
      todo!()
    }

    fn config(&self) -> DiffConfig {
      todo!()
    }

    fn clone_with(&self, _matchers: &MatchingRuleCategory) -> Box<dyn MatchingContext> {
      Box::new(MockContext {
        calls: self.calls.clone()
      })
    }
  }

  #[test]
  fn compare_maps_with_matchingrule_with_no_value_matcher_at_path() {
    let rule = MatchingRule::Type;
    let expected = btreemap!{
      "a".to_string() => "100".to_string(),
      "b".to_string() => "101".to_string()
    };
    let actual = btreemap!{
      "a".to_string() => "101".to_string()
    };

    let context = MockContext {
      calls: Rc::new(RefCell::new(vec![]))
    };
    let mut calls = vec![];
    let mut callback = |p: &DocPath, a: &String, b: &String| {
      calls.push(format!("{}, {}, {}", p, a, b));
      Ok(())
    };
    let result = compare_maps_with_matchingrule(&rule, false, &DocPath::root(),
      &expected, &actual, &context, &mut callback);

    expect!(result).to(be_ok());

    // We expect match keys to be called, then the callback of each key that is also in the
    // actual map
    let v = vec![
      "match_keys($, {\"a\", \"b\"}, {\"a\"})".to_string()
    ];
    expect!(context.calls.borrow().clone()).to(be_equal_to(v));
    let v = vec![
      "$.a, 100, 101".to_string()
    ];
    expect!(calls).to(be_equal_to(v));
  }

  #[test]
  fn compare_maps_with_matchingrule_with_value_matcher_at_path() {
    let expected = btreemap!{
      "a".to_string() => "100".to_string()
    };
    let actual = btreemap!{
      "a".to_string() => "101".to_string(),
      "b".to_string() => "102".to_string()
    };

    let context = MockContext {
      calls: Rc::new(RefCell::new(vec![]))
    };
    let mut calls = vec![];
    let mut callback = |p: &DocPath, a: &String, b: &String| {
      calls.push(format!("{}, {}, {}", p, a, b));
      Ok(())
    };
    let result = compare_maps_with_matchingrule(&MatchingRule::Values, false, &DocPath::root(),
      &expected, &actual, &context, &mut callback);
    let rule = MatchingRule::EachValue(MatchingRuleDefinition {
      value: "".to_string(),
      value_type: ValueType::Unknown,
      rules: vec![],
      generator: None
    });
    let result2 = compare_maps_with_matchingrule(&rule, false, &DocPath::root(),
      &expected, &actual, &context, &mut callback);

    expect!(result).to(be_ok());
    expect!(result2).to(be_ok());

    // With a values matcher, we expect the callback to be called for each key in the actual map
    // and no other methods called on the context
    let v: Vec<String> = vec![];
    expect!(context.calls.borrow().clone()).to(be_equal_to(v));
    let v = vec![
      "$.a, 100, 101".to_string(),
      "$.b, 100, 102".to_string(),
      "$.a, 100, 101".to_string(),
      "$.b, 100, 102".to_string()
    ];
    expect!(calls).to(be_equal_to(v));
  }

  #[test]
  fn compare_lists_with_matchingrule_with_empty_expected_list() {
    let expected = vec![  ];
    let actual = vec![ "one".to_string(), "two".to_string() ];

    let context = MockContext {
      calls: Rc::new(RefCell::new(vec![]))
    };
    let mut calls = vec![];
    let mut callback = |p: &DocPath, a: &String, b: &String, _context: &dyn MatchingContext| {
      calls.push(format!("{}, {}, {}", p, a, b));
      Ok(())
    };

    let result = compare_lists_with_matchingrule(&MatchingRule::Type,
                                                 &DocPath::root(), &expected, &actual, &context, false, &mut callback);

    expect!(result).to(be_ok());

    let v: Vec<String> = vec![];
    expect!(context.calls.borrow().clone()).to(be_equal_to(v.clone()));
    expect!(calls).to(be_equal_to(v));
  }

  #[test]
  fn compare_lists_with_matchingrule_with_simple_matcher() {
    let expected = vec![ "value one".to_string(), "value two".to_string(), "value three".to_string() ];
    let actual = vec![ "one".to_string(), "two".to_string() ];

    let context = MockContext {
      calls: Rc::new(RefCell::new(vec![]))
    };
    let mut calls = vec![];
    let mut callback = |p: &DocPath, a: &String, b: &String, _context: &dyn MatchingContext| {
      calls.push(format!("{}, {}, {}", p, a, b));
      Ok(())
    };

    let result = compare_lists_with_matchingrule(&MatchingRule::Type,
      &DocPath::root(), &expected, &actual, &context, false, &mut callback);

    expect!(result).to(be_ok());

    let v: Vec<String> = vec![
      "matcher_is_defined($[2])".to_string()
    ];
    expect!(context.calls.borrow().clone()).to(be_equal_to(v));

    let v: Vec<String> = vec![
      "$[0], value one, one".to_string(),
      "$[1], value two, two".to_string()
    ];
    expect!(calls).to(be_equal_to(v));
  }

  #[test]
  fn compare_lists_with_matchingrule_with_each_key_matcher() {
    let expected = vec![ "value one".to_string(), "value two".to_string(), "value three".to_string() ];
    let actual = vec![ "one".to_string(), "two".to_string() ];

    let context = MockContext {
      calls: Rc::new(RefCell::new(vec![]))
    };
    let mut calls = vec![];
    let mut callback = |p: &DocPath, a: &String, b: &String, _context: &dyn MatchingContext| {
      calls.push(format!("{}, {}, {}", p, a, b));
      Ok(())
    };

    let rule = MatchingRule::EachKey(MatchingRuleDefinition {
      value: "".to_string(),
      value_type: ValueType::Unknown,
      rules: vec![],
      generator: None
    });
    let result = compare_lists_with_matchingrule(&rule, &DocPath::root(),
      &expected, &actual, &context, false, &mut callback);

    expect!(result).to(be_ok());

    let v: Vec<String> = vec![
      "matcher_is_defined($[2])".to_string()
    ];
    expect!(context.calls.borrow().clone()).to(be_equal_to(v));

    let v: Vec<String> = vec![
      "$[0], value one, one".to_string(),
      "$[1], value two, two".to_string()
    ];
    expect!(calls).to(be_equal_to(v));
  }
}
