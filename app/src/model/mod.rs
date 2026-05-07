#[derive(Clone)]
pub struct User {
    id: Option<u64>,
    email: String,
    name: String,
    password: String,
}

impl User {
    pub fn new(
        id: Option<u64>,
        email: String,
        name: String,
        password: String,
    ) -> Result<Self, String> {
        let mut user = Self {
            id,
            email: "".to_string(),
            name: "".to_string(),
            password: "".to_string(),
        };

        user.set_email(email)?;
        user.set_name(name)?;
        user.set_password(password)?;

        Ok(user)
    }

    pub fn set_id(&mut self, id: u64) {
        self.id = Some(id);
    }

    pub fn set_email(&mut self, email: String) -> Result<(), String> {
        if email.len() < 4 || email.len() > 32 {
            return Err("Email must be between 4 and 32 characters".to_string());
        }
        self.email = email;
        Ok(())
    }

    pub fn set_name(&mut self, name: String) -> Result<(), String> {
        if name.len() < 5 || name.len() > 32 {
            return Err("Name must be between 5 and 32 characters".to_string());
        }
        self.name = name;
        Ok(())
    }

    pub fn set_password(&mut self, password: String) -> Result<(), String> {
        if password.len() < 5 || password.len() > 32 {
            return Err("Password must be between 5 and 32 characters".to_string());
        }
        self.password = password;
        Ok(())
    }

    pub fn id(&self) -> Option<u64> {
        self.id
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}
