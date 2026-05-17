use dashmap::DashSet;
use std::any::TypeId;

pub trait TestTrace {
    fn test_trace(&self) -> &Option<DashSet<TypeId>>;

    fn in_test(&self) -> bool {
        self.test_trace().is_some()
    }

    fn check_circular_dependency(&self, type_id: &TypeId) -> () {
        if let Some(test_trace) = &self.test_trace() {
            if test_trace.contains(&type_id) {
                panic!("circular dependency detected");
            }
        }
    }

    fn enter_trace(&self, type_id: &TypeId) {
        if let Some(test_trace) = &self.test_trace() {
            test_trace.insert(type_id.clone());
        }
    }

    fn exit_trace(&self, type_id: &TypeId) {
        if let Some(test_trace) = &self.test_trace() {
            test_trace.remove(type_id);
        }
    }
}

pub trait IoCBuilderScript {
    async fn on_build(&self);

    async fn run_test(&self);
}
