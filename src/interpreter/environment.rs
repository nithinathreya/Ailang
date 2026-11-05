use super::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Default)]
pub struct Environment {
    values: HashMap<String, Value>,
    enclosing: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            enclosing: None,
        }
    }

    pub fn with_enclosing(enclosing: Rc<RefCell<Environment>>) -> Self {
        Self {
            values: HashMap::new(),
            enclosing: Some(enclosing),
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(val) = self.values.get(name) {
            Some(val.clone())
        } else if let Some(ref enc) = self.enclosing {
            enc.borrow().get(name)
        } else {
            None
        }
    }

    pub fn assign(&mut self, name: &str, value: Value) -> bool {
        if self.values.contains_key(name) {
            self.values.insert(name.to_string(), value);
            true
        } else if let Some(ref enc) = self.enclosing {
            enc.borrow_mut().assign(name, value)
        } else {
            false
        }
    }

    pub fn all_variables(&self) -> HashMap<String, Value> {
        let mut map = if let Some(ref enc) = self.enclosing {
            enc.borrow().all_variables()
        } else {
            HashMap::new()
        };
        for (k, v) in &self.values {
            map.insert(k.clone(), v.clone());
        }
        map
    }
}
