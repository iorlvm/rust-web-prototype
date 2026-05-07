use crate::error::ErrorPayload;
use crate::model::User;
use ioc_lite::Component;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use web_kernel::error::KernelError;

pub enum Authentication {
    Anonymous,
    User(Principal),
}

#[derive(Component)]
pub struct JwtProvider {
    #[script(async |_| DecodingKey::from_rsa_pem(PUBLIC_KEY_PEM.as_bytes()).unwrap())]
    public_key: DecodingKey,

    #[script(async |_| EncodingKey::from_rsa_pem(PRIVATE_KEY_PEM.as_bytes()).unwrap())]
    private_key: EncodingKey,

    #[script(async |_| Header::new(Algorithm::RS256))]
    header: Header,

    #[script(async |_| Validation::new(Algorithm::RS256))]
    validation: Validation,

    #[value = 3600]
    expire_secs: u64,
}

impl JwtProvider {
    pub fn generate_token(&self, user: &User) -> String {
        let principal = Principal::from(user);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let claims = Claims {
            sub: principal.id,
            name: principal.name.clone(),
            email: principal.email.clone(),
            exp: now + self.expire_secs,
        };

        encode(&self.header, &claims, &self.private_key).expect("Failed to generate token")
    }

    pub fn resolve_token(&self, token: &str) -> Result<Authentication, KernelError> {
        if token.is_empty() {
            return Ok(Authentication::Anonymous);
        }

        decode::<Claims>(token, &self.public_key, &self.validation)
            .map(|token_data| Authentication::User(token_data.claims.into_principal()))
            .map_err(|e| {
                KernelError::External(
                    ErrorPayload {
                        __ext_status: 401,
                        error_type: "jwt-decode".to_string(),
                        message: e.to_string(),
                    }
                    .into(),
                )
            })
    }
}

// structs
pub struct Principal {
    id: u64,
    name: String,
    email: String,
}

impl Principal {
    pub fn from(user: &User) -> Self {
        Self {
            id: user.id().expect("User must have id"),
            name: user.name().to_string(),
            email: user.email().to_string(),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn email(&self) -> &str {
        &self.email
    }
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: u64,
    name: String,
    email: String,
    exp: u64,
}
impl Claims {
    fn into_principal(self) -> Principal {
        Principal {
            id: self.sub,
            name: self.name,
            email: self.email,
        }
    }
}

// key pem
const PRIVATE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCk9a08FjnDIZjk
A54YbTgmCZ7PjWXbIEycb994vYFN3Lr5SAuMpjhSH+vFyQ7+VtEJFBudp7POD12f
QwvwWmPZWfhi8bUqDkvbS+Z5EzWdk9nSMQJl/EdL9YkwvdD1N8GDE5iMCqD2hdmg
JBES/m4+sNYP1U1Cmrfaz9I/5Nc/98J0gveUkRYp1O5MX3aD31TwEiNYuL2gb8r7
fl7lGPlwM3rqJTiaa3XB0MuNWthHIiS+tDTvlWtcW5PZRJ9RBq4LvSoOgr6HzbHz
l82o4mrcs2Wuat1w5mrpIvOYyrSiXWgTkM/w31gl3BizzNVI44JCe2nDdE35zyre
CPM8LLEDAgMBAAECggEAAZtRpvkscjBx6x6h5pOsbVUV/T2KVW+4xKP4+usVnFWJ
uK3b9vBo3LE/kaXwsHIrXQ7wv0Cyk/o7AOHH9v0FEJ6QuHNUPrxJlDGRZXKAyNp3
elWh3q5uCgOyj2Kklg7coqiJNCv8/18Jt0aX/VBfGzCLv+G6++rCia4RL9rMbJdl
hk483irnJjDj+i4ogT+mFvGnlhdhUWrByCcGBMFXowpsOakMoGanBGpiAqWrqoS5
7GM1ZCmlDYSbn2HTaH1vzmf0xOEaV1N+bE7acQOd8DyTUhsyXjyL/piipvElf38a
f2yuJelL66/UbYXM5Ck2TKoBb+qYMeWCgCBBJTG+OQKBgQDOwTxbUlbaLUI6uPvo
BxezlnbiHDAYyPbbioc3bQ2wuvP//ruGMXepg0iWRZsi7/n67iEOKFYd6u1srYv7
ooR7LqHQoueEQS/mbZCD12HJYnL1KCCg7WkDfloRvVgiCBlLjlSnJWnTLfv2wIrA
RAdCIwB14nROTx6qJoHLHAMCVwKBgQDMQAEGxiEb3YsDn12qzGRvyKol7wMQzdE4
EQ9qZfxr+t9Bq6ue2HbnwWEi0fhDgHDXsE3+QfPR2yy59P/Xjp70f1uUcJqawWbz
11RjrAxY0EUpiJ9VvUY8DATwdt2+Dv8AGhWyYhLL03sGAM1Yiys0itZ73T6EWWVZ
KNq7h61TNQKBgDLmM3uv653Ooo1eWPWoOVeG5UGI+vY/Eza9BcjJWiN2Ave0uGmy
5+idX1NPU3/oYDw8P3sCxyCaY1Tr7JhITCEfO8rnL+PJQIeT1Y3/ih1P1UkxVccI
a1/mzTmsyXOnVsLZCVIUzdalbXAzunTWyoqnn0dmNofIxp3Q2QctoDgFAoGAbkLD
CxgcOYHAkOUQWKrcMWg/ShkcK48gOccj7kk+GkClEzuMe0TZ88R2HqkA/9evkxBv
GioaaJiMLZrwHjq03sJ9+sVLAO4VGN+Og/wV8kAEhiZl9ZAnATVv2SwiCn7n8/Mg
8Vp2UShKSnRWZk1CtquYm+dU33eu+ZHPetsEm10CgYEAukSDw/fkFdYO4Ea2WkmX
FK1xbNmNuUhKwoDzi4J8wQdUAh3oZDJNqw/OhHDksIEd1kRbnQDUBBFeNvX5YoW6
9wvk7zENMKR3B5pldzajowLPwks6wjd+X7gsXRfmS/d7b7S9jA9qtDX1BJac4vky
zr5Re/hMRO2rkzvulVWgiHo=
-----END PRIVATE KEY-----";

const PUBLIC_KEY_PEM: &str = "-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEApPWtPBY5wyGY5AOeGG04
Jgmez41l2yBMnG/feL2BTdy6+UgLjKY4Uh/rxckO/lbRCRQbnaezzg9dn0ML8Fpj
2Vn4YvG1Kg5L20vmeRM1nZPZ0jECZfxHS/WJML3Q9TfBgxOYjAqg9oXZoCQREv5u
PrDWD9VNQpq32s/SP+TXP/fCdIL3lJEWKdTuTF92g99U8BIjWLi9oG/K+35e5Rj5
cDN66iU4mmt1wdDLjVrYRyIkvrQ075VrXFuT2USfUQauC70qDoK+h82x85fNqOJq
3LNlrmrdcOZq6SLzmMq0ol1oE5DP8N9YJdwYs8zVSOOCQntpw3RN+c8q3gjzPCyx
AwIDAQAB
-----END PUBLIC KEY-----";
