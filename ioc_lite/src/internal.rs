use std::any::TypeId;
use std::sync::Arc;
use dashmap::DashMap;

pub trait TestTrace {
    fn test_trace(&self) -> &Option<Arc<DashMap<TypeId, ()>>>;

    fn in_test(&self) -> bool {
        self.test_trace().is_some()
    }

    fn check_circular_dependency(&self, type_id: &TypeId) -> () {
        if let Some(test_trace) = &self.test_trace() {
            if test_trace.contains_key(&type_id) {
                panic!("circular dependency detected");
            }
        }
    }

    fn enter_trace(&self, type_id: TypeId) {
        if let Some(test_trace) = &self.test_trace() {
            test_trace.insert(type_id, ());
        }
    }

    fn exit_trace(&self, type_id: &TypeId) {
        if let Some(test_trace) = &self.test_trace() {
            test_trace.remove(type_id);
        }
    }
}

pub trait IoCBuilderScript {
    fn into_test_mode(self) -> Self;

    async fn on_build(&self);

    async fn run_test(&self);
}