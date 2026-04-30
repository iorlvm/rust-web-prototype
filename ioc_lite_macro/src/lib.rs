mod attribute;
mod expand;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// # Component Derive Macro
///
/// 提供 `#[derive(Component)]` 自動生成 IoC 元件註冊與建構邏輯。
///
/// ## 功能
/// 1. 自動實作 `ioc_lite::Component` trait
/// 2. 自動生成 `create(ioc_lite)` 建構函式
/// 3. 支援欄位注入：
///    - `#[component]`：從 IoC 取得依賴（必須為 Arc<T>）
///    - `#[value = "..."]`：常數注入
///    - `#[script(async fn(ioc: &mut IoC) -> T)`：腳本注入
///    - 無標註：使用 Default
/// 4. 自動註冊至 `inventory`
///
/// ## 限制
/// - 僅支援 struct
/// - 不支援 generic struct
/// - 不支援 tuple / unnamed struct
/// - component 欄位必須為 `Arc<T>`
///
/// ## 使用範例
/// ```rust
/// #[derive(Component)]
///
/// async fn init_depend(ioc: &mut IoC) -> Vec<Depend> { ... }
///
/// struct Foo {
///     #[component]
///     service: Arc<Service>,
///
///     #[value = "hello"]
///     name: String,
///
///     #[script(init_depend)]
///     depend: Vec<Depend>,
///
///     cache: Cache, // Default::default()
/// }
/// ```
#[proc_macro_derive(Component, attributes(component, value, script))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand::expand_component(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
