#[macro_use]
mod macros;
use std::fmt::Debug;
use std::{any::Any, sync::Arc};

mod rational;
pub use rational::*;
mod string;
pub use string::*;
mod unit;
pub use unit::*;
mod i64;
pub use self::i64::*;
mod f64;
pub use self::f64::*;
mod map;
pub use map::*;
mod set;
pub use set::*;
mod vec;
pub use vec::*;

use crate::extract::{Cost, Extractor};
use crate::*;

pub trait Sort: Any + Send + Sync + Debug {
    fn name(&self) -> Symbol;
    fn as_arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync + 'static>;
    fn is_eq_sort(&self) -> bool {
        false
    }

    // return true if it is a container sort.
    fn is_container_sort(&self) -> bool {
        false
    }

    // return true if it is a container sort that contains ids.
    // only eq_sort and eq_container_sort need to be canonicalized.
    fn is_eq_container_sort(&self) -> bool {
        false
    }

    // Only eq_container_sort need to implement this method,
    // which returns a list of ids to be tracked.
    fn foreach_tracked_values<'a>(&'a self, value: &'a Value, mut f: Box<dyn FnMut(Value) + 'a>) {
        for (sort, value) in self.inner_values(value) {
            if sort.is_eq_sort() {
                f(value)
            }
        }
    }

    // Sort-wise canonicalization. Return true if value is modified.
    // Only EqSort or containers of EqSort should override.
    fn canonicalize(&self, value: &mut Value, unionfind: &UnionFind) -> bool {
        debug_assert_eq!(self.name(), value.tag);
        let _ = unionfind;
        false
    }

    /// Return the inner values and sorts.
    /// Only eq_container_sort need to implement this method,
    fn inner_values(&self, value: &Value) -> Vec<(&ArcSort, Value)> {
        let _ = value;
        vec![]
    }

    fn register_primitives(self: Arc<Self>, info: &mut TypeInfo) {
        let _ = info;
    }

    /// Extracting an expression (with smallest cost) out of a primitive value
    fn make_expr(&self, egraph: &EGraph, value: Value) -> (Cost, Expr);

    /// For values like EqSort containers, to make/extract an expression from it
    /// requires an extractor. Moreover, the extraction may be unsuccessful if
    /// the extractor is not fully initialized.
    ///
    /// The default behavior is to call make_expr
    fn extract_expr(
        &self,
        egraph: &EGraph,
        value: Value,
        _extractor: &Extractor,
        _termdag: &mut TermDag,
    ) -> Option<(Cost, Expr)> {
        Some(self.make_expr(egraph, value))
    }
}

#[derive(Debug)]
pub struct EqSort {
    pub name: Symbol,
}

impl Sort for EqSort {
    fn name(&self) -> Symbol {
        self.name
    }

    fn as_arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync + 'static> {
        self
    }

    fn is_eq_sort(&self) -> bool {
        true
    }

    fn canonicalize(&self, value: &mut Value, unionfind: &UnionFind) -> bool {
        debug_assert_eq!(self.name(), value.tag);
        let bits = usize::from(unionfind.find(Id::from(value.bits as usize))) as u64;
        if bits != value.bits {
            value.bits = bits;
            true
        } else {
            false
        }
    }

    fn make_expr(&self, _egraph: &EGraph, _value: Value) -> (Cost, Expr) {
        unimplemented!("No make_expr for EqSort {}", self.name)
    }
}

pub trait FromSort: Sized {
    type Sort: Sort;
    fn load(sort: &Self::Sort, value: &Value) -> Self;
}

pub trait IntoSort: Sized {
    type Sort: Sort;
    fn store(self, sort: &Self::Sort) -> Option<Value>;
}

impl<T: IntoSort> IntoSort for Option<T> {
    type Sort = T::Sort;

    fn store(self, sort: &Self::Sort) -> Option<Value> {
        self?.store(sort)
    }
}

pub type PreSort =
    fn(typeinfo: &mut TypeInfo, name: Symbol, params: &[Expr]) -> Result<ArcSort, TypeError>;
