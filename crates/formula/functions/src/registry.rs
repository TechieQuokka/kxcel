use std::collections::HashMap;

use crate::{Function, logic, math};

/// The functions a formula can call.
#[derive(Default)]
pub struct FunctionRegistry {
    functions: HashMap<&'static str, Box<dyn Function + Send + Sync>>,
}

impl FunctionRegistry {
    pub fn with_builtins() -> Self {
        let mut registry = Self::default();
        registry.register(math::sum::Sum);
        registry.register(math::average::Average);
        registry.register(math::count::Count);
        registry.register(math::min::Min);
        registry.register(math::max::Max);
        registry.register(logic::if_fn::If);
        registry
    }

    pub fn register(&mut self, function: impl Function + Send + Sync + 'static) {
        self.functions.insert(function.name(), Box::new(function));
    }

    /// Every function's name and help, sorted by name.
    pub fn list(&self) -> Vec<(&'static str, &'static str)> {
        let mut list: Vec<_> = self.functions.values().map(|function| (function.name(), function.help())).collect();
        list.sort_unstable();
        list
    }

    /// Finds a function by its uppercase name.
    pub fn get(&self, name: &str) -> Option<&(dyn Function + Send + Sync)> {
        self.functions.get(name).map(|function| &**function)
    }
}
