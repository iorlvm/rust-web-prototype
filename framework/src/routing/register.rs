use crate::routing::route::Route;
use http::Method;
use std::collections::HashMap;

pub struct RouteRegister {
    map: HashMap<Method, Vec<Route>>,
}

impl RouteRegister {
    pub fn get_routes(&self, method: &Method) -> Option<&Vec<Route>> {
        self.map.get(method)
    }
}

pub struct RouteRegisterBuilder {
    routes: Vec<Route>,
}

impl RouteRegisterBuilder {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn register(mut self, route: Route) -> Self {
        self.routes.push(route);
        self
    }

    pub fn build(self) -> RouteRegister {
        let mut map: HashMap<Method, Vec<Route>> = HashMap::new();

        for route in self.routes {
            map.entry(route.method().clone()).or_default().push(route);
        }

        for routes in map.values_mut() {
            routes.sort_by_key(|route| route.path().len());
        }

        RouteRegister { map }
    }
}
