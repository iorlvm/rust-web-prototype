use crate::model::User;
use ioc_lite::Component;
use std::collections::HashMap;

// Memory-mocked user repository
#[derive(Component)]
pub struct UserRepository {
    #[value = 1]
    next_id: u64,
    map: HashMap<u64, User>,

    mail_index: HashMap<String, u64>,
    name_index: HashMap<String, u64>,
}

impl UserRepository {
    pub async fn query_by_name_like(&self, search_name: &str) -> Vec<User> {
        let mut result = vec![];
        for (name, user_id) in &self.name_index {
            if name.contains(search_name) {
                result.push(self.map.get(user_id).unwrap().clone());
            }
        }

        result.sort_by(|a, b| a.id().unwrap().cmp(&b.id().unwrap()));
        result
    }

    pub async fn find_by_email_and_password(&self, email: &str, password: &str) -> Option<User> {
        self.mail_index
            .get(email)
            .and_then(|id| self.map.get(id))
            .filter(|user| user.password() == password)
            .cloned()
    }

    pub async fn find_by_id(&self, user_id: u64) -> Option<User> {
        self.map.get(&user_id).cloned()
    }

    pub async fn save(&mut self, user: User) -> Result<User, String> {
        if user.id().is_none() {
            self.insert(user)
        } else {
            self.update(user)
        }
    }

    fn insert(&mut self, user: User) -> Result<User, String> {
        if self.mail_index.contains_key(user.email()) {
            return Err("Duplicate email".to_string());
        }

        let mut user = user;
        user.set_id(self.next_id);
        self.next_id += 1;

        let user_id = user.id().unwrap();
        self.map.insert(user_id, user.clone());
        self.mail_index.insert(user.email().to_string(), user_id);
        self.name_index.insert(user.name().to_string(), user_id);

        Ok(user)
    }

    fn update(&mut self, user: User) -> Result<User, String> {
        let user_id: u64 = user.id().unwrap();
        let original = self.map.get(&user_id).ok_or("User not found".to_string())?;

        if let Some(id) = self.mail_index.get(user.email()) {
            // 改變了 email, 但新的 email 與其他人相同
            if id != &user_id {
                return Err("Duplicate email".to_string());
            }
        }

        self.mail_index.remove(original.email());
        self.name_index.remove(original.name());
        self.map.remove(&user_id);

        self.map.insert(user_id, user.clone());
        self.mail_index.insert(user.email().to_string(), user_id);
        self.name_index.insert(user.name().to_string(), user_id);

        Ok(user)
    }
}
